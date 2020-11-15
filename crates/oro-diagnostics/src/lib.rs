use std::path::PathBuf;

use url::{Host, Url};

pub trait Diagnostic: std::error::Error + Send + Sync {
    fn category(&self) -> DiagnosticCategory;
    fn subpath(&self) -> String;
    fn advice(&self) -> Option<String>;
    fn diagnostic_path(&self) -> String {
        format!("{}::{}", self.category().prefix(), self.subpath())
    }
    fn pretty_print(&self) -> String {
        use DiagnosticCategory::*;
        let mut output = String::new();
        output.push_str("Code: ");
        output.push_str(&self.diagnostic_path()[..]);
        output.push_str("\n\n");
        output.push_str(&self.to_string()[..]);
        output.push_str(&match self.category() {
            Misc => "".into(),
            Net { ref host, ref url  } => {
                if let Some(url) = url {
                    format!("\n\nurl: {}", url)
                } else {
                    format!("\n\nhost: {}", host)
                }
            },
            Fs { .. } => "something happened with the filesystem".into(),
            Parse { .. } => "something happened while parsing".into(),
        }[..]);
        if let Some(advice) = self.advice() {
            output.push_str(&format!("\n\nhelp: {}", advice)[..])
        }
        output
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
