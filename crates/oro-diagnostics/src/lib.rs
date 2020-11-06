pub trait Diagnostic: std::error::Error + Send + Sync {
    fn code(&self) -> DiagnosticCode;
}

impl std::error::Error for Box<dyn Diagnostic> {}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
#[non_exhaustive]
pub enum DiagnosticCode {
    /// An internal error has occurred. Please refer to the error message for
    /// more details.
    OR1000,
    /// Failed to parse a package spec.
    OR1001,
    /// Package spec contains invalid characters.
    OR1002,
    /// Package spec contains invalid drive letter.
    OR1003,
    /// Resolver name mismatch.
    OR1004,
    /// dist-tag not found.
    OR1005,
    /// An error occurred deserializing package metadata.
    OR1006,
    /// Tried to resolve an unsupported package type.
    OR1007,
    /// No compatible version was found while resolving a package request.
    OR1008,
    /// Package metadata contains no versions.
    OR1009,
    /// Failure parsing Semver VersionReq.
    OR1010,
    /// Semver version string was too long.
    OR1011,
    /// Failure parsing Semver Version.
    OR1012,
    /// Error parsing digit. This is probably an issue with the Semver parser itself.
    OR1013,
    /// Semver number component is larger than the allowed limit (JavaScript's Number.MAX_SAFE_INTEGER).
    OR1014,
}
