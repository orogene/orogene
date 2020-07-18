use std::path::PathBuf;

use semver::{Version, VersionReq as Range};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum PackageArgError {
    #[error("Failed to parse package arg.")]
    ParseError,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VersionReq {
    Tag(String),
    Version(Version),
    Range(Range),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PackageArg {
    Dir {
        path: PathBuf,
    },
    Alias {
        name: String,
        package: Box<PackageArg>,
    },
    Npm {
        scope: Option<String>,
        name: String,
        requested: Option<VersionReq>,
    },
}
