use oro_diagnostics::{Diagnostic, DiagnosticCategory, Explain};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum NodeMaintainerError {
    /// Should probably be an internal error. Signals that we tried to match
    /// two packages with different names.
    #[error("`{0}` and `{1}` do not match.")]
    NameMismatch(String, String),
    #[error("Tag `{0}` does not exist in registry.")]
    TagNotFound(String),
    #[error("Current directory could not be detected.")]
    NoCwd(#[from] std::io::Error),
    /// Error returned from Rogga
    #[error(transparent)]
    RoggaError(#[from] rogga::RoggaError),
}

impl Explain for NodeMaintainerError {}

impl Diagnostic for NodeMaintainerError {
    fn category(&self) -> DiagnosticCategory {
        use DiagnosticCategory::*;
        use NodeMaintainerError::*;
        match self {
            NameMismatch(_, _) => Misc,
            TagNotFound(_) => Misc,
            NoCwd(_) => Misc,
            RoggaError(source) => source.category(),
        }
    }

    fn label(&self) -> String {
        use NodeMaintainerError::*;
        match self {
            NameMismatch(_, _) => "node_maintainer::name_mismatch".into(),
            TagNotFound(_) => "node_maintainer::tag_not_found".into(),
            NoCwd(_) => "node_maintainer::no_cwd".into(),
            RoggaError(source) => source.label(),
        }
    }

    fn advice(&self) -> Option<String> {
        use NodeMaintainerError::*;
        match self {
            NameMismatch(_, _) => None,
            TagNotFound(_) => None,
            NoCwd(_) => None,
            RoggaError(source) => source.advice(),
        }
    }
}

/// The result type returned by calls to this library
pub type Result<T> = std::result::Result<T, NodeMaintainerError>;
