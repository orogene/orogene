use oro_diagnostics::{Diagnostic, DiagnosticCategory};
use thiserror::Error;

use crate::resolver::ResolverError;

/// Error type returned by all API calls.
#[derive(Error, Debug)]
pub enum RoggaError {
    /// Something went wrong while fetching a package.
    #[error("Something went wrong with fetching a package:\n\t{0}")]
    PackageFetcherError(String),

    /// Something went wrong while trying to parse a PackageArg
    #[error(transparent)]
    PackageSpecError(#[from] oro_package_spec::PackageSpecError),

    #[error(transparent)]
    ResolverError(#[from] ResolverError),

    #[error(transparent)]
    IoError(#[from] std::io::Error),

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
        todo!()
    }

    fn subpath(&self) -> String {
        todo!()
    }

    fn advice(&self) -> Option<String> {
        todo!()
    }
}

/// The result type returned by calls to this library
pub type Result<T> = std::result::Result<T, RoggaError>;
