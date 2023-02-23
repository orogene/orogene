use std::path::{Path, PathBuf};

use async_process::{Command, Stdio};
use async_std::sync::{Arc, Mutex};
use async_trait::async_trait;
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
    git: Arc<Mutex<Option<PathBuf>>>,
}

impl GitFetcher {
    pub(crate) fn new(client: OroClient) -> Self {
        Self {
            client,
            dir_fetcher: DirFetcher::new(),
            git: Arc::new(Mutex::new(None)),
        }
    }

    async fn fetch_to_temp_dir(&self, info: &GitInfo, dir: &Path) -> Result<()> {
        match info {
            GitInfo::Url {
                url, committish, ..
            } => {
                self.fetch_clone(dir, url.to_string(), committish).await?;
            }
            GitInfo::Ssh {
                ssh, committish, ..
            } => {
                self.fetch_clone(dir, ssh, committish).await?;
            }
            hosted @ GitInfo::Hosted { .. } => match &hosted {
                GitInfo::Hosted {
                    requested,
                    committish,
                    ..
                } => {
                    if let Some(requested) = requested {
                        self.fetch_clone(dir, requested, committish).await?;
                    } else if let (Some(tarball), Some(https), Some(ssh)) =
                        (hosted.tarball(), hosted.https(), hosted.ssh())
                    {
                        match self.fetch_tarball(dir, &tarball).await {
                            Ok(_) => {}
                            Err(_) => {
                                match self.fetch_clone(dir, https.to_string(), committish).await {
                                    Ok(_) => {}
                                    Err(_) => {
                                        self.fetch_clone(dir, ssh, committish).await?;
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
            .to_temp()
            .await?
            .extract_to_dir(dir)?;
        Ok(())
    }

    async fn fetch_clone(
        &self,
        dir: &Path,
        repo: impl AsRef<str>,
        committish: &Option<String>,
    ) -> Result<()> {
        let repo = repo.as_ref();
        let git = if let Some(git) = self.git.lock().await.as_ref() {
            git.clone()
        } else {
            let git = which::which("git").map_err(NassunError::WhichGit)?;
            let mut selfgit = self.git.lock().await;
            *selfgit = Some(git.clone());
            git
        };
        Command::new(&git)
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
        if let Some(committish) = committish {
            Command::new(&git)
                .arg("checkout")
                .arg(committish)
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
                            committish.clone(),
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
