use std::path::PathBuf;

use oro_diagnostics::{Diagnostic, DiagnosticCategory};
use oro_node_semver::Version;
use oro_package_spec::PackageSpec;
use thiserror::Error;

use crate::resolver::ResolverError;

/// Error type returned by all API calls.
#[derive(Error, Debug)]
pub enum RoggaError {
    /// Something went wrong while fetching a package.
    #[error("Package for `{0}` was found, but resolved version `{1}` does not exist.")]
    MissingVersion(PackageSpec, Version),

    /// Something went wrong while trying to parse a PackageArg
    #[error(transparent)]
    PackageSpecError(#[from] oro_package_spec::PackageSpecError),

    #[error(transparent)]
    ResolverError(#[from] ResolverError),

    #[error("{0}")]
    IoError(#[source] std::io::Error, PathBuf),

    #[error(transparent)]
    OroClientError(#[from] oro_client::OroClientError),

    #[error(transparent)]
    SerdeError(#[from] serde_json::Error),

    #[error(transparent)]
    UrlError(#[from] url::ParseError),

    /// A miscellaneous, usually internal error. This is used mainly to wrap
    /// either manual InternalErrors, or those using external errors that
    /// don't implement std::error::Error.
    #[error("A miscellaneous error occurred: {0}")]
    MiscError(String),
}

impl Diagnostic for RoggaError {
    fn category(&self) -> DiagnosticCategory {
        use DiagnosticCategory::*;
        use RoggaError::*;
        match self {
            MissingVersion(..) => Misc,
            PackageSpecError(err) => err.category(),
            ResolverError(err) => err.category(),
            IoError(_, ref path) => Fs { path: path.clone() },
            OroClientError(err) => err.category(),
            SerdeError(_) => todo!(),
            UrlError(_) => todo!(),
            MiscError(_) => Misc,
        }
    }

    fn subpath(&self) -> String {
        use RoggaError::*;
        match self {
            MissingVersion(..) => "rogga::missing_version".into(),
            PackageSpecError(err) => err.subpath(),
            ResolverError(err) => err.subpath(),
            IoError(_, _) => "rogga::dir::read".into(),
            OroClientError(err) => err.subpath(),
            SerdeError(_) => "rogga::serde".into(),
            UrlError(_) => "rogga::bad_url".into(),
            MiscError(_) => "rogga::misc".into(),
        }
    }

    fn advice(&self) -> Option<String> {
        use RoggaError::*;
        match self {
            MissingVersion(..) => {
                Some("Try using `oro view` to see what versions are available".into())
            }
            PackageSpecError(err) => err.advice(),
            ResolverError(err) => err.advice(),
            IoError(..) => None,
            OroClientError(err) => err.advice(),
            SerdeError(..) => None,
            UrlError(..) => None,
            MiscError(..) => None,
        }
    }
}

/// The result type returned by calls to this library
pub type Result<T> = std::result::Result<T, RoggaError>;
