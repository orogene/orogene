use std::path::PathBuf;

use miette::Diagnostic;
use node_semver::{Version, SemverError};
use oro_common::CorgiVersionMetadata;
use oro_package_spec::PackageSpec;
use thiserror::Error;

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

    /// Could not parse a version or version spec
    #[error(transparent)]
    VersionError(#[from] SemverError),

    /// Something went wrong while trying to parse a PackageArg
    #[error(transparent)]
    PackageSpecError(#[from] oro_package_spec::PackageSpecError),

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

    #[error("Failed to extract tarball while {2}{}", if let Some(path) = .1 {
        format!(" (file: {})", path.to_string_lossy())
    } else {
        "".to_string()
    })]
    #[diagnostic(code(nassun::io::extract))]
    ExtractIoError(#[source] std::io::Error, Option<PathBuf>, String),

    #[error("Failed to extract tarball to cache. {0}{}", if let Some(path) = .1 {
        format!(" (file: {})", path.to_string_lossy())
    } else {
        "".to_string()
    })]
    #[diagnostic(code(nassun::cache::extract))]
    ExtractCacheError(#[source] cacache::Error, Option<PathBuf>),

    #[error(transparent)]
    #[diagnostic(code(nassun::io::generic))]
    IoError(#[from] std::io::Error),

    #[error(transparent)]
    #[diagnostic(transparent)]
    OroClientError(#[from] oro_client::OroClientError),

    #[error(transparent)]
    #[diagnostic(code(nassun::serde))]
    SerdeError(#[from] serde_json::Error),

    #[error(transparent)]
    #[diagnostic(code(nassun::bad_url))]
    UrlError(#[from] url::ParseError),

    #[error(transparent)]
    #[diagnostic(code(nassun::integrity_parse_error))]
    IntegrityError(#[from] ssri::Error),

    #[error("Package metadata for {0} is missing a package tarball URL.")]
    #[diagnostic(code(nassun::no_tarball))]
    NoTarball(String, PackageSpec, Box<CorgiVersionMetadata>),

    #[error("No matching `{name}` version found for spec `{spec}`.")]
    #[diagnostic(
        code(resolver::no_matching_version),
        // TODO: format help string using variables?
        help("Try using `oro view` to see what versions are available")
    )]
    NoVersion {
        name: String,
        spec: PackageSpec,
        versions: Vec<String>,
    },

    #[cfg(not(target_arch = "wasm32"))]
    #[error(transparent)]
    #[diagnostic(
        code(nassun::which_git_failure),
        help("Are you sure git is installed and available in your $PATH?")
    )]
    WhichGit(#[from] which::Error),

    #[error("Only Version, Tag, Range, and Alias package args are supported, but got `{0}`.")]
    #[diagnostic(code(nassun::invalid_package_spec))]
    InvalidPackageSpec(PackageSpec),

    #[error("Unsupported dummy package operation: {0}")]
    #[diagnostic(code(nassun::unsupported_dummy_operation))]
    UnsupportedDummyOperation(String),

    #[error("Dummy package does not have a name.")]
    #[diagnostic(code(nassun::dummy_no_name))]
    DummyNoName,

    /// A miscellaneous, usually internal error. This is used mainly to wrap
    /// either manual InternalErrors, or those using external errors that
    /// don't implement std::error::Error.
    #[error("{0}")]
    #[diagnostic(code(nassun::misc))]
    MiscError(String),
}

/// The result type returned by calls to this library
pub type Result<T> = std::result::Result<T, NassunError>;
