use miette::Diagnostic;
use thiserror::Error;

#[derive(Debug, Error, Diagnostic)]
pub enum OroClientError {
    #[error(transparent)]
    #[diagnostic(code(oro_client::url_parse_error))]
    UrlParseError(#[from] url::ParseError),

    #[error("Package was not found in registry: {0}")]
    #[diagnostic(code(oro_client::package_not_found))]
    PackageNotFound(String),

    #[error(transparent)]
    #[diagnostic(code(oro_client::request_error))]
    RequestError(#[from] reqwest::Error),
}
