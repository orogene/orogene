use miette::{Diagnostic, NamedSource, SourceOffset};
use thiserror::Error;

#[derive(Debug, Error, Diagnostic)]
pub enum OroClientError {
    #[error(transparent)]
    #[diagnostic(code(oro_client::url_parse_error))]
    UrlParseError(#[from] url::ParseError),

    #[error("Package was not found in registry: {0}")]
    #[diagnostic(code(oro_client::package_not_found))]
    PackageNotFound(String),

    /// Got some bad JSON we couldn't parse.
    #[error("Received some unexpected JSON. Unable to parse.")]
    #[diagnostic(code(oro_client::bad_json))]
    BadJson {
        source: serde_json::Error,
        url: String,
        #[source_code]
        json: NamedSource,
        #[label("here")]
        err_loc: (usize, usize),
    },

    #[error(transparent)]
    #[diagnostic(code(oro_client::request_error))]
    RequestError(#[from] reqwest::Error),

    #[cfg(not(target_arch = "wasm32"))]
    #[error(transparent)]
    #[diagnostic(code(oro_client::request_middleware_error))]
    RequestMiddlewareError(#[from] reqwest_middleware::Error),
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
