pub trait Diagnostic: std::error::Error {
    fn code(&self) -> &DiagnosticCode;
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
#[non_exhaustive]
pub enum DiagnosticCode {
    /// Failed to parse a package spec.
    OR1001,
    /// Package spec contains invalid characters.
    OR1002,
    /// Package spec contains invalid drive letter.
    OR1003,
}
