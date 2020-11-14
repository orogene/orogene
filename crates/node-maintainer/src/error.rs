use oro_diagnostics::{Diagnostic, DiagnosticCode};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum NodeMaintainerError {
    /// Should probably be an internal error. Signals that we tried to match
    /// two packages with different names.
    #[error("{0:#?}: {1} and {2} do not match.")]
    NameMismatch(DiagnosticCode, String, String),
    #[error("{0:#?}: Tag '{1}' does not exist in registry.")]
    TagNotFound(DiagnosticCode, String),
    /// Error returned from Rogga
    #[error(transparent)]
    RoggaError {
        #[from]
        source: rogga::RoggaError,
    },
    /// Returned if an internal (e.g. io) operation has failed.
    #[error(transparent)]
    InternalError {
        #[from]
        /// The underlying error
        source: InternalError,
    },
}

impl Diagnostic for NodeMaintainerError {
    fn code(&self) -> DiagnosticCode {
        use NodeMaintainerError::*;
        match self {
            NameMismatch(code, ..) => *code,
            TagNotFound(code, ..) => *code,
            RoggaError { source } => source.code(),
            InternalError { .. } => DiagnosticCode::OR1000,
        }
    }
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
pub type Result<T> = std::result::Result<T, NodeMaintainerError>;

pub type InternalResult<T> = std::result::Result<T, InternalError>;
