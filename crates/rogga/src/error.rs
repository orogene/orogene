use std::path::PathBuf;

use oro_diagnostics::{Diagnostic, DiagnosticCategory};
use oro_node_semver::Version;
use oro_package_spec::PackageSpec;
use thiserror::Error;

use crate::resolver::ResolverError;

/// Error type returned by all API calls.
#[derive(Error, Debug)]
pub enum RoggaError {
    /// Something went wrong while fetching a package.
    #[error("Package for `{0}` was found, but resolved version `{1}` does not exist.")]
    MissingVersion(PackageSpec, Version),

    /// Something went wrong while trying to parse a PackageArg
    #[error(transparent)]
    PackageSpecError(#[from] oro_package_spec::PackageSpecError),

    #[error(transparent)]
    ResolverError(#[from] ResolverError),

    #[error("{0}")]
    DirReadError(#[source] std::io::Error, PathBuf),

    #[error("Failed to execute git subprocess. {0}")]
    GitIoError(#[source] std::io::Error),

    #[error("Failed to clone repository at `{0}`")]
    GitCloneError(String),

    #[error("Failed to check out `{0}#{1}`")]
    GitCheckoutError(String, String),

    #[error("Failed to extract tarball to disk. {0}")]
    ExtractIoError(#[source] std::io::Error, Option<PathBuf>),

    #[error(transparent)]
    OroClientError(#[from] oro_client::OroClientError),

    #[error(transparent)]
    SerdeError(#[from] serde_json::Error),

    #[error(transparent)]
    UrlError(#[from] url::ParseError),

    #[error(transparent)]
    WhichGit(#[from] which::Error),

    /// A miscellaneous, usually internal error. This is used mainly to wrap
    /// either manual InternalErrors, or those using external errors that
    /// don't implement std::error::Error.
    #[error("A miscellaneous error occurred: {0}")]
    MiscError(String),
}

impl Diagnostic for RoggaError {
    fn category(&self) -> DiagnosticCategory {
        use DiagnosticCategory::*;
        use RoggaError::*;
        match self {
            MissingVersion(..) => Misc,
            PackageSpecError(err) => err.category(),
            ResolverError(err) => err.category(),
            DirReadError(_, ref path) => Fs { path: path.clone() },
            GitIoError(_) => Misc,
            GitCloneError(_) => Misc,
            GitCheckoutError(..) => Misc,
            ExtractIoError(_, None) => Misc,
            ExtractIoError(_, Some(path)) => Fs { path: path.clone() },
            OroClientError(err) => err.category(),
            SerdeError(_) => Misc,
            UrlError(_) => Misc,
            WhichGit(_) => Misc,
            MiscError(_) => Misc,
        }
    }

    fn label(&self) -> String {
        use RoggaError::*;
        match self {
            MissingVersion(..) => "rogga::missing_version".into(),
            PackageSpecError(err) => err.label(),
            ResolverError(err) => err.label(),
            DirReadError(_, _) => "rogga::dir::read".into(),
            GitIoError(..) => "rogga::git::clone::io".into(),
            GitCloneError(..) => "rogga::git::clone::repo".into(),
            GitCheckoutError(..) => "rogga::git::checkout::repo".into(),
            ExtractIoError(_, _) => "rogga::io::extract".into(),
            OroClientError(err) => err.label(),
            SerdeError(_) => "rogga::serde".into(),
            UrlError(_) => "rogga::bad_url".into(),
            WhichGit(..) => "rogga::which_git_failure".into(),
            MiscError(_) => "rogga::misc".into(),
        }
    }

    fn advice(&self) -> Option<String> {
        use RoggaError::*;
        match self {
            MissingVersion(..) => {
                Some("Try using `oro view` to see what versions are available".into())
            }
            PackageSpecError(err) => err.advice(),
            ResolverError(err) => err.advice(),
            DirReadError(..) => None,
            GitIoError(..) => None,
            GitCloneError(..) => None,
            GitCheckoutError(..) => None,
            ExtractIoError(..) => None,
            OroClientError(err) => err.advice(),
            SerdeError(..) => None,
            UrlError(..) => None,
            WhichGit(..) => {
                Some("Are you sure git is installed and available in your $PATH?".into())
            }
            MiscError(..) => None,
        }
    }
}

/// The result type returned by calls to this library
pub type Result<T> = std::result::Result<T, RoggaError>;
