use std::sync::Arc;

use reqwest::Client;
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
            client: Client::new(),
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
        Self {
            registry: Arc::new(Url::parse("https://registry.npmjs.org").unwrap()),
            client: Client::new(),
        }
    }
}
