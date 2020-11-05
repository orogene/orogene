use package_spec::PackageArgError;
use thiserror::Error;

use crate::package::ResolverError;

#[derive(Error, Debug)]
#[error("{source}\n\n  {}", context.join("\n  "))]
pub struct InternalError {
    source: Box<dyn std::error::Error + Send + Sync>,
    context: Vec<String>,
}

pub trait Internal<T> {
    fn to_internal(self) -> InternalResult<T>;
    fn with_context<F: FnOnce() -> String>(self, f: F) -> InternalResult<T>;
}

impl<T, E: 'static + std::error::Error + Send + Sync> Internal<T> for std::result::Result<T, E> {
    fn to_internal(self) -> InternalResult<T> {
        self.map_err(|e| InternalError {
            source: Box::new(e),
            context: Vec::new(),
        })
    }

    fn with_context<F: FnOnce() -> String>(self, f: F) -> InternalResult<T> {
        self.map_err(|e| InternalError {
            source: Box::new(e),
            context: vec![f()],
        })
    }
}

/// Error type returned by all API calls.
#[derive(Error, Debug)]
pub enum Error {
    /// Something went wrong while fetching a package.
    #[error("Something went wrong with fetching a package.")]
    PackageFetcherError(String),

    /// Something went wrong while trying to parse a PackageArg
    #[error(transparent)]
    PackageArgError(#[from] PackageArgError),

    #[error(transparent)]
    ResolverError(#[from] ResolverError),

    #[error("Failed to deserialize package data for {name}: {serde_error}\n{}",
            &.data[(.serde_error.column() - 100) .. (.serde_error.column() + 30)])]
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

    /// Returned if an internal (e.g. io) operation has failed.
    #[error(transparent)]
    InternalError {
        #[from]
        /// The underlying error
        source: InternalError,
    },
}

/// The result type returned by calls to this library
pub type Result<T> = std::result::Result<T, Error>;

pub type InternalResult<T> = std::result::Result<T, InternalError>;
