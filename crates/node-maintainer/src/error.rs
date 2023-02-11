use crate::maintainer::NodeDependency;
use futures::channel::mpsc;
use kdl::{KdlDocument, KdlNode};
use miette::Diagnostic;
use thiserror::Error;

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

    /// Failed to parse an integrity value.
    #[error(transparent)]
    #[diagnostic(code(node_maintainer::kdl::integrity_parse_error))]
    IntegrityParseError(#[from] ssri::Error),

    /// Missing package node name.
    #[error("Missing package node name.")]
    #[diagnostic(code(node_maintainer::kdl::missing_node_name))]
    MissingName(KdlNode),

    /// No root node in KDL lockfile.
    #[error("No root node in KDL lockfile.")]
    #[diagnostic(code(node_maintainer::kdl::missing_root))]
    MissingRoot(KdlDocument),

    /// Error parsing lockfile.
    #[error(transparent)]
    #[diagnostic(code(node_maintainer::kdl::parse_error))]
    KdlParseError(#[from] kdl::KdlError),

    #[error("Invalid lockfile version format.")]
    #[diagnostic(code(node_maintainer::kdl::invalid_lockfile_version))]
    InvalidLockfileVersion,

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

    // Generic error
    #[error("{0}")]
    #[diagnostic(code(node_maintainer::miscellaneous_error))]
    MiscError(String),

    #[error("Failed to send data on idx mpsc channel.")]
    #[diagnostic(code(node_maintainer::kdl::io_error))]
    TryIdxSendError(#[from] mpsc::TrySendError<()>),

    #[error("Failed to send data on dep mpsc channel.")]
    #[diagnostic(code(node_maintainer::kdl::io_error))]
    TryDepSendError(#[from] mpsc::TrySendError<NodeDependency>),
}
