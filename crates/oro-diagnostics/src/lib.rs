use std::fmt;
use std::path::PathBuf;

use colored::Colorize;
use thiserror::Error;
use url::{Host, Url};

#[derive(Error)]
#[error("{:?}", self)]
pub struct DiagnosticError {
    pub error: Box<dyn std::error::Error + Send + Sync>,
    pub category: DiagnosticCategory,
    pub label: String,
    pub advice: Option<String>,
}

impl fmt::Debug for DiagnosticError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if f.alternate() {
            return fmt::Debug::fmt(&self.error, f);
        } else {
            use DiagnosticCategory::*;
            write!(f, "{}", self.label.red())?;
            if let Net { ref host, ref url } = &self.category {
                if let Some(url) = url {
                    write!(f, " @ {}", format!("{}", url).cyan().underline())?;
                } else {
                    write!(f, " @ {}", format!("{}", host).cyan().underline())?;
                }
            }
            write!(f, "\n\n")?;
            write!(f, "{}", self.error)?;
            if let Some(advice) = &self.advice {
                write!(f, "\n\n{}", "help".yellow())?;
                write!(f, ": {}", advice)?;
            }
        }
        Ok(())
    }
}

pub type DiagnosticResult<T> = Result<T, DiagnosticError>;

impl<E> From<E> for DiagnosticError
where
    E: Diagnostic + Send + Sync,
{
    fn from(error: E) -> Self {
        Self {
            category: error.category(),
            label: error.label(),
            advice: error.advice(),
            error: Box::new(error),
        }
    }
}

pub trait Diagnostic: std::error::Error + Send + Sync + 'static {
    fn category(&self) -> DiagnosticCategory;
    fn label(&self) -> String;
    fn advice(&self) -> Option<String>;
}

// This is needed so Box<dyn Diagnostic> is correctly treated as an Error.
impl std::error::Error for Box<dyn Diagnostic> {}

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub enum DiagnosticCategory {
    /// oro::misc
    Misc,
    /// oro::net
    Net { host: Host, url: Option<Url> },
    /// oro::fs
    Fs { path: PathBuf },
    /// oro::parse
    Parse {
        input: String,
        row: usize,
        col: usize,
        path: Option<PathBuf>,
    },
}

pub trait AsDiagnostic<T, E> {
    fn as_diagnostic(self, subpath: impl AsRef<str>) -> std::result::Result<T, DiagnosticError>;
}

impl<T, E: std::error::Error + Send + Sync + 'static> AsDiagnostic<T, E> for Result<T, E> {
    fn as_diagnostic(self, label: impl AsRef<str>) -> Result<T, DiagnosticError> {
        self.map_err(|e| DiagnosticError {
            category: DiagnosticCategory::Misc,
            error: Box::new(e),
            label: label.as_ref().into(),
            advice: None,
        })
    }
}
