use std::path::PathBuf;

use oro_error_code::OroErrCode;
use semver::{Version, VersionReq as Range};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum PackageArgError {
    #[error("{0}")]
    ParseError(OroErrCode),
    #[error("Found invalid characters in identifier: {0}")]
    InvalidCharacters(String),
    #[error("Drive letters on Windows can only be alphabetical. Got {0}")]
    InvalidDriveLetter(char),
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
