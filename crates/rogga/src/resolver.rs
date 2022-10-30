use std::path::PathBuf;

use async_trait::async_trait;
use http_types::Url;
use node_semver::Version;
use oro_diagnostics::{Diagnostic, DiagnosticCategory, Explain};
use oro_package_spec::{GitInfo, PackageSpec};
use thiserror::Error;

use crate::request::PackageRequest;

#[derive(Debug, Error, Diagnostic)]
pub enum ResolverError {
    #[error("No matching `{name}` version found for spec `{spec}`.")]
    #[label("classic_resolver::no_matching_version")]
    // TODO: format advice string using variables?
    #[advice("Try using `oro view` to see what versions are available")]
    NoVersion {
        name: String,
        spec: PackageSpec,
        versions: Vec<String>,
    },

    #[error(transparent)]
    OtherError(
        #[from]
        #[ask]
        Box<dyn Diagnostic>,
    ),
}

impl Explain for ResolverError {}

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
    F: Fn(&PackageRequest) -> std::result::Result<PackageResolution, ResolverError> + Sync + Send,
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
