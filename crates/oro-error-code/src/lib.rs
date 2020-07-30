use derive_more::Display;
use http_types::StatusCode;
use std::path::PathBuf;

/// Contextual error codes for a variety of `orogene` error messages. These
/// codes have an M:N relationship to actual errors and are intended to
/// provide users with additional context that they can easily look up in the
/// `orogene` documentation.
#[derive(Debug, Display)]
pub enum OroErrCode {
    /// Failed to parse a package arg for some reason. The message includes
    /// the actual error.
    #[display(fmt = "OR1000: Package arg `{}` failed to parse:\n{}", input, msg)]
    OR1000 { input: String, msg: String },

    /// Didn't get any response from a registry during ping.
    #[display(fmt = "OR1001: No pong response from registry at {}", _0)]
    OR1001(String),

    /// Got a response but failed to get a response body during ping.
    #[display(fmt = "OR1002: Failed to get response body during.")]
    OR1002,

    /// Failed to ping
    #[display(
        fmt = "OR1003: Error response from registry {}: {} {}",
        registry,
        status,
        message
    )]
    OR1003 {
        registry: String,
        status: StatusCode,
        message: String,
    },

    /// Failed to parse response body in ping response
    #[display(fmt = "OR1004: Failed to parse response body")]
    OR1004,

    /// Failed to parse registry URL given to ping
    #[display(fmt = "OR1005: Failed to parse registry URL from `{}`", _0)]
    OR1005(String),

    /// This error occurs due to a failure to get the current executable (see
    /// [std::env::current_exe](https://doc.rust-lang.org/1.39.0/std/env/fn.current_exe.html#)),
    /// and can be for any number of system-related reasons beyond the control
    /// of `oro`.
    #[display(fmt = "OR1006: Failed to get the location of the current ds binary")]
    OR1006,

    /// `oro shell` tried to execute a given Node.js binary, but the operation
    /// failed for some reason. Is Node installed and available in your $PATH?
    /// Did you pass in an invalid `--node` argument? Are you sure the file is
    /// executable?
    #[display(fmt = "OR1007: Failed to execute node binary at `{}`", _0)]
    OR1007(String),

    #[display(fmt = "OR1008: A home directory is required for oro patch scripts.")]
    OR1008,

    #[display(fmt = "OR1009: Failed to write oro data file at `{:?}`", _0)]
    OR1009(PathBuf),

    #[display(fmt = "OR1010: Failed to create data directory at `{:?}`", _0)]
    OR1010(PathBuf),
}
