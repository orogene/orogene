use miette::Diagnostic;
use thiserror::Error;

#[derive(Debug, Error, Diagnostic)]
pub enum OroConfigError {
    /// A generic config error happened while loading the config. Refer to the
    /// error message for more details.
    #[error(transparent)]
    #[diagnostic(code(oro_config::error), url(docsrs))]
    ConfigError(#[from] config::ConfigError),

    /// A generic error happened while parsing the config. Refer to the error
    /// message for more details.
    #[error(transparent)]
    #[diagnostic(code(oro_config::error), url(docsrs))]
    ConfigParseError(#[from] Box<dyn std::error::Error + Send + Sync>),
}
