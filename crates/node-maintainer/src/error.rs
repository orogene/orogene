use oro_diagnostics::{Diagnostic, DiagnosticCategory, Explain};
use thiserror::Error;

#[derive(Debug, Error, Diagnostic)]
pub enum NodeMaintainerError {
    /// Should probably be an internal error. Signals that we tried to match
    /// two packages with different names.
    #[error("`{0}` and `{1}` do not match.")]
    #[label("node_maintainer::name_mismatch")]
    NameMismatch(String, String),

    #[error("Tag `{0}` does not exist in registry.")]
    #[label("node_maintainer::tag_not_found")]
    TagNotFound(String),

    #[error("Current directory could not be detected.")]
    #[label("node_maintainer::no_cwd")]
    NoCwd(#[from] std::io::Error),

    /// Error returned from Rogga
    #[error(transparent)]
    RoggaError(
        #[from]
        #[ask]
        rogga::RoggaError,
    ),
}

impl Explain for NodeMaintainerError {}

/// The result type returned by calls to this library
pub type Result<T> = std::result::Result<T, NodeMaintainerError>;
