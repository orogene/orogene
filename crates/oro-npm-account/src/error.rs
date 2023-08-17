use miette::Diagnostic;
use thiserror::Error;

#[derive(Debug, Error, Diagnostic)]
pub enum OroNpmAccountError {
    /// An error was thrown in OroClient.
    #[error(transparent)]
    #[diagnostic(code(oro_npm_account::client_error), url(docsrs))]
    ClientError(#[from] oro_client::OroClientError),

    /// Failed to open URL.
    #[error(transparent)]
    #[diagnostic(code(oro_npm_account::url_open_error), url(docsrs))]
    OpenURLError(std::io::Error),

    /// Failed to read user input.
    #[error(transparent)]
    #[diagnostic(code(oro_npm_account::read_user_input_error), url(docsrs))]
    ReadUserInputError(std::io::Error),

    /// Invalid header value
    #[error(transparent)]
    #[diagnostic(code(oro_npm_accout::invalid_header_value), url(docsrs))]
    InvalidHeaderValueError(#[from] reqwest::header::InvalidHeaderValue),

    /// Unsupported conversion
    #[error("Unsupported conversion.")]
    #[diagnostic(code(oro_npm_account::unsupported_conversion_error), url(docsrs))]
    UnsupportedConversionError,

    /// Received unexpected response.
    #[error("Received unexpected response.")]
    #[diagnostic(code(oro_npm_account::unexpected_response_error), url(docsrs))]
    UnexpectedResponseError,
}
