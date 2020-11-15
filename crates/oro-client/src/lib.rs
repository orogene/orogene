use oro_diagnostics::{Diagnostic, DiagnosticCategory};
use serde::Deserialize;
use surf::Client;
use thiserror::Error;

pub use surf::{
    http::{url::ParseError, Method, StatusCode, Url},
    Error as SurfError, RequestBuilder, Response,
};

use crate::http_client::PoolingClient;

mod http_client;

#[derive(Debug, Error)]
pub enum OroClientError {
    // TODO: add registry URL here?
    #[error("Registry request failed:\n\t{surf_err}")]
    RequestError { surf_err: SurfError, url: Url },
    #[error("Registry returned failed status code {status_code} for a request.\n\t{}", context.join("\n  "))]
    ResponseError {
        url: Url,
        status_code: StatusCode,
        context: Vec<String>,
    },
}

impl Diagnostic for OroClientError {
    fn category(&self) -> DiagnosticCategory {
        use DiagnosticCategory::*;
        use OroClientError::*;
        match self {
            RequestError { ref url, .. } => Net {
                url: Some(url.clone()),
                host: url.host().expect("this should have a host").to_owned(),
            },
            ResponseError { ref url, .. } => Net {
                url: Some(url.clone()),
                host: url.host().expect("this should have a host").to_owned(),
            },
        }
    }

    fn subpath(&self) -> String {
        todo!()
    }

    fn advice(&self) -> Option<String> {
        todo!()
    }
}

impl OroClientError {
    fn res_err(url: Url, res: &Response, ctx: Vec<String>) -> Self {
        Self::ResponseError {
            status_code: res.status(),
            context: ctx,
            url,
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
                parse_err @ Err(_) => match res.body_string().await {
                    Ok(msg) => msg,
                    body_err @ Err(_) => {
                        return Err(OroClientError::res_err(
                            url,
                            &res,
                            vec![format!("{:?}", parse_err), format!("{:?}", body_err)],
                        ));
                    }
                },
            };
            Err(OroClientError::res_err(url, &res, vec![msg]))
        } else {
            Ok(res)
        }
    }
}
