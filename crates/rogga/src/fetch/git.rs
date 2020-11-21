use std::path::Path;

use async_process::{Command, Stdio};
use async_std::sync::{Arc, Mutex};
use async_trait::async_trait;
use futures::AsyncRead;
use http_types::Method;
use oro_client::{self, OroClient};
use oro_package_spec::{GitInfo, PackageSpec};
use url::Url;

use crate::error::{Result, RoggaError};
use crate::extract;
use crate::fetch::dir::DirFetcher;
use crate::fetch::PackageFetcher;
use crate::package::Package;
use crate::packument::{Packument, VersionMetadata};

#[derive(Debug)]
pub struct GitFetcher {
    client: Arc<Mutex<OroClient>>,
    dir_fetcher: DirFetcher,
}

impl GitFetcher {
    pub fn new(client: Arc<Mutex<OroClient>>) -> Self {
        Self {
            client,
            dir_fetcher: DirFetcher::new(),
        }
    }

    async fn fetch_to_temp_dir(&self, spec: &PackageSpec, dir: &Path) -> Result<()> {
        use PackageSpec::*;
        let spec = match spec {
            Alias { spec, .. } => spec,
            otherwise => otherwise,
        };
        match spec {
            Git(GitInfo::Url {
                url, committish, ..
            }) => {
                self.fetch_clone(dir, url.to_string(), committish).await?;
            }
            Git(GitInfo::Ssh {
                ssh, committish, ..
            }) => {
                self.fetch_clone(dir, ssh, committish).await?;
            }
            Git(hosted @ GitInfo::Hosted { .. }) => match &hosted {
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
            wtf => panic!("This method should only receive git specs. Got {:#?}", wtf),
        }
        Ok(())
    }

    async fn fetch_tarball(&self, dir: &Path, tarball: &Url) -> Result<()> {
        let client = self.client.lock().await.clone();
        let opts = client.opts(Method::Get, tarball.clone());
        let tarball = client
            .send(opts)
            .await
            .map_err(RoggaError::OroClientError)?;
        extract::extract_to_dir(tarball, dir).await?;
        Ok(())
    }

    async fn fetch_clone(
        &self,
        dir: &Path,
        repo: impl AsRef<str>,
        committish: &Option<String>,
    ) -> Result<()> {
        let repo = repo.as_ref();
        Command::new("git")
            .arg("clone")
            .arg(repo)
            .arg("package")
            .current_dir(dir)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .await
            .map_err(RoggaError::GitIoError)
            .and_then(|status| {
                if status.success() {
                    Ok(())
                } else {
                    Err(RoggaError::GitCloneError(String::from(repo)))
                }
            })?;
        if let Some(committish) = committish {
            Command::new("git")
                .arg("checkout")
                .arg(committish)
                .current_dir(dir.join("package"))
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status()
                .await
                .map_err(RoggaError::GitIoError)
                .and_then(|status| {
                    if status.success() {
                        Ok(())
                    } else {
                        Err(RoggaError::GitCheckoutError(
                            String::from(repo),
                            committish.clone(),
                        ))
                    }
                })?;
        }
        Ok(())
    }
}

#[async_trait]
impl PackageFetcher for GitFetcher {
    async fn name(&self, spec: &PackageSpec, _base_dir: &Path) -> Result<String> {
        let dir = tempfile::tempdir().map_err(RoggaError::GitIoError)?;
        self.fetch_to_temp_dir(spec, dir.path()).await?;
        self.dir_fetcher
            .name_from_path(&dir.path().join("package"))
            .await
    }

    async fn metadata(&self, pkg: &Package) -> Result<VersionMetadata> {
        // TODO: this needs to use the resolved version, I think. But that's after we get semver support.
        let dir = tempfile::tempdir().map_err(RoggaError::GitIoError)?;
        self.fetch_to_temp_dir(&pkg.from, dir.path()).await?;
        self.dir_fetcher
            .metadata_from_path(&dir.path().join("package"))
            .await
    }

    async fn packument(&self, spec: &PackageSpec, _base_dir: &Path) -> Result<Arc<Packument>> {
        let dir = tempfile::tempdir().map_err(RoggaError::GitIoError)?;
        self.fetch_to_temp_dir(spec, dir.path()).await?;
        self.dir_fetcher
            .packument_from_path(&dir.path().join("package"))
            .await
    }

    async fn tarball(
        &self,
        _pkg: &crate::Package,
    ) -> Result<Box<dyn AsyncRead + Unpin + Send + Sync>> {
        todo!()
    }
}
