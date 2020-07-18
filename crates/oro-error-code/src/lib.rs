use derive_more::Display;

/// Contextual error codes for a variety of `orogene` error messages. These
/// codes have an M:N relationship to actual errors and are intended to
/// provide users with additional context that they can easily look up in the
/// `orogene` documentation.
#[derive(Display)]
pub enum OroErrCode {
    /// Failed to parse a package arg for some reason. The message includes
    /// the actual error.
    #[display(fmt = "OR1000: Package arg `{}` failed to parse:\n{}", input, msg)]
    OR1000 { input: String, msg: String },
}
