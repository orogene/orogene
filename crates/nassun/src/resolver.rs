use std::{fmt::Display, path::PathBuf};

use async_trait::async_trait;
use miette::Diagnostic;
use node_semver::Version;
use oro_package_spec::{GitInfo, PackageSpec};
use thiserror::Error;
use url::Url;

use crate::request::PackageRequest;

#[derive(Debug, Error, Diagnostic)]
pub enum ResolverError {
    #[error("No matching `{name}` version found for spec `{spec}`.")]
    #[diagnostic(
        code(classic_resolver::no_matching_version),
        // TODO: format help string using variables?
        help("Try using `oro view` to see what versions are available")
    )]
    NoVersion {
        name: String,
        spec: PackageSpec,
        versions: Vec<String>,
    },

    #[error("{0}")]
    OtherError(Box<dyn Diagnostic + Send + Sync + 'static>),
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

impl Display for PackageResolution {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use PackageResolution::*;
        match self {
            Npm { tarball, .. } => write!(f, "{}", tarball),
            Dir { path } => write!(f, "{}", path.to_string_lossy()),
            Git(info) => write!(f, "{}", info),
        }
    }
}
