use std::path::{Path, PathBuf};

use async_process::{Command, Stdio};
use async_std::sync::Arc;
use async_trait::async_trait;
use node_semver::{Range, Version};
use once_cell::sync::OnceCell;
use oro_client::{self, OroClient};
use oro_common::{CorgiPackument, CorgiVersionMetadata, Packument, VersionMetadata};
use oro_package_spec::{GitInfo, PackageSpec};
use url::Url;

use crate::error::{NassunError, Result};
use crate::fetch::dir::DirFetcher;
use crate::fetch::PackageFetcher;
use crate::package::Package;
use crate::resolver::PackageResolution;
use crate::tarball::Tarball;

#[derive(Debug)]
pub(crate) struct GitFetcher {
    client: OroClient,
    dir_fetcher: DirFetcher,
    git: OnceCell<PathBuf>,
}

impl GitFetcher {
    pub(crate) fn new(client: OroClient) -> Self {
        Self {
            client,
            dir_fetcher: DirFetcher::new(),
            git: OnceCell::new(),
        }
    }

    async fn fetch_to_temp_dir(&self, info: &GitInfo, dir: &Path) -> Result<()> {
        match info {
            GitInfo::Url {
                url,
                committish,
                semver,
                ..
            } => {
                self.fetch_clone(dir, url.to_string(), committish, semver, info)
                    .await?;
            }
            GitInfo::Ssh {
                ssh,
                committish,
                semver,
                ..
            } => {
                self.fetch_clone(dir, ssh, committish, semver, info).await?;
            }
            hosted @ GitInfo::Hosted { .. } => match &hosted {
                GitInfo::Hosted {
                    requested,
                    committish,
                    semver,
                    ..
                } => {
                    if let Some(requested) = requested {
                        self.fetch_clone(dir, requested, committish, semver, info)
                            .await?;
                    } else if let (Some(tarball), Some(https), Some(ssh)) =
                        (hosted.tarball(), hosted.https(), hosted.ssh())
                    {
                        match self.fetch_tarball(dir, &tarball).await {
                            Ok(_) => {}
                            Err(_) => {
                                match self
                                    .fetch_clone(dir, https.to_string(), committish, semver, info)
                                    .await
                                {
                                    Ok(_) => {}
                                    Err(_) => {
                                        self.fetch_clone(dir, ssh, committish, semver, info)
                                            .await?;
                                    }
                                }
                            }
                        }
                    } else {
                        panic!("Something is seriously wrong with hosted git deps.")
                    }
                }
                _ => unreachable!(),
            },
        }
        Ok(())
    }

    async fn fetch_tarball(&self, dir: &Path, tarball: &Url) -> Result<()> {
        let tarball = self.client.stream_external(tarball).await?;
        Tarball::new_unchecked(tarball)
            .extract_from_tarball_data(dir, None, crate::ExtractMode::AutoHardlink)
            .await?;
        Ok(())
    }

    async fn fetch_clone(
        &self,
        dir: &Path,
        repo: impl AsRef<str>,
        committish: &Option<String>,
        semver: &Option<Range>,
        info: &GitInfo,
    ) -> Result<()> {
        let repo = repo.as_ref();
        let git = self
            .git
            .get_or_try_init(|| which::which("git").map_err(NassunError::WhichGit))?;
        Command::new(git)
            .arg("clone")
            .arg(repo)
            .arg("package")
            .current_dir(dir)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .await
            .map_err(NassunError::GitIoError)
            .and_then(|status| {
                if status.success() {
                    Ok(())
                } else {
                    Err(NassunError::GitCloneError(String::from(repo)))
                }
            })?;
        let checkout_ref = if let Some(range) = semver {
            let refs_output = Command::new(git)
                .arg("show-ref")
                .arg("--tags")
                .current_dir(dir.join("package"))
                .stdin(Stdio::null())
                .stderr(Stdio::null())
                .output()
                .await
                .map_err(NassunError::GitIoError)?;
            let versions: Vec<Version> = String::from_utf8(refs_output.stdout)
                .map_err(|e| {
                    NassunError::MiscError(format!("Could not decode git output as UTF-8. {}", e))
                })?
                .lines()
                .filter_map(|line| {
                    line.split('/')
                        .last()
                        .and_then(|tag| Version::parse(tag).ok())
                })
                .collect();
            Some(
                versions
                    .iter()
                    .filter(|v| range.satisfies(v))
                    .max()
                    .ok_or_else(|| NassunError::NoVersion {
                        name: repo.to_string(),
                        spec: PackageSpec::Git(info.clone()),
                        versions: versions.iter().map(|v| v.to_string()).collect(),
                    })?
                    .to_string(),
            )
        } else {
            committish.clone()
        };
        if let Some(checkout_ref) = checkout_ref {
            Command::new(git)
                .arg("checkout")
                .arg(&checkout_ref)
                .current_dir(dir.join("package"))
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status()
                .await
                .map_err(NassunError::GitIoError)
                .and_then(|status| {
                    if status.success() {
                        Ok(())
                    } else {
                        Err(NassunError::GitCheckoutError(
                            String::from(repo),
                            checkout_ref.clone(),
                        ))
                    }
                })?;
        }
        Ok(())
    }
}

