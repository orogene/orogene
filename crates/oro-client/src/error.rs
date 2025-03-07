use miette::{Diagnostic, NamedSource, SourceOffset};
use reqwest::Url;
use thiserror::Error;

#[derive(Debug)]
pub struct Response(Option<String>);

#[derive(Debug, Error, Diagnostic)]
pub enum OroClientError {
    /// An invalid URL was provided.
    #[error(transparent)]
    #[diagnostic(code(oro_client::url_parse_error), url(docsrs))]
    UrlParseError(#[from] url::ParseError),

    /// The package was not found in the registry.
    ///
    /// Make sure the package name is spelled correctly and that you've
    /// configured the right registry to fetch it from.
    #[error("Package `{1}` was not found in registry {0}.")]
    #[diagnostic(code(oro_client::package_not_found), url(docsrs))]
    PackageNotFound(Url, String),

    /// Got some bad JSON we couldn't parse.
    #[error("Received some unexpected JSON. Unable to parse.")]
    #[diagnostic(code(oro_client::bad_json), url(docsrs))]
    BadJson {
        source: serde_json::Error,
        url: String,
        #[source_code]
        json: NamedSource<String>,
        #[label("here")]
        err_loc: (usize, usize),
    },

    /// A generic request error happened while making a request. Refer to the
    /// error message for more details.
    #[error(transparent)]
    #[diagnostic(code(oro_client::request_error), url(docsrs))]
    RequestError(#[from] reqwest::Error),

    /// Recived unexpected response.
    #[error("Received unexpected response. \n {0}")]
    #[diagnostic(code(oro_client::response_error), url(docsrs))]
    ResponseError(Response),

    /// No such user.
    #[error("No such user. (provided username: {0})")]
    #[diagnostic(code(oro_client::no_such_user_error), url(docsrs))]
    NoSuchUserError(String),

    /// Incorrect or missing password.
    #[error("Incorrect or missing password.")]
    #[diagnostic(code(oro_client::incorrect_password_error), url(docsrs))]
    IncorrectPasswordError,

    /// Unable to authenticate, your authentication token seems to be invalid.
    #[error("Unable to authenticate, your authentication token seems to be invalid.")]
    #[diagnostic(code(oro_client::invalid_token_error), url(docsrs))]
    InvalidTokenError,

    /// This operation requires a one-time password from your authenticator.
    #[error("This operation requires a one-time password from your authenticator.")]
    #[diagnostic(code(oro_client::otp_required_error), url(docsrs))]
    OTPRequiredError,

    /// A generic request middleware error happened while making a request.
    /// Refer to the error message for more details.
    #[error(transparent)]
    #[diagnostic(code(oro_client::request_middleware_error), url(docsrs))]
    RequestMiddlewareError(#[from] reqwest_middleware::Error),

    /// An error during reading the configuration
    #[error("Could not parse credentials config. {0}")]
    #[diagnostic(code(oro_client::credentials_config_error), url(docsrs))]
    CredentialsConfigError(String),

    /// Auth string did not include a username when decoded.
    #[error("Auth string did not include a username when decoded.")]
    #[diagnostic(code(oro_client::auth_string_missing_username), url(docsrs))]
    AuthStringMissingUsername(String),

    /// Failed to decode base64.
    #[error(transparent)]
    #[diagnostic(code(oro_client::base64_decode_error), url(docsrs))]
    Base64DecodeError(#[from] base64::DecodeError),
}

impl OroClientError {
    pub fn from_json_err(err: serde_json::Error, url: String, json: String) -> Self {
        // These json strings can get VERY LONG and miette doesn't (yet?)
        // support any "windowing" mechanism for displaying stuff, so we have
        // to manually shorten the string to only the relevant bits and
        // translate the spans accordingly.
        let err_offset = SourceOffset::from_location(&json, err.line(), err.column());
        let json_len = json.len();
        let local_offset = err_offset.offset().saturating_sub(40);
        let local_len = std::cmp::min(40, json_len - err_offset.offset());
        let snipped_json = json[local_offset..err_offset.offset() + local_len].to_string();
        Self::BadJson {
            source: err,
            url: url.clone(),
            json: NamedSource::new(url, snipped_json),
            err_loc: (err_offset.offset() - local_offset, 0),
        }
    }
}

impl From<Option<String>> for Response {
    fn from(value: Option<String>) -> Self {
        Response(value)
    }
}

impl std::fmt::Display for Response {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(
            f,
            "{}",
            if let Some(response) = &self.0 {
                response
            } else {
                ""
            }
        )
    }
}
