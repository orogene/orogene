use std::path::PathBuf;

use miette::Diagnostic;
use node_semver::Version;
use oro_package_spec::PackageSpec;
use thiserror::Error;

use crate::resolver::ResolverError;

/// Error type returned by all API calls.
#[derive(Error, Debug, Diagnostic)]
pub enum NassunError {
    /// Something went wrong while fetching a package.
    #[error("Package for `{0}` was found, but resolved version `{1}` does not exist.")]
    #[diagnostic(
        code(nassun::missing_version),
        help("Try using `oro view` to see what versions are available")
    )]
    MissingVersion(PackageSpec, Version),

    /// Something went wrong while trying to parse a PackageArg
    #[error(transparent)]
    PackageSpecError(#[from] oro_package_spec::PackageSpecError),

    #[error(transparent)]
    ResolverError(#[from] ResolverError),

    #[error("{0}")]
    #[diagnostic(code(nassun::dir::read))]
    DirReadError(#[source] std::io::Error, PathBuf),

    #[error("Failed to execute git subprocess. {0}")]
    #[diagnostic(code(nassun::git::clone::io))]
    GitIoError(#[source] std::io::Error),

    #[error("Failed to clone repository at `{0}`")]
    #[diagnostic(code(nassun::git::clone::repo))]
    GitCloneError(String),

    #[error("Failed to check out `{0}#{1}`")]
    #[diagnostic(code(nassun::git::checkout::repo))]
    GitCheckoutError(String, String),

    #[error("Failed to extract tarball. {0}")]
    #[diagnostic(code(nassun::io::extract))]
    ExtractIoError(#[source] std::io::Error, Option<PathBuf>),

    #[error(transparent)]
    #[diagnostic(code(nassun::io::generic))]
    IoError(#[from] std::io::Error),

    #[error(transparent)]
    OroClientError(#[from] oro_client::OroClientError),

    #[error(transparent)]
    #[diagnostic(code(nassun::serde))]
    SerdeError(#[from] serde_json::Error),

    #[error(transparent)]
    #[diagnostic(code(nassun::bad_url))]
    UrlError(#[from] url::ParseError),

    #[cfg(feature = "git")]
    #[error(transparent)]
    #[diagnostic(
        code(nassun::which_git_failure),
        help("Are you sure git is installed and available in your $PATH?")
    )]
    WhichGit(#[from] which::Error),

    /// A miscellaneous, usually internal error. This is used mainly to wrap
    /// either manual InternalErrors, or those using external errors that
    /// don't implement std::error::Error.
    #[error("{0}")]
    #[diagnostic(code(nassun::misc))]
    MiscError(String),
}

/// The result type returned by calls to this library
pub type Result<T> = std::result::Result<T, NassunError>;