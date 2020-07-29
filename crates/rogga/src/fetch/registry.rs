use async_std::sync::{Arc, Mutex};
use async_trait::async_trait;
use futures::io::AsyncRead;
use http_types::Method;
use oro_client::{self, OroClient};

use super::PackageFetcher;

use crate::error::{Error, Internal, Result};
use crate::package::{Package, PackageRequest, PackageResolution};
use crate::packument::{Manifest, Packument};

pub struct RegistryFetcher {
    client: Arc<Mutex<OroClient>>,
    packument: Option<Packument>,
}

impl RegistryFetcher {
    pub fn new(client: Arc<Mutex<OroClient>>) -> Self {
        Self {
            client,
            packument: None,
        }
    }
}

impl RegistryFetcher {
    async fn packument_from_name<T: AsRef<str>>(&mut self, name: T) -> Result<&Packument> {
        if self.packument.is_none() {
            let client = self.client.lock().await.clone();
            let opts = client.opts(Method::Get, name.as_ref());
            self.packument = Some(
                client
                    .send(opts.header(
                        "Accept",
                        "application/vnd.npm.install-v1+json; q=1.0, application/json; q=0.8, */*",
                    ))
                    .await
                    .with_context(|| "Failed to get packument.".into())?
                    .body_json::<Packument>()
                    .await
                    .map_err(|e| Error::MiscError(e.to_string()))?,
            );
        }
        Ok(self.packument.as_ref().unwrap())
    }
}

#[async_trait]
impl PackageFetcher for RegistryFetcher {
    async fn manifest(&mut self, pkg: &Package) -> Result<Manifest> {
        let wanted = match pkg.resolved {
            PackageResolution::Npm { ref version, .. } => version,
            _ => panic!("How did a non-Npm resolution get here?"),
        };
        let client = self.client.lock().await.clone();
        let opts = client.opts(Method::Get, format!("{}/{}", pkg.name, wanted));
        let info = client
            .send(opts)
            .await
            .with_context(|| "Failed to get manifest.".into())?
            .body_json::<Manifest>()
            .await
            .map_err(|e| Error::MiscError(e.to_string()))?;
        Ok(info)
    }

    async fn packument(&mut self, pkg: &PackageRequest) -> Result<Packument> {
        // TODO: get rid of this clone, maybe?
        Ok(self.packument_from_name(pkg.name().await?).await?.clone())
    }

    async fn tarball(&mut self, pkg: &Package) -> Result<Box<dyn AsyncRead + Unpin + Send + Sync>> {
        // NOTE: This .clone() is so we can free up the client lock, which
        // would otherwise, you know, make it so we can only make one request
        // at a time :(
        let client = self.client.lock().await.clone();
        let url = match pkg.resolved {
            PackageResolution::Npm { ref tarball, .. } => tarball,
            _ => panic!("How did a non-Npm resolution get here?"),
        };
        Ok(Box::new(
            client
                .send(client.opts(Method::Get, url))
                .await
                .with_context(|| "Failed to get packument.".into())?,
        ))
    }
}
