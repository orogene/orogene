use oro_diagnostics::{Diagnostic, DiagnosticCategory};
use oro_package_spec::PackageSpecError;
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
    PackageSpecError(#[from] PackageSpecError),

    #[error(transparent)]
    ResolverError(#[from] ResolverError),

    #[error("Failed to deserialize package data for `{name}`:\n\t{serde_error}")]
    SerdeError {
        name: String,
        data: String,
        #[source]
        serde_error: serde_json::Error,
    },

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
