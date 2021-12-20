use std::path::PathBuf;

use http_types::Url;

use oro_common::{
    async_trait::async_trait,
    miette::{self, Diagnostic},
    node_semver::Version,
    thiserror::{self, Error},
};
use oro_package_spec::{GitInfo, PackageSpec};

use crate::request::PackageRequest;

#[derive(Debug, Error, Diagnostic)]
pub enum ResolverError {
    #[error("No matching `{name}` version found for spec `{spec}`.")]
    #[diagnostic(code(oro_torus::resolver::no_matching_version))]
    NoVersion {
        name: String,
        spec: PackageSpec,
        versions: Vec<String>,
    },

    #[error(transparent)]
    OtherError(Box<dyn std::error::Error + Send + Sync + 'static>),
}

#[async_trait]
pub trait PackageResolver {
    async fn resolve(
        &self,
        wanted: &PackageRequest,
    ) -> std::result::Result<PackageResolution, ResolverError>;
}

#[async_trait]
impl<F> PackageResolver for F
where
    F: Fn(&PackageRequest) -> Result<PackageResolution, ResolverError> + Send + Sync,
{
    async fn resolve(
        &self,
        wanted: &PackageRequest,
    ) -> std::result::Result<PackageResolution, ResolverError> {
        self(wanted)
    }
}

/// Represents a fully-resolved, specific version of a package as it would be fetched.
#[derive(Clone, Debug)]
pub enum PackageResolution {
    Npm { version: Version, tarball: Url },
    Dir { path: PathBuf },
    Git(GitInfo),
}
