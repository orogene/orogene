use futures::channel::mpsc;
use kdl::{KdlDocument, KdlNode};
use miette::Diagnostic;
use thiserror::Error;

use crate::{NpmPackageLock, NpmPackageLockEntry};

#[allow(clippy::large_enum_variant)]
#[derive(Debug, Error, Diagnostic)]
pub enum NodeMaintainerError {
    /// Unsupported resolved URL scheme
    #[error("Unsupported resolved URL scheme")]
    #[diagnostic(code(node_maintainer::kdl::unsupported_url_scheme))]
    UnsupportedScheme(String),

    /// Failed to parse a resolved URL while parsing lockfile
    #[error("Failed to parse a resolved URL while parsing lockfile: {0}")]
    #[diagnostic(code(node_maintainer::kdl::url_parse_error))]
    UrlParseError(String, #[source] url::ParseError),

    /// Failed to parse a Semver string.
    #[error("Failed to parse a Semver string.")]
    #[diagnostic(code(node_maintainer::kdl::semver_parse_error))]
    SemverParseError(#[from] node_semver::SemverError),

    /// Missing version for NPM package entry in lockfile.
    #[error("Missing version for NPM package entry in lockfile.")]
    #[diagnostic(code(node_maintainer::kdl::missing_version))]
    MissingVersion,

    /// Missing resolution for package entry in lockfile.
    #[error("Missing version for NPM package entry in lockfile.")]
    #[diagnostic(code(node_maintainer::kdl::missing_version))]
    MissingResolution,

    /// Failed to parse an integrity value.
    #[error(transparent)]
    #[diagnostic(code(node_maintainer::kdl::integrity_parse_error))]
    IntegrityParseError(#[from] ssri::Error),

    /// Failed to parse an integrity value while loading lockfile.
    #[error("Failed to parse an integrity value while loading lockfile node:\n{0}")]
    #[diagnostic(code(node_maintainer::kdl::integrity_parse_error))]
    KdlLockfileIntegrityParseError(KdlNode, #[source] ssri::Error),

    /// Missing package node name.
    #[error("Missing package node name:\n{0}")]
    #[diagnostic(code(node_maintainer::kdl::missing_node_name))]
    KdlLockMissingName(KdlNode),

    /// Missing package node name.
    #[error("Missing package name:\n{0:#?}")]
    #[diagnostic(code(node_maintainer::npm::missing_name))]
    NpmLockMissingName(Box<NpmPackageLockEntry>),

    /// Failed to parse an integrity value while loading NPM lockfile.
    #[error("Failed to parse an integrity value while loading lockfile node:\n{0:#?}")]
    #[diagnostic(code(node_maintainer::npm::integrity_parse_error))]
    NpmLockfileIntegrityParseError(Box<NpmPackageLockEntry>, #[source] ssri::Error),

    /// Unsupported NPM Package Lock version.
    #[error("Unsupported NPM Package Lock version: {0}")]
    #[diagnostic(code(node_maintainer::npm::unsupported_package_lock_Version))]
    NpmUnsupportedPackageLockVersion(u64),

    /// No root node in KDL lockfile.
    #[error("No root node in KDL lockfile.")]
    #[diagnostic(code(node_maintainer::kdl::missing_root))]
    KdlLockMissingRoot(KdlDocument),

    /// No root node in NPM lockfile.
    #[error("No root package in NPM lockfile.")]
    #[diagnostic(code(node_maintainer::npm::missing_root))]
    NpmLockMissingRoot(NpmPackageLock),

    /// Error parsing lockfile.
    #[error(transparent)]
    #[diagnostic(code(node_maintainer::kdl::parse_error))]
    KdlParseError(#[from] kdl::KdlError),

    #[error("Invalid lockfile version format.")]
    #[diagnostic(code(node_maintainer::kdl::invalid_lockfile_version))]
    InvalidLockfileVersion,

    /// Error from serde_wasm_bindgen
    #[cfg(target_arch = "wasm32")]
    #[error(transparent)]
    #[diagnostic(code(node_maintainer::serde_wasm_bindgen::error))]
    SerdeWasmBindgenError(#[from] serde_wasm_bindgen::Error),

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
    NassunError(#[from] nassun::error::NassunError),

    /// Generic serde_json error.
    #[error(transparent)]
    #[diagnostic(code(node_maintainer::serde_json_error))]
    SerdeJsonError(#[from] serde_json::Error),

    /// Generic error
    #[error("{0}")]
    #[diagnostic(code(node_maintainer::miscellaneous_error))]
    MiscError(String),

    #[error("Failed to send data through mpsc channel.")]
    #[diagnostic(code(node_maintainer::mpsc_error))]
    TrySendError,

    #[error("{0}")]
    #[diagnostic(code(node_maintainer::graph_error))]
    GraphValidationError(String),

    #[cfg(not(target_arch = "wasm32"))]
    #[error(transparent)]
    #[diagnostic(code(node_maintainer::walkdir_error))]
    WalkDirError(#[from] walkdir::Error),

    #[cfg(not(target_arch = "wasm32"))]
    #[error("Failed to read manifest during build step, at {}", .0.display())]
    #[diagnostic(code(node_maintainer::build_manifest_read_error))]
    BuildManifestReadError(std::path::PathBuf, #[source] std::io::Error),

    #[cfg(not(target_arch = "wasm32"))]
    #[error(transparent)]
    #[diagnostic(transparent)]
    OroScriptError(#[from] oro_script::OroScriptError),

    /// Locked file was requested, but a new dependency tree was resolved that
    /// would cause changes to the lockfile. The contents of `package.json`
    /// may have changed since the last time the lockfile was updated.
    ///
    /// This typically happens when a dependency is added or removed from
    /// package.json while locked mode is enabled. If you have an existing
    /// lockfile and get this error without any modifications to package.json,
    /// please [report this as a
    /// bug](https://github.com/orogene/orogene/issues/new).
    #[error("Locked file was requested, but a new dependency tree was resolved that would cause changes to the lockfile. The contents of `package.json` may have changed since the last time the lockfile was updated.")]
    #[diagnostic(
        code(node_maintainer::lockfile_mismatch),
        url(docsrs),
        help("Did you modify package.json by hand?")
    )]
    LockfileMismatch,
}

impl<T> From<mpsc::TrySendError<T>> for NodeMaintainerError {
    fn from(_: mpsc::TrySendError<T>) -> Self {
        NodeMaintainerError::TrySendError
    }
}
