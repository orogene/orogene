use std::path::PathBuf;

use miette::Diagnostic;
use node_semver::Version;
use oro_common::CorgiVersionMetadata;
use oro_package_spec::PackageSpec;
use thiserror::Error;

/// Error type returned by all API calls.
#[derive(Error, Debug, Diagnostic)]
pub enum NassunError {
    /// A given package exists, but the version that the specifier resolved to
    /// does not.
    ///
    /// Check that the version or range you're requesting actually exists and
    /// try again.
    #[error("Package for `{0}` was found, but resolved version `{1}` does not exist.")]
    #[diagnostic(
        code(nassun::missing_version),
        url(docsrs),
        help("Try using `oro view` to see what versions are available")
    )]
    MissingVersion(PackageSpec, Version),

    /// Something went wrong while trying to parse a PackageSpec.
    #[error(transparent)]
    #[diagnostic(transparent)]
    PackageSpecError(#[from] oro_package_spec::PackageSpecError),

    /// Failed to read a directory dependency. Refer to the error message for
    /// more details.
    #[error("{0}")]
    #[diagnostic(code(nassun::dir::read), url(docsrs))]
    DirReadError(#[source] std::io::Error, PathBuf),

    /// An io-related error occurred while executing git.
    #[error("Failed to execute git subprocess. {0}")]
    #[diagnostic(code(nassun::git::clone::io), url(docsrs))]
    GitIoError(#[source] std::io::Error),

    /// An error occurred while trying to clone a repository.
    #[error("Failed to clone repository at `{0}`")]
    #[diagnostic(code(nassun::git::clone::repo), url(docsrs))]
    GitCloneError(String),

    /// An error occurred while trying to checkout a repository.
    #[error("Failed to check out `{0}#{1}`")]
    #[diagnostic(code(nassun::git::checkout::repo), url(docsrs))]
    GitCheckoutError(String, String),

    /// Failed to extract a tarball while doing a certain IO operation. Refer
    /// to the error message for more details.
    #[error("Failed to extract tarball while {2}{}", if let Some(path) = .1 {
        format!(" (file: {})", path.to_string_lossy())
    } else {
        "".to_string()
    })]
    #[diagnostic(code(nassun::io::extract), url(docsrs))]
    ExtractIoError(#[source] std::io::Error, Option<PathBuf>, String),

    /// Failed to extract a tarball to the cache. Refer to the error message
    /// for more details.
    #[cfg(not(target_arch = "wasm32"))]
    #[error("Failed to extract tarball to cache. {0}{}", if let Some(path) = .1 {
        format!(" (file: {})", path.to_string_lossy())
    } else {
        "".to_string()
    })]
    #[diagnostic(code(nassun::cache::extract), url(docsrs))]
    ExtractCacheError(#[source] cacache::Error, Option<PathBuf>),

    #[cfg(not(target_arch = "wasm32"))]
    #[error("Missing file index for cache entry for {0}.")]
    #[diagnostic(code(nassun::cache::missing_index), url(docsrs))]
    CacheMissingIndexError(String),

    /// A generic IO error occurred. Refer tot he error message for more
    /// details.
    #[error("{0}")]
    #[diagnostic(code(nassun::io::generic), url(docsrs))]
    IoError(String, #[source] std::io::Error),

    /// A generic oro-client error.
    #[error(transparent)]
    #[diagnostic(transparent)]
    OroClientError(#[from] oro_client::OroClientError),

    /// A generic serde error.
    #[error(transparent)]
    #[diagnostic(code(nassun::serde), url(docsrs))]
    SerdeError(#[from] serde_json::Error),

    /// Failed to parse a URL.
    #[error(transparent)]
    #[diagnostic(code(nassun::bad_url), url(docsrs))]
    UrlError(#[from] url::ParseError),

    /// Failed to parse a package integrity string.
    #[error(transparent)]
    #[diagnostic(code(nassun::integrity_parse_error), url(docsrs))]
    IntegrityError(#[from] ssri::Error),

    /// There's no tarball specified as part of the package metadata for a
    /// given package. This is likely a bug in the registry.
    #[error("Package metadata for {0} is missing a package tarball URL.")]
    #[diagnostic(code(nassun::no_tarball), url(docsrs))]
    NoTarball(String, PackageSpec, Box<CorgiVersionMetadata>),

    /// No matching version could be found for a given specifier. Make sure
    /// that the version, range, or dist-tag you requested actually exists.
    ///
    /// Using `oro view` can help.
    #[error("No matching `{name}` version found for spec `{spec}`.")]
    #[diagnostic(
        code(resolver::no_matching_version),
        url(docsrs),
        // TODO: format help string using variables?
        help("Try using `oro view` to see what versions are available")
    )]
    NoVersion {
        name: String,
        spec: PackageSpec,
        versions: Vec<String>,
    },

    /// Generic serde-wasm-bindgen error.
    #[cfg(target_arch = "wasm32")]
    #[error(transparent)]
    #[diagnostic(code(node_maintainer::serde_wasm_bindgen::error), url(docsrs))]
    SerdeWasmBindgenError(#[from] serde_wasm_bindgen::Error),

    /// Failed to find git in the user's `$PATH`.
    ///
    /// Make sure git is installed and visible from the executing shell's `$PATH`.
    #[cfg(not(target_arch = "wasm32"))]
    #[error(transparent)]
    #[diagnostic(
        code(nassun::which_git_failure),
        url(docsrs),
        help("Are you sure git is installed and available in your $PATH?")
    )]
    WhichGit(#[from] which::Error),

    /// The version resolver ran into an unexpected package spec. This is
    /// almost definitely a bug.
    #[error("Only Version, Tag, Range, and Alias package specs are supported, but got `{0}`.")]
    #[diagnostic(code(nassun::invalid_package_spec), url(docsrs))]
    InvalidPackageSpec(PackageSpec),

    /// Some unsupported operation happened while working with a dummy
    /// package. This is an internal detail and almost definitely a bug worth
    /// reporting.
    #[error("Unsupported dummy package operation: {0}")]
    #[diagnostic(code(nassun::unsupported_dummy_operation), url(docsrs))]
    UnsupportedDummyOperation(String),

    /// A dummy package was missing a name. This is an internal detail and
    /// almost definitely a bug worth reporting.
    #[error("Dummy package does not have a name.")]
    #[diagnostic(code(nassun::dummy_no_name), url(docsrs))]
    DummyNoName,

    /// An error occurred while serializing tarball metadata to cache.
    #[error("Failed to serialize tarball metadata to cache: {0}")]
    #[diagnostic(code(nassun::cache::serialize), url(docsrs))]
    SerializeCacheError(String),

    /// An error happened while deserializing cache metadata.
    #[error("Failed to deserialize cache metadata: {0}")]
    #[diagnostic(code(nassun::cache::deserialize), url(docsrs))]
    DeserializeCacheError(String),

    /// A miscellaneous, usually internal error. This is used mainly to wrap
    /// either manual InternalErrors, or those using external errors that
    /// don't implement std::error::Error.
    ///
    /// If you see this error, please file a bug report so that a better error
    /// can take its place.
    #[error("{0}")]
    #[diagnostic(code(nassun::misc), url(docsrs))]
    MiscError(String),
}

/// The result type returned by calls to this library
pub type Result<T> = std::result::Result<T, NassunError>;

pub trait IoContext {
    type T;

    fn io_context(self, context: impl FnOnce() -> String) -> Result<Self::T>;
}

impl<T> IoContext for std::result::Result<T, std::io::Error> {
    type T = T;

    fn io_context(self, context: impl FnOnce() -> String) -> Result<Self::T> {
        self.map_err(|e| NassunError::IoError(context(), e))
    }
}
