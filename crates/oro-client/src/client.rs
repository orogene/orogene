use std::sync::Arc;

use reqwest::{Client, ClientBuilder};
use url::Url;

#[derive(Clone, Debug)]
pub struct OroClient {
    pub(crate) registry: Arc<Url>,
    pub(crate) client: Client,
}

impl OroClient {
    pub fn new(registry: Url) -> Self {
        Self {
            registry: Arc::new(registry),
            client: ClientBuilder::new()
                .user_agent("orogene")
                .pool_max_idle_per_host(20)
                .build()
                .expect("Failed to build HTTP client."),
        }
    }

    pub fn with_registry(&self, registry: Url) -> Self {
        Self {
            registry: Arc::new(registry),
            client: self.client.clone(),
        }
    }
}

impl Default for OroClient {
    fn default() -> Self {
        Self::new(Url::parse("https://registry.npmjs.org").unwrap())
    }
}
