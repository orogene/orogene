use std::sync::Arc;

use oro_error_code::OroErrCode as Code;
use serde::Deserialize;
use surf::Client;
use thiserror::Error;

pub use surf::{
    http::{url::ParseError, Method, StatusCode, Url},
    Error as SurfError, RequestBuilder, Response,
};

use crate::http_client::H1Client;

mod http_client;

#[derive(Debug, Error)]
pub enum OroClientError {
    #[error(transparent)]
    UrlParseError(#[from] ParseError),
    #[error("Request failed: {0}")]
    RequestError(SurfError),
    #[error("{}", context.join("\n  "))]
    ResponseError {
        code: StatusCode,
        context: Vec<String>,
    },
}

impl OroClientError {
    fn res_err(res: &Response, ctx: Vec<String>) -> Self {
        Self::ResponseError {
            code: res.status(),
            context: ctx,
        }
    }
}

#[derive(Debug, Deserialize)]
struct NpmError {
    message: String,
}

#[derive(Clone)]
pub struct OroClient {
    base: Url,
    client: Client,
}

impl OroClient {
    pub fn new(registry_uri: impl AsRef<str>) -> Self {
        Self {
            base: Url::parse(registry_uri.as_ref()).expect("Invalid registry URI"),
            client: Client::with_http_client(Arc::new(H1Client::new())),
        }
    }

    pub fn opts<T: AsRef<str>>(&self, method: Method, uri: T) -> RequestBuilder {
        let uri =
            Url::parse(uri.as_ref()).unwrap_or_else(|_| self.base.join(uri.as_ref()).unwrap());
        RequestBuilder::new(method, uri)
    }

    pub async fn send(&self, request: RequestBuilder) -> Result<Response, OroClientError> {
        let mut res = self
            .client
            .send(request)
            .await
            .map_err(OroClientError::RequestError)?;
        if res.status().is_client_error() || res.status().is_server_error() {
            let msg = match res.body_json::<NpmError>().await {
                Ok(err) => err.message,
                parse_err @ Err(_) => match res.body_string().await {
                    Ok(msg) => msg,
                    body_err @ Err(_) => {
                        return Err(OroClientError::res_err(
                            &res,
                            vec![
                                format!("{}", Code::OR1002),
                                format!("{:?}", parse_err),
                                format!("{:?}", body_err),
                            ],
                        ));
                    }
                },
            };
            Err(OroClientError::res_err(
                &res,
                vec![format!(
                    "{}",
                    Code::OR1003 {
                        registry: self.base.to_string(),
                        status: res.status(),
                        message: msg,
                    }
                )],
            ))
        } else {
            Ok(res)
        }
    }
}
