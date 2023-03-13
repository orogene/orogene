use std::collections::HashMap;
use std::path::Path;

use async_std::sync::Arc;
use async_trait::async_trait;
use dashmap::DashMap;
use oro_client::{self, OroClient};
use oro_common::{CorgiPackument, CorgiVersionMetadata, Packument, VersionMetadata};
use oro_package_spec::PackageSpec;
use url::Url;

use crate::error::{NassunError, Result};
use crate::fetch::PackageFetcher;
use crate::package::Package;
use crate::resolver::PackageResolution;

#[derive(Debug)]
pub(crate) struct NpmFetcher {
    client: OroClient,
    registries: HashMap<Option<String>, Url>,
    cache_packuments: bool,
    packuments: DashMap<String, Arc<Packument>>,
    corgi_packuments: DashMap<String, Arc<CorgiPackument>>,
}

impl NpmFetcher {
    pub(crate) fn new(
        client: OroClient,
        registries: HashMap<Option<String>, Url>,
        cache_packuments: bool,
    ) -> Self {
        Self {
            client,
            registries,
            packuments: DashMap::new(),
            corgi_packuments: DashMap::new(),
            cache_packuments,
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

impl NpmFetcher {
    fn _name<'a>(&'a self, spec: &'a PackageSpec) -> &'a str {
        match spec {
            PackageSpec::Npm { ref name, .. } | PackageSpec::Alias { ref name, .. } => name,
            _ => unreachable!(),
        }
    }
}

#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
impl PackageFetcher for NpmFetcher {
    async fn name(&self, spec: &PackageSpec, _base_dir: &Path) -> Result<String> {
        Ok(self._name(spec).to_string())
    }

    async fn corgi_metadata(&self, pkg: &Package) -> Result<CorgiVersionMetadata> {
        let wanted = match pkg.resolved() {
            PackageResolution::Npm { ref version, .. } => version,
            _ => unreachable!(),
        };
        let packument = self.corgi_packument(pkg.from(), Path::new("")).await?;
        packument
            .versions
            .get(wanted)
            .cloned()
            .ok_or_else(|| NassunError::MissingVersion(pkg.from().clone(), wanted.clone()))
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
            .ok_or_else(|| NassunError::MissingVersion(pkg.from().clone(), wanted.clone()))
    }

    async fn corgi_packument(
        &self,
        spec: &PackageSpec,
        _base_dir: &Path,
    ) -> Result<Arc<CorgiPackument>> {
        // When fetching the packument itself, we need the _package_ name, not
        // its alias! Hence these shenanigans.
        if let PackageSpec::Npm {
            ref name,
            ref scope,
            ..
        } = spec.target()
        {
            if let Some(packument) = self.corgi_packuments.get(name) {
                if self.cache_packuments {
                    return Ok(packument.value().clone());
                }
            }
            let client = self.client.with_registry(self.pick_registry(scope));
            let packument = Arc::new(client.corgi_packument(&name).await?);
            if self.cache_packuments {
                self.corgi_packuments
                    .insert(name.clone(), packument.clone());
            }
            Ok(packument)
        } else {
            unreachable!("How did a non-Npm resolution get here?");
        }
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
            ref name,
            ref scope,
            ..
        } = pkg
        {
            if let Some(packument) = self.packuments.get(name) {
                if self.cache_packuments {
                    return Ok(packument.value().clone());
                }
            }
            let client = self.client.with_registry(self.pick_registry(scope));
            let packument = Arc::new(client.packument(&name).await?);
            if self.cache_packuments {
                self.packuments.insert(name.clone(), packument.clone());
            }
            Ok(packument)
        } else {
            unreachable!()
        }
    }

    async fn tarball(&self, pkg: &Package) -> Result<crate::TarballStream> {
        let url = match pkg.resolved() {
            PackageResolution::Npm { ref tarball, .. } => tarball,
            _ => panic!("How did a non-Npm resolution get here?"),
        };
        Ok(self.client.stream_external(url).await?)
    }
}
