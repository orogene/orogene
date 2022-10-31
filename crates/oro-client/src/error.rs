use miette::Diagnostic;
use thiserror::Error;

#[derive(Debug, Error, Diagnostic)]
pub enum OroClientError {
    #[error(transparent)]
    #[diagnostic(code(oro_client::url_parse_error))]
    UrlParseError(#[from] url::ParseError),

    #[error(transparent)]
    #[diagnostic(code(oro_client::generic_error))]
    GenericError(#[from] reqwest::Error),
}
