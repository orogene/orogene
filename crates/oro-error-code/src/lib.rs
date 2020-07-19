use derive_more::Display;
use http_types::StatusCode;

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
}
