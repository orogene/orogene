use std::{collections::HashMap, path::Path};

use crate::{fetch::PackageFetcher, package::Package};

use async_std::sync::Arc;
use async_trait::async_trait;
use node_semver::Version;
use oro_common::{CorgiManifest, CorgiPackument, CorgiVersionMetadata, Packument, VersionMetadata};
use oro_package_spec::PackageSpec;

use crate::error::{NassunError, Result};

#[derive(Debug)]
pub(crate) struct DummyFetcher(pub(crate) CorgiManifest);

#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
impl PackageFetcher for DummyFetcher {
    async fn name(&self, _spec: &PackageSpec, _base_dir: &Path) -> Result<String> {
        Ok(self
            .0
            .name
            .clone()
            .ok_or_else(|| NassunError::DummyNoName)?)
    }

    async fn metadata(&self, pkg: &Package) -> Result<VersionMetadata> {
        Ok(self.corgi_metadata(pkg).await?.into())
    }

    async fn packument(&self, pkg: &PackageSpec, base_dir: &Path) -> Result<Arc<Packument>> {
        let corgi_packument = self.corgi_packument(pkg, base_dir).await?;
        let cloned = (*corgi_packument).clone();
        Ok(Arc::new(cloned.into()))
    }

    async fn corgi_metadata(&self, _pkg: &Package) -> Result<CorgiVersionMetadata> {
        let corgi_meta: CorgiVersionMetadata = self.0.clone().into();
        Ok(corgi_meta)
    }

    async fn corgi_packument(
        &self,
        _pkg: &PackageSpec,
        _base_dir: &Path,
    ) -> Result<Arc<CorgiPackument>> {
        let corgi_meta: CorgiVersionMetadata = self.0.clone().into();
        let mut packument = CorgiPackument {
            versions: HashMap::new(),
            tags: HashMap::new(),
        };
        let version = corgi_meta
            .manifest
            .version
            .clone()
            .unwrap_or_else(|| Version::parse("0.0.0").expect("Oops, typo"));
        packument.tags.insert("latest".into(), version.clone());
        Ok(Arc::new(packument))
    }

    async fn tarball(&self, pkg: &Package) -> Result<crate::TarballStream> {
        Err(NassunError::UnsupportedDummyOperation(format!(
            "tarball({:?})",
            pkg
        )))
    }
}
