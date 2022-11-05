use std::path::Path;

use async_std::sync::Arc;
use async_trait::async_trait;
use oro_common::{Packument, VersionMetadata};
use oro_package_spec::PackageSpec;

use crate::error::Result;
use crate::package::Package;

#[cfg(not(target_arch = "wasm32"))]
pub(crate) use dir::DirFetcher;
#[cfg(not(target_arch = "wasm32"))]
pub(crate) use git::GitFetcher;
pub(crate) use npm::NpmFetcher;

#[cfg(not(target_arch = "wasm32"))]
mod dir;
#[cfg(not(target_arch = "wasm32"))]
mod git;
mod npm;

#[cfg(not(target_arch = "wasm32"))]
#[async_trait]
pub trait PackageFetcher: std::fmt::Debug + Send + Sync {
    async fn name(&self, spec: &PackageSpec, base_dir: &Path) -> Result<String>;
    async fn metadata(&self, pkg: &Package) -> Result<VersionMetadata>;
    async fn packument(&self, pkg: &PackageSpec, base_dir: &Path) -> Result<Arc<Packument>>;
    async fn tarball(&self, pkg: &Package) -> Result<crate::TarballStream>;
}

#[async_trait(?Send)]
#[cfg(target_arch = "wasm32")]
pub trait PackageFetcher: std::fmt::Debug {
    async fn name(&self, spec: &PackageSpec, base_dir: &Path) -> Result<String>;
    async fn metadata(&self, pkg: &Package) -> Result<VersionMetadata>;
    async fn packument(&self, pkg: &PackageSpec, base_dir: &Path) -> Result<Arc<Packument>>;
    async fn tarball(&self, pkg: &Package) -> Result<crate::TarballStream>;
}