#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
impl PackageFetcher for GitFetcher {
    async fn name(&self, spec: &PackageSpec, _base_dir: &Path) -> Result<String> {
        use PackageSpec::*;
        let info = match spec {
            Alias { name, .. } => return Ok(name.clone()),
            Git(info) => info,
            _ => panic!("Only git specs allowed."),
        };
        let dir = tempfile::tempdir().map_err(NassunError::GitIoError)?;
        self.fetch_to_temp_dir(info, dir.path()).await?;
        self.dir_fetcher
            .name_from_path(&dir.path().join("package"))
            .await
    }

    async fn corgi_metadata(&self, pkg: &Package) -> Result<CorgiVersionMetadata> {
        use PackageResolution::*;
        let info = match pkg.resolved() {
            Git { info, .. } => info,
            _ => panic!("Only git specs allowed."),
        };
        let dir = tempfile::tempdir().map_err(NassunError::GitIoError)?;
        self.fetch_to_temp_dir(info, dir.path()).await?;
        self.dir_fetcher
            .corgi_metadata_from_path(&dir.path().join("package"))
            .await
    }

    async fn metadata(&self, pkg: &Package) -> Result<VersionMetadata> {
        use PackageResolution::*;
        let info = match pkg.resolved() {
            Git { info, .. } => info,
            _ => panic!("Only git specs allowed."),
        };
        let dir = tempfile::tempdir().map_err(NassunError::GitIoError)?;
        self.fetch_to_temp_dir(info, dir.path()).await?;
        self.dir_fetcher
            .metadata_from_path(&dir.path().join("package"))
            .await
    }

    async fn corgi_packument(
        &self,
        spec: &PackageSpec,
        _base_dir: &Path,
    ) -> Result<Arc<CorgiPackument>> {
        use PackageSpec::*;
        let info = match spec.target() {
            Git(info) => info,
            _ => panic!("Only git specs allowed."),
        };
        let dir = tempfile::tempdir().map_err(NassunError::GitIoError)?;
        self.fetch_to_temp_dir(info, dir.path()).await?;
        self.dir_fetcher
            .corgi_packument_from_path(&dir.path().join("package"))
            .await
    }

    async fn packument(&self, spec: &PackageSpec, _base_dir: &Path) -> Result<Arc<Packument>> {
        use PackageSpec::*;
        let spec = match spec {
            Alias { spec, .. } => spec,
            spec => spec,
        };
        let info = match spec {
            Git(info) => info,
            _ => panic!("Only git specs allowed."),
        };
        let dir = tempfile::tempdir().map_err(NassunError::GitIoError)?;
        self.fetch_to_temp_dir(info, dir.path()).await?;
        self.dir_fetcher
            .packument_from_path(&dir.path().join("package"))
            .await
    }

    async fn tarball(&self, _pkg: &crate::Package) -> Result<crate::TarballStream> {
        todo!()
    }
}

#[cfg(test)]
mod test {
    use std::{fs::File, io::Write, process};

    use oro_client::OroClient;
    use oro_package_spec::{GitInfo, PackageSpec};
    use tempfile::tempdir;

    use crate::fetch::PackageFetcher;

    use super::GitFetcher;

    fn setup_git_dir() -> miette::Result<tempfile::TempDir> {
        let git_dir = tempdir().unwrap();

        process::Command::new("git")
            .args(["init", "--initial-branch", "main"])
            .current_dir(&git_dir)
            .status()
            .expect("Could not init git test directory");

        process::Command::new("git")
            .args(["config", "user.name", "Orogene Tester"])
            .current_dir(&git_dir)
            .status()
            .expect("Could not set user name");

        process::Command::new("git")
            .args(["config", "user.email", "orogene.tester@example.com"])
            .current_dir(&git_dir)
            .status()
            .expect("Could not set user name");

        let mut package_file = File::create(git_dir.path().join("package.json")).unwrap();
        package_file
            .write_all(
                r#"{
            "name": "oro-test",
            "version": "1.0.0"
        }"#
                .as_bytes(),
            )
            .unwrap();
        drop(package_file);

