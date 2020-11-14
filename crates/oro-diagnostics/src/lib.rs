use std::path::PathBuf;

use url::{Host, Url};

pub trait Diagnostic: std::error::Error + Send + Sync {
    fn category(&self) -> DiagnosticCategory;
    fn subpath(&self) -> String;
    fn advice(&self) -> Option<String>;
    fn diagnostic_path(&self) -> String {
        format!("{}::{}", self.category().prefix(), self.subpath())
    }
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

impl DiagnosticCategory {
    pub fn prefix(&self) -> String {
        use DiagnosticCategory::*;
        match self {
            Misc => "oro::misc",
            Net { .. } => "oro::net",
            Fs { .. } => "oro::fs",
            Parse { .. } => "oro::parse",
        }
        .into()
    }
}
