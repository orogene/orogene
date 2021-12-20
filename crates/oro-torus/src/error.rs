use std::path::PathBuf;

use oro_common::{
    http,
    miette::{self, Diagnostic},
    node_semver::Version,
    serde_json,
    thiserror::{self, Error},
};
use oro_package_spec::PackageSpec;

use crate::resolver::ResolverError;

/// Error type returned by all API calls.
#[derive(Error, Debug, Diagnostic)]
pub enum TorusError {
    /// Something went wrong while fetching a package.
    #[error("Package for `{0}` was found, but resolved version `{1}` does not exist.")]
    #[diagnostic(code(oro_torus::missing_version))]
    MissingVersion(PackageSpec, Version),

    /// Something went wrong while trying to parse a PackageArg
    #[error(transparent)]
    PackageSpecError(#[from] oro_package_spec::PackageSpecError),

    #[error(transparent)]
    ResolverError(#[from] ResolverError),

    #[error("{0}")]
    #[diagnostic(code(oro_torus::dir::read))]
    DirReadError(#[source] std::io::Error, PathBuf),

    #[error("Failed to execute git subprocess. {0}")]
    #[diagnostic(code(oro_torus::git::clone::io))]
    GitIoError(#[source] std::io::Error),

    #[error("Failed to clone repository at `{0}`")]
    #[diagnostic(code(oro_torus::git::clone::repo))]
    GitCloneError(String),

    #[error("Failed to check out `{0}#{1}`")]
    #[diagnostic(code(oro_torus::git::checkout::repo))]
    GitCheckoutError(String, String),

    #[error("Failed to extract tarball to disk. {0}")]
    #[diagnostic(code(oro_torus::io::extract))]
    ExtractIoError(#[source] std::io::Error, Option<PathBuf>),

    #[error(transparent)]
    #[diagnostic(code(oro_torus::serde))]
    SerdeError(#[from] serde_json::Error),

    #[error(transparent)]
    #[diagnostic(code(oro_torus::bad_url))]
    UrlError(#[from] url::ParseError),

    #[error(transparent)]
    #[diagnostic(
        code(oro_torus::which_git_failure),
        help("Are you sure git is installed and available in your $PATH?")
    )]
    WhichGit(#[from] which::Error),

    #[error(transparent)]
    #[diagnostic(code(oro_torus::client_error))]
    ClientError(#[from] oro_api_client::Error),

    #[error(transparent)]
    #[diagnostic(code(oro_torus::http_error))]
    HttpError(#[from] http::Error),

    /// A miscellaneous, usually internal error. This is used mainly to wrap
    /// either manual InternalErrors, or those using external errors that
    /// don't implement std::error::Error.
    #[error("A miscellaneous error occurred: {0}")]
    #[diagnostic(code(oro_torus::misc))]
    MiscError(String),
}
