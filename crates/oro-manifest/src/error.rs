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
    #[error("Failed to parse person string `{input}`: {msg}")]
    ParsePersonError { input: String, msg: String },

    #[error("Invalid package file {0}. Package files should be JSON objects")]
    InvalidPackageFile(PathBuf),

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
