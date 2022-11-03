use std::path::Path;

use async_std::sync::Arc;
use async_trait::async_trait;
use futures::io::AsyncRead;
use oro_common::{Packument, VersionMetadata};
use oro_package_spec::PackageSpec;

use crate::error::Result;
use crate::package::Package;

pub(crate) use dir::DirFetcher;
pub(crate) use git::GitFetcher;
pub(crate) use npm::NpmFetcher;

mod dir;
mod git;
mod npm;

#[async_trait]
pub trait PackageFetcher: std::fmt::Debug + Send + Sync {
    async fn name(&self, spec: &PackageSpec, base_dir: &Path) -> Result<String>;
    async fn metadata(&self, pkg: &Package) -> Result<VersionMetadata>;
    async fn packument(&self, pkg: &PackageSpec, base_dir: &Path) -> Result<Arc<Packument>>;
    async fn tarball(&self, pkg: &Package) -> Result<Box<dyn AsyncRead + Unpin + Send + Sync>>;
}
