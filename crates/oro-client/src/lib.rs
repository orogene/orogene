use oro_diagnostics::{Diagnostic, DiagnosticCategory, Explain, Meta};
use oro_diagnostics_derive::Diagnostic;
use serde::Deserialize;
use surf::Client;
use thiserror::Error;

pub use surf::{
    http::{url::ParseError, Method, StatusCode, Url},
    Error as SurfError, RequestBuilder, Response,
};

use crate::http_client::PoolingClient;

mod http_client;

#[derive(Debug, Error, Diagnostic)]
pub enum OroClientError {
    // TODO: add registry URL here?
    #[error("Registry request failed:\n\t{surf_err}")]
    #[category(Net)]
    #[label("client::bad_request")]
    RequestError { surf_err: SurfError, url: Url },

    #[error("Registry returned failed status code {status_code} for a request.")]
    #[category(Net)]
    #[label("client::response_failure")]
    ResponseError {
        url: Url,
        status_code: StatusCode,
        message: Option<String>,
    },
}

impl Explain for OroClientError {
    fn meta(&self) -> Option<Meta> {
        use OroClientError::*;
        match self {
            RequestError { ref url, .. } => Some(Meta::Net {
                url: Some(url.clone()),
            }),
            ResponseError { ref url, .. } => Some(Meta::Net {
                url: Some(url.clone()),
            }),
        }
    }
}

#[derive(Debug, Deserialize)]
struct NpmError {
    message: String,
}

#[derive(Clone, Debug)]
pub struct OroClient {
    client: Client,
}

impl Default for OroClient {
    fn default() -> Self {
        Self {
            client: Client::with_http_client(PoolingClient::new()),
        }
    }
}

impl OroClient {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn opts(&self, method: Method, uri: Url) -> RequestBuilder {
        RequestBuilder::new(method, uri)
    }

    pub async fn send(&self, request: RequestBuilder) -> Result<Response, OroClientError> {
        let req = request.build();
        let url = req.url().clone();
        let mut res = self
            .client
            .send(req)
            .await
            .map_err(|e| OroClientError::RequestError {
                surf_err: e,
                url: url.clone(),
            })?;
        if res.status().is_client_error() || res.status().is_server_error() {
            let msg = match res.body_json::<NpmError>().await {
                Ok(err) => err.message,
                Err(_) => match res.body_string().await {
                    Ok(msg) => msg,
                    Err(_) => {
                        return Err(OroClientError::ResponseError {
                            url,
                            status_code: res.status(),
                            message: None,
                        });
                    }
                },
            };
            Err(OroClientError::ResponseError {
                status_code: res.status(),
                url,
                message: Some(msg),
            })
        } else {
            Ok(res)
        }
    }
}
