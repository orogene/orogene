use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    /// Should probably be an internal error. Signals that we tried to match
    /// two packages with different names.
    #[error("{0} and {1} do not match.")]
    NameMismatch(String, String),
    #[error("Tag '{0}' does not exist in registry.")]
    TagNotFound(String),
    /// Error returned from Rogga
    #[error(transparent)]
    RoggaError {
        #[from]
        source: rogga::Error,
    },
    /// Returned if an internal (e.g. io) operation has failed.
    #[error(transparent)]
    InternalError {
        #[from]
        /// The underlying error
        source: InternalError,
    },
}

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

/// The result type returned by calls to this library
pub type Result<T> = std::result::Result<T, Error>;

pub type InternalResult<T> = std::result::Result<T, InternalError>;
