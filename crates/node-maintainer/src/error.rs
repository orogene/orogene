use miette::Diagnostic;
use thiserror::Error;

#[allow(clippy::large_enum_variant)]
#[derive(Debug, Error, Diagnostic)]
pub enum NodeMaintainerError {
    /// Generic package spec error.
    #[error(transparent)]
    #[diagnostic(transparent)]
    PackageSpecError(#[from] oro_package_spec::PackageSpecError),

    /// Generic IO Error.
    #[error(transparent)]
    #[diagnostic(code(node_maintainer::io_error))]
    IoError(#[from] std::io::Error),

    /// Generic error returned from Nassun.
    #[error(transparent)]
    #[diagnostic(transparent)]
    NassunError(#[from] nassun::NassunError),
}
