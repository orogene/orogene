use std::path::PathBuf;

use async_trait::async_trait;
use http_types::Url;
use oro_diagnostics::{Diagnostic, DiagnosticCategory};
use oro_node_semver::Version;
use oro_package_spec::PackageSpec;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::request::PackageRequest;

#[derive(Debug, Error)]
pub enum ResolverError {
    #[error("No matching `{name}` version found for spec `{spec}`.")]
    NoVersion {
        name: String,
        spec: PackageSpec,
        versions: Vec<String>,
    },
    #[error(transparent)]
    OtherError(#[from] Box<dyn Diagnostic>),
}

impl Diagnostic for ResolverError {
    fn category(&self) -> DiagnosticCategory {
        DiagnosticCategory::Misc
    }

    fn subpath(&self) -> String {
        use ResolverError::*;
        match self {
            NoVersion { .. } => "resolver::no_matching_version".into(),
            OtherError(err) => err.subpath(),
        }
    }

    fn advice(&self) -> Option<String> {
        use ResolverError::*;
        match self {
            NoVersion { ref name, .. } => Some(format!(
                "Try using `oro view {}` to see what versions are available",
                name
            )),
            OtherError(err) => err.advice(),
        }
    }
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
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum PackageResolution {
    Npm { version: Version, tarball: Url },
    Dir { path: PathBuf },
}
