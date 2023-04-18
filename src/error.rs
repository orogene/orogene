use miette::Diagnostic;
use thiserror::Error;

#[derive(Debug, Error, Diagnostic)]
pub enum OroError {
    /// Invalid package name. Only package names should be passed to `oro
    /// remove`, but you passed either a package specifier or an invalid
    /// package name.
    ///
    /// Try passing the package name as it appears in your package.json.
    #[error("{0} is not a valid package name. Only package names should be passed to `oro remove`, but you passed either a non-NPM package specifier or an invalid package name.")]
    #[diagnostic(
        code(oro::remove::invalid_package_name),
        url(docsrs),
        help("Use the package name as it appears in your package.json instead.")
    )]
    InvalidPackageName(String),
}
