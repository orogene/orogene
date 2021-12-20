use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;

use dashmap::DashMap;
use oro_common::{
    async_compat::CompatExt,
    async_trait::async_trait,
    futures::{
        io::{self, AsyncRead},
        TryStreamExt,
    },
    reqwest::Client,
    serde_json,
};
use oro_package_spec::PackageSpec;
use url::Url;

use crate::error::TorusError;
use crate::fetch::PackageFetcher;
use crate::package::Package;
use crate::packument::{Packument, VersionMetadata};
use crate::registry::Registry;
use crate::resolver::PackageResolution;

#[derive(Debug)]
pub struct NpmFetcher {
    client: Client,
    /// Corgis are a compressed kind of packument that omits some
    /// "unnecessary" fields (for some common operations during package
    /// management). This can significantly speed up installs, and is done
    /// through a special Accept header on request.
    use_corgi: bool,
    registries: HashMap<String, Registry>,
    packuments: DashMap<Url, Arc<Packument>>,
}

impl NpmFetcher {
    pub fn new(client: Client, use_corgi: bool, registries: HashMap<String, Registry>) -> Self {
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
                .map(|registry| registry.url)
                .unwrap_or_else(|| "https://registry.npmjs.org/".parse().unwrap())
        } else {
            self.registries
                .get("")
                .cloned()
                .map(|registry| registry.url)
                .unwrap_or_else(|| "https://registry.npmjs.org/".parse().unwrap())
        }
    }

    async fn packument_from_name(
        &self,
        scope: &Option<String>,
        name: &str,
    ) -> Result<Arc<Packument>, TorusError> {
        let packument_url = self
            .pick_registry(scope)
            .join(name)
            // This... should not fail unless you did some shenanigans like
            // constructing PackageRequests by hand, so no error code.
            .map_err(TorusError::UrlError)?;
        if let Some(packument) = self.packuments.get(&packument_url) {
            return Ok(packument.value().clone());
        }
        let packument_data = self
            .client
            .get(packument_url.clone())
            .header(
                "Accept",
                if self.use_corgi {
                    "application/vnd.npm.install-v1+json; q=1.0, application/json; q=0.8, */*"
                } else {
                    "application/json"
                },
            )
            .send()
            .compat()
            .await
            .map_err(TorusError::ClientError)?
            .bytes()
            .compat()
            .await
            .map_err(TorusError::ClientError)?;
        let packument: Arc<Packument> =
            Arc::new(serde_json::from_slice(&packument_data[..]).map_err(TorusError::SerdeError)?);
        self.packuments.insert(packument_url, packument.clone());
        Ok(packument)
    }
}

#[async_trait]
impl PackageFetcher for NpmFetcher {
    async fn name(&self, spec: &PackageSpec, _base_dir: &Path) -> Result<String, TorusError> {
        match spec {
            PackageSpec::Npm { ref name, .. } | PackageSpec::Alias { ref name, .. } => {
                Ok(name.clone())
            }
            _ => unreachable!(),
        }
    }

    async fn metadata(&self, pkg: &Package) -> Result<VersionMetadata, TorusError> {
        let wanted = match pkg.resolved() {
            PackageResolution::Npm { ref version, .. } => version,
            _ => unreachable!(),
        };
        let packument = self.packument(pkg.from(), Path::new("")).await?;
        packument
            .versions
            .get(wanted)
            .cloned()
            .ok_or_else(|| TorusError::MissingVersion(pkg.from().clone(), wanted.clone()))
    }

    async fn packument(
        &self,
        spec: &PackageSpec,
        _base_dir: &Path,
    ) -> Result<Arc<Packument>, TorusError> {
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

    async fn tarball(
        &self,
        pkg: &Package,
    ) -> Result<Box<dyn AsyncRead + Unpin + Send + Sync>, TorusError> {
        // NOTE: This .clone() is so we can free up the client lock, which
        // would otherwise, you know, make it so we can only make one request
        // at a time :(
        let url = match pkg.resolved() {
            PackageResolution::Npm { ref tarball, .. } => tarball,
            _ => panic!("How did a non-Npm resolution get here?"),
        };
        let stream = Box::pin(
            self.client
                .get(url.to_string())
                .send()
                .compat()
                .await
                .map_err(TorusError::ClientError)?
                .bytes_stream()
                .map_err(|e| io::Error::new(io::ErrorKind::Other, e)),
        );
        Ok(Box::new(stream.into_async_read()))
    }
}
