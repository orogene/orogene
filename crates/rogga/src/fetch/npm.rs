use std::collections::HashMap;
use std::path::Path;

use async_std::sync::{Arc, Mutex};
use async_trait::async_trait;
use dashmap::DashMap;
use futures::io::AsyncRead;
use http_types::Method;
use oro_client::{self, OroClient};
use oro_package_spec::PackageSpec;
use url::Url;

use crate::error::{Result, RoggaError};
use crate::fetch::PackageFetcher;
use crate::package::Package;
use crate::packument::{Packument, VersionMetadata};
use crate::resolver::PackageResolution;

#[derive(Debug)]
pub struct NpmFetcher {
    client: Arc<Mutex<OroClient>>,
    /// Corgis are a compressed kind of packument that omits some
    /// "unnecessary" fields (for some common operations during package
    /// management). This can significantly speed up installs, and is done
    /// through a special Accept header on request.
    use_corgi: bool,
    registries: HashMap<String, Url>,
    packuments: DashMap<Url, Arc<Packument>>,
}

impl NpmFetcher {
    pub fn new(
        client: Arc<Mutex<OroClient>>,
        use_corgi: bool,
        registries: HashMap<String, Url>,
    ) -> Self {
        Self {
            client,
            use_corgi,
            registries,
            packuments: DashMap::new(),
        }
    }
}

impl NpmFetcher {
    fn pick_registry(&self, scope: &Option<String>) -> Url {
        if let Some(scope) = scope {
            self.registries
                .get(scope)
                .or_else(|| self.registries.get(""))
                .cloned()
                .unwrap_or_else(|| "https://registry.npmjs.org/".parse().unwrap())
        } else {
            self.registries
                .get("")
                .cloned()
                .unwrap_or_else(|| "https://registry.npmjs.org/".parse().unwrap())
        }
    }

    async fn packument_from_name(
        &self,
        scope: &Option<String>,
        name: &str,
    ) -> Result<Arc<Packument>> {
        let client = self.client.lock().await.clone();
        let packument_url = self
            .pick_registry(scope)
            .join(name)
            // This... should not fail unless you did some shenanigans like
            // constructing PackageRequests by hand, so no error code.
            .map_err(RoggaError::UrlError)?;
        if let Some(packument) = self.packuments.get(&packument_url) {
            return Ok(packument.value().clone());
        }
        let opts = client.opts(Method::Get, packument_url.clone());
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
            .map_err(RoggaError::OroClientError)?
            .body_string()
            .await
            .map_err(|e| RoggaError::MiscError(e.to_string()))?;
        let packument: Arc<Packument> =
            Arc::new(serde_json::from_str(&packument_data).map_err(RoggaError::SerdeError)?);
        self.packuments.insert(packument_url, packument.clone());
        Ok(packument)
    }
}

#[async_trait]
impl PackageFetcher for NpmFetcher {
    async fn name(&self, spec: &PackageSpec, _base_dir: &Path) -> Result<String> {
        match spec {
            PackageSpec::Npm { ref name, .. } | PackageSpec::Alias { ref name, .. } => {
                Ok(name.clone())
            }
            _ => unreachable!(),
        }
    }

    async fn metadata(&self, pkg: &Package) -> Result<VersionMetadata> {
        let wanted = match pkg.resolved() {
            PackageResolution::Npm { ref version, .. } => version,
            _ => unreachable!(),
        };
        let packument = self.packument(pkg.from(), Path::new("")).await?;
        packument
            .versions
            .get(wanted)
            .cloned()
            .ok_or_else(|| RoggaError::MissingVersion(pkg.from().clone(), wanted.clone()))
    }

    async fn packument(&self, spec: &PackageSpec, _base_dir: &Path) -> Result<Arc<Packument>> {
        // When fetching the packument itself, we need the _package_ name, not
        // its alias! Hence these shenanigans.
        let pkg = match spec {
            PackageSpec::Alias { ref spec, .. } => spec,
            pkg @ PackageSpec::Npm { .. } => pkg,
            _ => unreachable!(),
        };
        if let PackageSpec::Npm {
            ref scope,
            ref name,
            ..
        } = pkg
        {
            Ok(self.packument_from_name(scope, name).await?)
        } else {
            unreachable!()
        }
    }

    async fn tarball(&self, pkg: &Package) -> Result<Box<dyn AsyncRead + Unpin + Send + Sync>> {
        // NOTE: This .clone() is so we can free up the client lock, which
        // would otherwise, you know, make it so we can only make one request
        // at a time :(
        let client = self.client.lock().await.clone();
        let url = match pkg.resolved() {
            PackageResolution::Npm { ref tarball, .. } => tarball,
            _ => panic!("How did a non-Npm resolution get here?"),
        };
        Ok(Box::new(
            client
                .send(client.opts(Method::Get, url.clone()))
                .await
                .map_err(RoggaError::OroClientError)?,
        ))
    }
}