        // commit the first version
        process::Command::new("git")
            .args(["add", "package.json"])
            .current_dir(&git_dir)
            .status()
            .expect("Could not add package.json to git repo");
        process::Command::new("git")
            .args(["commit", "-m", "First version", "--no-gpg-sign"])
            .current_dir(&git_dir)
            .status()
            .expect("Could not commit first version");
        process::Command::new("git")
            .args(["tag", "--no-sign", "1.0.0"])
            .current_dir(&git_dir)
            .status()
            .expect("Could not tag first version");

        let mut package_file = File::create(git_dir.path().join("package.json")).unwrap();
        package_file
            .write_all(
                r#"{
            "name": "oro-test",
            "version": "1.2.0"
        }"#
                .as_bytes(),
            )
            .unwrap();
        drop(package_file);

        // commit the second version
        process::Command::new("git")
            .args(["commit", "-a", "-m", "Second version", "--no-gpg-sign"])
            .current_dir(&git_dir)
            .status()
            .expect("Could not commit second version");
        process::Command::new("git")
            .args(["tag", "--no-sign", "1.2.0"])
            .current_dir(&git_dir)
            .status()
            .expect("Could not tag first version");

        let mut package_file = File::create(git_dir.path().join("package.json")).unwrap();
        package_file
            .write_all(
                r#"{
            "name": "oro-test",
            "version": "1.5.0"
        }"#
                .as_bytes(),
            )
            .unwrap();
        drop(package_file);

        // commit the third version
        process::Command::new("git")
            .args(["commit", "-a", "-m", "Second version", "--no-gpg-sign"])
            .current_dir(&git_dir)
            .status()
            .expect("Could not commit second version");

        Ok(git_dir)
    }

    #[async_std::test]
    async fn read_name() -> miette::Result<()> {
        let git_dir = setup_git_dir()?;
        let fetcher = GitFetcher::new(OroClient::default());
        let spec = PackageSpec::Git(GitInfo::Url {
            url: format!("file://{}", git_dir.path().to_str().unwrap())
                .parse()
                .unwrap(),
            committish: None,
            semver: None,
        });
        let cache_path = tempdir().unwrap();
        let name = fetcher.name(&spec, cache_path.path()).await?;
        assert_eq!(name, "oro-test");
        Ok(())
    }

    #[async_std::test]
    async fn read_packument() -> miette::Result<()> {
        let git_dir = setup_git_dir()?;
        let fetcher = GitFetcher::new(OroClient::default());
        let tmp = tempdir().unwrap();
        // get last commit
        let packument = fetcher
            .packument(
                &PackageSpec::Git(GitInfo::Url {
                    url: format!("file://{}", git_dir.path().to_str().unwrap())
                        .parse()
                        .unwrap(),
                    committish: None,
                    semver: None,
                }),
                tmp.path(),
            )
            .await?;
        assert!(packument.versions.contains_key(&"1.5.0".parse()?));
        assert_eq!(
            packument
                .versions
                .get(&"1.5.0".parse()?)
                .unwrap()
                .dist
                .file_count,
            None
        );
        // get specific commit (by tag in that case)
        let packument = fetcher
            .packument(
                &PackageSpec::Git(GitInfo::Url {
                    url: format!("file://{}", git_dir.path().to_str().unwrap())
                        .parse()
                        .unwrap(),
                    committish: Some("1.0.0".to_string()),
                    semver: None,
                }),
                tmp.path(),
            )
            .await?;
        assert!(packument.versions.contains_key(&"1.0.0".parse()?));
        assert_eq!(
            packument
                .versions
                .get(&"1.0.0".parse()?)
                .unwrap()
                .dist
                .file_count,
            None
        );
        // get specific commit (by semver tag)
        let packument = fetcher
            .packument(
                &PackageSpec::Git(GitInfo::Url {
                    url: format!("file://{}", git_dir.path().to_str().unwrap())
                        .parse()
                        .unwrap(),
                    committish: None,
                    semver: Some(">1.0.0 <1.5.0".parse()?),
                }),
                tmp.path(),
            )
            .await?;
        assert!(packument.versions.contains_key(&"1.2.0".parse()?));
        assert_eq!(
            packument
                .versions
                .get(&"1.2.0".parse()?)
                .unwrap()
                .dist
                .file_count,
            None
        );
        Ok(())
    }
}
