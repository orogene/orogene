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
}

impl OroClient {
    pub fn new(registry_uri: impl AsRef<str>) -> Self {
        Self {
            base: Url::parse(registry_uri.as_ref()).expect("Invalid registry URI"),
            client: Client::new(),
        }
    }

    pub async fn get(&self, uri: impl AsRef<str>) -> Result<Response, OroClientError> {
        self.client
            .get(self.base.join(uri.as_ref()).unwrap())
            .await
            .map_err(OroClientError::RequestError)
    }
}
