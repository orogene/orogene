use oro_error_code::OroErrCode;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum PackageSpecError {
    #[error("{0}")]
    ParseError(OroErrCode),
    #[error("Found invalid characters in identifier: {0}")]
    InvalidCharacters(String),
    #[error("Drive letters on Windows can only be alphabetical. Got {0}")]
    InvalidDriveLetter(char),
}
