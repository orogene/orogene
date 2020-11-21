use std::path::PathBuf;

use oro_diagnostics::{Diagnostic, DiagnosticCategory, Explain, Meta};
use oro_diagnostics_derive::Diagnostic;
use oro_node_semver::Version;
use oro_package_spec::PackageSpec;
use thiserror::Error;

use crate::resolver::ResolverError;

/// Error type returned by all API calls.
#[derive(Error, Debug, Diagnostic)]
pub enum RoggaError {
    /// Something went wrong while fetching a package.
    #[error("Package for `{0}` was found, but resolved version `{1}` does not exist.")]
    #[category(Misc)]
    #[label("rogga::missing_version")]
    #[advice("Try using `oro view` to see what versions are available")]
    MissingVersion(PackageSpec, Version),

    /// Something went wrong while trying to parse a PackageArg
    #[error(transparent)]
    PackageSpecError(
        #[from]
        #[ask]
        oro_package_spec::PackageSpecError,
    ),

    #[error(transparent)]
    ResolverError(
        #[from]
        #[ask]
        ResolverError,
    ),

    #[error("{0}")]
    #[label("rogga::dir::read")]
    DirReadError(#[source] std::io::Error, PathBuf),

    #[error("Failed to execute git subprocess. {0}")]
    #[label("rogga::git::clone::io")]
    GitIoError(#[source] std::io::Error),

    #[error("Failed to clone repository at `{0}`")]
    #[label("rogga::git::clone::repo")]
    GitCloneError(String),

    #[error("Failed to check out `{0}#{1}`")]
    #[label("rogga::git::checkout::repo")]
    GitCheckoutError(String, String),

    #[error("Failed to extract tarball to disk. {0}")]
    #[label("rogga::io::extract")]
    ExtractIoError(#[source] std::io::Error, Option<PathBuf>),

    #[error(transparent)]
    OroClientError(
        #[from]
        #[ask]
        oro_client::OroClientError,
    ),

    #[error(transparent)]
    #[label("rogga::serde")]
    SerdeError(#[from] serde_json::Error),

    #[error(transparent)]
    #[label("rogga::bad_url")]
    UrlError(#[from] url::ParseError),

    #[error(transparent)]
    #[label("rogga::which_git_failure")]
    #[advice("Are you sure git is installed and available in your $PATH?")]
    WhichGit(#[from] which::Error),

    /// A miscellaneous, usually internal error. This is used mainly to wrap
    /// either manual InternalErrors, or those using external errors that
    /// don't implement std::error::Error.
    #[error("A miscellaneous error occurred: {0}")]
    #[label("rogga::misc")]
    #[category(Misc)]
    MiscError(String),
}

impl Explain for RoggaError {
    fn meta(&self) -> Option<Meta> {
        use RoggaError::*;
        match self {
            DirReadError(_, ref path) => Some(Meta::Fs { path: path.clone() }),
            ExtractIoError(_, Some(path)) => Some(Meta::Fs { path: path.clone() }),
            _ => None,
        }
    }
}

/// The result type returned by calls to this library
pub type Result<T> = std::result::Result<T, RoggaError>;
