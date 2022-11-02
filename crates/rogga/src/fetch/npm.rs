use std::collections::HashMap;
use std::path::Path;

use async_std::sync::Arc;
use async_trait::async_trait;
use dashmap::DashMap;
use futures::io::AsyncRead;
use oro_client::{self, OroClient};
use oro_common::{Packument, VersionMetadata};
use oro_package_spec::PackageSpec;
use url::Url;

use crate::error::{Result, RoggaError};
use crate::fetch::PackageFetcher;
use crate::package::Package;
use crate::resolver::PackageResolution;

#[derive(Debug)]
pub(crate) struct NpmFetcher {
    client: OroClient,
    /// Corgis are a compressed kind of packument that omits some
    /// "unnecessary" fields (for some common operations during package
    /// management). This can significantly speed up installs, and is done
    /// through a special Accept header on request.
    use_corgi: bool,
    registries: HashMap<Option<String>, Url>,
    packuments: DashMap<String, Arc<Packument>>,
}

impl NpmFetcher {
    pub(crate) fn new(
        client: OroClient,
        use_corgi: bool,
        registries: HashMap<Option<String>, Url>,
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
        self.registries
            .get(scope)
            .or_else(|| self.registries.get(&None))
            .cloned()
            .unwrap_or_else(|| "https://registry.npmjs.org/".parse().unwrap())
    }
}

#[async_trait]
impl PackageFetcher for NpmFetcher {
    async fn name(&self, spec: &PackageSpec, _base_dir: &Path) -> Result<String> {
        match spec {
            PackageSpec::Npm { name, .. } | PackageSpec::Alias { name, .. } => {
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
        if let PackageSpec::Npm { ref scope, .. } = pkg {
            let spec_str = format!("{}", pkg);
            if let Some(packument) = self.packuments.get(&spec_str) {
                return Ok(packument.value().clone());
            }
            let client = self.client.with_registry(self.pick_registry(scope));
            let packument = Arc::new(client.packument(&spec_str, self.use_corgi).await?);
            self.packuments.insert(spec_str, packument.clone());
            Ok(packument)
        } else {
            unreachable!()
        }
    }

    async fn tarball(&self, pkg: &Package) -> Result<Box<dyn AsyncRead + Unpin + Send + Sync>> {
        let url = match pkg.resolved() {
            PackageResolution::Npm { ref tarball, .. } => tarball,
            _ => panic!("How did a non-Npm resolution get here?"),
        };
        Ok(self.client.stream_external(url).await?)
    }
}
