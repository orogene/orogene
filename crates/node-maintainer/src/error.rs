use miette::Diagnostic;
use thiserror::Error;

#[derive(Debug, Error, Diagnostic)]
pub enum NodeMaintainerError {
    /// Should probably be an internal error. Signals that we tried to match
    /// two packages with different names.
    #[error("`{0}` and `{1}` do not match.")]
    #[diagnostic(code(node_maintainer::name_mismatch))]
    NameMismatch(String, String),

    #[error("Tag `{0}` does not exist in registry.")]
    #[diagnostic(code(node_maintainer::tag_not_found))]
    TagNotFound(String),

    #[error("Current directory could not be detected.")]
    #[diagnostic(code(node_maintainer::no_cwd))]
    NoCwd(#[from] std::io::Error),

    /// Error returned from Nassun
    #[error(transparent)]
    NassunError(#[from] nassun::NassunError),
}
