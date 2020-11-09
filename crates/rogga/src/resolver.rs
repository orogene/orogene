use std::path::PathBuf;

use async_trait::async_trait;
use http_types::Url;
use oro_diagnostics::{Diagnostic, DiagnosticCode};
use oro_node_semver::Version;
use oro_package_spec::PackageSpec;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::request::PackageRequest;

#[derive(Debug, Error)]
pub enum ResolverError {
    #[error("No matching version found for spec {name}@{spec:?} in {versions:#?}.")]
    NoVersion {
        code: DiagnosticCode,
        name: String,
        spec: PackageSpec,
        versions: Vec<String>,
    },
    #[error(transparent)]
    OtherError(#[from] Box<dyn Diagnostic>),
}

impl Diagnostic for ResolverError {
    fn code(&self) -> DiagnosticCode {
        use ResolverError::*;
        match self {
            NoVersion { code, .. } => *code,
            // TODO: Maybe make the value of OtherError a Diagnostic?
            OtherError(..) => DiagnosticCode::OR1000,
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
