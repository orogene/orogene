use miette::Diagnostic;
use thiserror::Error;

#[derive(Debug, Error, Diagnostic)]
pub enum OroPackageJsonError {
    /// An error was thrown in `walkdir`.
    #[error(transparent)]
    #[diagnostic(code(oro_package_json::io_error), url(docsrs))]
    WalkError(#[from] walkdir::Error),

    #[error(transparent)]
    #[diagnostic(code(oro_package_json::io_error), url(docsrs))]
    IoError(#[from] std::io::Error),
}
