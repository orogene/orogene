use oro_diagnostics::{Diagnostic, DiagnosticCode};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum PackageSpecError {
    #[error("{code:#?}: Error while parsing `{input}`:\n  {msg}")]
    ParseError {
        code: DiagnosticCode,
        input: String,
        msg: String,
    },
    #[error("{0:#?}: Found invalid characters in identifier: {1}")]
    InvalidCharacters(DiagnosticCode, String),
    #[error("{0:#?}: Drive letters on Windows can only be alphabetical. Got {1}")]
    InvalidDriveLetter(DiagnosticCode, char),
}

impl Diagnostic for PackageSpecError {
    fn code(&self) -> DiagnosticCode {
        use PackageSpecError::*;
        match self {
            ParseError { code, .. } => *code,
            InvalidCharacters(code, ..) => *code,
            InvalidDriveLetter(code, ..) => *code,
        }
    }
}
