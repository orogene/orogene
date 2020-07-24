use http_types::StatusCode;
use oro_error_code::OroErrCode as Code;
use serde::Deserialize;
use surf::Client;
use thiserror::Error;

pub use surf::{http::Url, Error as SurfError, Response};

pub struct OroClient {
    base: Url,
    client: Client,
}

#[derive(Debug, Error)]
pub enum OroClientError {
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

impl OroClient {
    pub fn new(registry_uri: impl AsRef<str>) -> Self {
        Self {
            base: Url::parse(registry_uri.as_ref()).expect("Invalid registry URI"),
            client: Client::new(),
        }
    }

    pub async fn get(&self, uri: impl AsRef<str>) -> Result<Response, OroClientError> {
        self.get_absolute(self.base.join(uri.as_ref()).unwrap())
            .await
    }
    pub async fn get_absolute(&self, uri: impl AsRef<str>) -> Result<Response, OroClientError> {
        let mut res = self
            .client
            .get(uri.as_ref())
            // TODO: how tf do I abstract header-setting away while still controlling the output stuff??
            // .set_header(
            //     "accept",
            //     "application/vnd.npm.install-v1+json; q=1.0, application/json; q=0.8, */*",
            // )
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
