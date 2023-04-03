use miette::Diagnostic;
use thiserror::Error;

#[derive(Debug, Error, Diagnostic)]
pub enum OroConfigError {
    #[error(transparent)]
    #[diagnostic(code(config::error))]
    ConfigError(#[from] config::ConfigError),

    #[error(transparent)]
    #[diagnostic(code(config::error))]
    ConfigParseError(#[from] Box<dyn std::error::Error + Send + Sync>),
}
