use std::path::PathBuf;

use thiserror::Error;

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
    /// Returned when an index entry could not be found during
    /// lookup.
    #[error("Entry not found for key {1:?} in cache {0:?}")]
    EntryNotFound(PathBuf, String),

    /// Returned when a size check has failed.
    #[error("Size check failed.\n\tWanted: {0}\n\tActual: {1}")]
    SizeError(usize, usize),

    /// Returned when an integrity check has failed.
    #[error(transparent)]
    IntegrityError {
        #[from]
        /// The underlying error
        source: ssri::Error,
    },

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
