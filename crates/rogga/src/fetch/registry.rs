use async_std::sync::{Arc, Mutex};
use async_trait::async_trait;
use futures::io::AsyncRead;
use http_types::Method;
use oro_client::{self, OroClient};
use oro_package_spec::PackageSpec;

use crate::error::{Error, Internal, Result};
use crate::fetch::PackageFetcher;
use crate::package::{Package, PackageResolution};
use crate::packument::{Packument, VersionMetadata};

#[derive(Debug)]
pub struct RegistryFetcher {
    client: Arc<Mutex<OroClient>>,
    packument: Option<Packument>,
    /// Corgis are a compressed kind of packument that omits some
    /// "unnecessary" fields (for some common operations during package
    /// management). This can significantly speed up installs, and is done
    /// through a special Accept header on request.
    use_corgi: bool,
}

impl RegistryFetcher {
    pub fn new(client: Arc<Mutex<OroClient>>, use_corgi: bool) -> Self {
        Self {
            client,
            packument: None,
            use_corgi,
        }
    }
}

impl RegistryFetcher {
    async fn packument_from_name(
        &mut self,
        scope: &Option<String>,
        name: &str,
    ) -> Result<&Packument> {
        if self.packument.is_none() {
            let client = self.client.lock().await.clone();
            let full_name = format!(
                "{}{}",
                scope
                    .clone()
                    .map(|s| format!("@{}/", s))
                    .unwrap_or_else(|| String::from("")),
                name
            );
            let opts = client.opts(Method::Get, &full_name);
            let packument_data = client
                .send(opts.header(
                    "Accept",
                    if self.use_corgi {
                        "application/vnd.npm.install-v1+json; q=1.0, application/json; q=0.8, */*"
                    } else {
                        "application/json"
                    },
                ))
                .await
                .with_context(|| format!("Failed to get packument for {}.", full_name))?
                .body_string()
                .await
                .map_err(|e| Error::MiscError(e.to_string()))?;
            // let val: serde_json::Value = serde_json::from_str(&packument_data).to_internal()?;
            // println!("{:#?}", val);
            self.packument =
                serde_json::from_str(&packument_data).map_err(|err| Error::SerdeError {
                    name: full_name,
                    data: packument_data,
                    serde_error: err,
                })?;
        }
        Ok(self.packument.as_ref().unwrap())
    }
}

#[async_trait]
impl PackageFetcher for RegistryFetcher {
    async fn name(&mut self, spec: &PackageSpec) -> Result<String> {
        match spec {
            // TODO: scopes
            PackageSpec::Npm { ref name, .. } | PackageSpec::Alias { ref name, .. } => {
                Ok(name.clone())
            }
            _ => unreachable!(),
        }
    }

    async fn metadata(&mut self, pkg: &Package) -> Result<VersionMetadata> {
        let wanted = match pkg.resolved {
            PackageResolution::Npm { ref version, .. } => version,
            _ => panic!("How did a non-Npm resolution get here?"),
        };
        let packument = self.packument(&pkg.from).await?;
        // TODO: unwrap
        Ok(packument.versions.get(&wanted).unwrap().clone())
    }

    async fn packument(&mut self, spec: &PackageSpec) -> Result<Packument> {
        // When fetching the packument itself, we need the _package_ name, not
        // its alias! Hence these shenanigans.
        let pkg = match spec {
            PackageSpec::Alias { ref package, .. } => package,
            pkg @ PackageSpec::Npm { .. } => pkg,
            _ => unreachable!(),
        };
        if let PackageSpec::Npm {
            ref scope,
            ref name,
            ..
        } = pkg
        {
            Ok(self.packument_from_name(scope, name).await?.clone())
        } else {
            unreachable!()
        }
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
                .with_context(|| format!("Failed to get tarball for {:#?}.", pkg.resolved))?,
        ))
    }
}
