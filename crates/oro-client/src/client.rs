use std::{
    path::{Path, PathBuf},
    sync::Arc,
};

#[cfg(not(target_arch = "wasm32"))]
use http_cache_reqwest::{CACacheManager, Cache, CacheMode, HttpCache};
#[cfg(not(target_arch = "wasm32"))]
use reqwest::{Client, ClientBuilder};
use reqwest_middleware::ClientWithMiddleware;
#[cfg(target_arch = "wasm32")]
use reqwest_wasm::Client;
use url::Url;

#[derive(Clone, Debug)]
pub struct OroClientBuilder {
    registry: Url,
    #[cfg(not(target_arch = "wasm32"))]
    cache: Option<PathBuf>,
}

impl Default for OroClientBuilder {
    fn default() -> Self {
        Self {
            registry: Url::parse("https://registry.npmjs.org").unwrap(),
            #[cfg(not(target_arch = "wasm32"))]
            cache: None,
        }
    }
}

impl OroClientBuilder {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn registry(mut self, registry: Url) -> Self {
        self.registry = registry;
        self
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub fn cache(mut self, cache: impl AsRef<Path>) -> Self {
        self.cache = Some(PathBuf::from(cache.as_ref()));
        self
    }

    pub fn build(self) -> OroClient {
        #[cfg(not(target_arch = "wasm32"))]
        let client_uncached = ClientBuilder::new()
            .user_agent("orogene")
            .pool_max_idle_per_host(20)
            .build()
            .expect("Failed to build HTTP client.");

        #[cfg(target_arch = "wasm32")]
        let client_uncached = Client::new();

        #[cfg(not(target_arch = "wasm32"))]
        let mut client_builder = reqwest_middleware::ClientBuilder::new(client_uncached.clone());

        #[cfg(not(target_arch = "wasm32"))]
        if let Some(cache_loc) = self.cache {
            client_builder = client_builder.with(Cache(HttpCache {
                mode: CacheMode::Default,
                manager: CACacheManager {
                    path: cache_loc.to_string_lossy().into(),
                },
                options: None,
            }));
        }

        OroClient {
            registry: Arc::new(self.registry),
            #[cfg(not(target_arch = "wasm32"))]
            client: client_builder.build(),
            // wasm client doesn't support extra options.
            #[cfg(target_arch = "wasm32")]
            client: client_uncached.clone(),
            client_uncached,
        }
    }
}

#[derive(Clone, Debug)]
pub struct OroClient {
    pub(crate) registry: Arc<Url>,
    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) client: ClientWithMiddleware,
    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) client_uncached: Client,
    #[cfg(target_arch = "wasm32")]
    pub(crate) client: Client,
    #[cfg(target_arch = "wasm32")]
    pub(crate) client_uncached: Client,
}

impl OroClient {
    pub fn builder() -> OroClientBuilder {
        OroClientBuilder::new()
    }

    pub fn new(registry: Url) -> Self {
        Self::builder().registry(registry).build()
    }

    pub fn with_registry(&self, registry: Url) -> Self {
        Self {
            registry: Arc::new(registry),
            client: self.client.clone(),
            client_uncached: self.client_uncached.clone(),
        }
    }
}

impl Default for OroClient {
    fn default() -> Self {
        OroClientBuilder::new().registry(Url::parse("https://registry.npmjs.org").unwrap()).build()
    }
}
