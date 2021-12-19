use std::path::PathBuf;

use oro_common::thiserror::{self, Error};

/// Error type returned by all API calls.
#[derive(Error, Debug)]
pub enum ManifestError {
    #[error("Failed to parse person string `{input}`: {msg}")]
    ParsePersonError { input: String, msg: String },

    #[error("Invalid package file {0}. Package files should be JSON objects")]
    InvalidPackageFile(PathBuf),

    #[error(transparent)]
    IoError(#[from] std::io::Error),

    #[error(transparent)]
    JsonError(#[from] serde_json::Error),
}
