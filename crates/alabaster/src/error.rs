use miette::Diagnostic;
use thiserror::Error;

#[derive(Debug, Error, Diagnostic)]
pub enum AlabasterError {
    /// Failed to initialize the macOS File Provider for managing
    /// `node_modules/`.
    #[error("Failed to initialize macOS File Provider: {0}")]
    #[diagnostic(code(alabaster::macos::file_provider_init_error))]
    FileProviderInitError(String),
}
