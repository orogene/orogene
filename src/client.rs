use anyhow;
use http_types::Url;
use surf::Client;

// TODO: eventually wrap this?
pub use surf::Response;

pub struct OroClient {
    base: Url,
    client: Client,
}

impl OroClient {
    pub fn new(registry_uri: impl AsRef<str>) -> Self {
        Self {
            base: Url::parse(registry_uri.as_ref()).expect("Invalid registry URI"),
            client: Client::new(),
        }
    }

    pub async fn get(&self, uri: impl AsRef<str>) -> anyhow::Result<Response> {
        self.client.get(self.base.join(uri.as_ref()).unwrap()).await.map_err(|e| anyhow::anyhow!(e))
    }
}
