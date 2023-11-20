use miette::Diagnostic;
use thiserror::Error;

#[derive(Debug, Error, Diagnostic)]
pub enum OroNpmPublishError {
    /// An error was thrown in OroClient.
    #[error(transparent)]
    #[diagnostic(code(oro_npm_publish::client_error), url(docsrs))]
    ClientError(#[from] oro_client::OroClientError),

    // An error was thrown in `futures` crate.
    #[error(transparent)]
    #[diagnostic(code(oro_npm_publish::io_error), url(docsrs))]
    IoError(#[from] futures::io::Error),

    /// Failed to parse URL.
    #[error(transparent)]
    #[diagnostic(code(oro_npm_publish::parse_url_error), url(docsrs))]
    ParseURLError(#[from] url::ParseError),

    /// Failed to open URL.
    #[error(transparent)]
    #[diagnostic(code(oro_npm_publish::open_url_error), url(docsrs))]
    OpenURLError(std::io::Error),

    /// Received unexpected response.
    #[error("Received unexpacted response.")]
    #[diagnostic(code(oro_npm_publish::unexpectex_response_error), url(docsrs))]
    ReceivedUnexpectedResponse,

    /// This package has been marked as private.
    #[error("This package has been marked as private.")]
    #[diagnostic(code(oro_npm_publish::private_package_error), url(docsrs))]
    PrivatePackageError,

    /// Required field is missing.
    #[error("Required field is missing: {0}")]
    #[diagnostic(code(oro_npm_publish::required_field_is_missing), url(docsrs))]
    RequiredFieldIsMissing(String),

    /// Can't restrict access to unscoped packages.
    #[error("Can't restrict access to unscoped packages.")]
    #[diagnostic(code(oro_npm_publish::access_to_unscoped_package_error), url(docsrs))]
    AccessToUnscopedPackageError,
}
