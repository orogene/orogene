use oro_diagnostics::Diagnostic;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum PackageSpecError {
    #[error("Error while parsing `{input}`:\n  {msg}")]
    ParseError { input: String, msg: String },
    #[error("Found invalid characters in identifier: {0}")]
    InvalidCharacters(String),
    #[error("Drive letters on Windows can only be alphabetical. Got {0}")]
    InvalidDriveLetter(char),
    #[error("Invalid git host `{0}`. Only GitHub, GitLab, Gist, and Bitbucket are supported.")]
    InvalidGitHost(String),
}

impl Diagnostic for PackageSpecError {
    fn category(&self) -> oro_diagnostics::DiagnosticCategory {
        todo!()
    }

    fn subpath(&self) -> String {
        todo!()
    }

    fn advice(&self) -> Option<String> {
        todo!()
    }
}
