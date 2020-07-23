use async_std::sync::{Arc, Mutex};
use async_trait::async_trait;
use futures::io::AsyncRead;
use oro_client::OroClient;

use super::PackageFetcher;

use crate::error::{Error, Internal, Result};
use crate::package::{Manifest, Package, PackageRequest, PackageResolution};
use crate::packument::Packument;

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
            let client = self.client.lock().await;
            self.packument = Some(
                client
                    .get(name.as_ref())
                    .await
                    .with_context(|| "Failed to get packument.".into())?
                    .body_json::<Packument>()
                    .await
                    .map_err(|e| Error::MiscError(e.to_string()))?,
            )
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
        let packument = self.packument_from_name(&pkg.name).await?;
        // TODO: get rid of this .expect()
        let version = packument
            .versions
            .get(&wanted.to_string())
            .expect("What? It should be there");
        Ok(Manifest {
            name: packument.name.clone(),
            version: Some(wanted.clone()),
            // TODO: Make this less reckless.
            integrity: version
                .dist
                .integrity
                .clone()
                .map(|sri_str| sri_str.parse().expect("Failed to parse integrity string")),
            resolved: pkg.resolved.clone(),
        })
    }

    async fn packument(&mut self, pkg: &PackageRequest) -> Result<Packument> {
        // TODO: get rid of this clone, maybe?
        Ok(self.packument_from_name(pkg.name().await?).await?.clone())
    }

    async fn tarball(&mut self, pkg: &Package) -> Result<Box<dyn AsyncRead + Send + Sync>> {
        let client = self.client.lock().await;
        let url = match pkg.resolved {
            PackageResolution::Npm { ref tarball, .. } => tarball,
            _ => panic!("How did a non-Npm resolution get here?"),
        };
        Ok(Box::new(
            client
                .get_absolute(url)
                .await
                .with_context(|| "Failed to get packument.".into())?,
        ))
    }
}
