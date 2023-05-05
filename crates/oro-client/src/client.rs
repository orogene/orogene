#[cfg(not(target_arch = "wasm32"))]
use std::path::{Path, PathBuf};
use std::{collections::HashMap, sync::Arc};

#[cfg(not(target_arch = "wasm32"))]
use http_cache_reqwest::{CACacheManager, Cache, CacheMode, HttpCache};
use miette::Result;
use reqwest::Client;
#[cfg(not(target_arch = "wasm32"))]
use reqwest::ClientBuilder;
#[cfg(not(target_arch = "wasm32"))]
use reqwest_middleware::ClientWithMiddleware;
use url::Url;

use crate::{credentials::Credentials, OroClientError};

#[derive(Clone, Debug)]
pub struct OroClientBuilder {
    registry: Url,
    credentials: HashMap<String, Credentials>,
    #[cfg(not(target_arch = "wasm32"))]
    cache: Option<PathBuf>,
}

impl Default for OroClientBuilder {
    fn default() -> Self {
        Self {
            registry: Url::parse("https://registry.npmjs.org").unwrap(),
            credentials: HashMap::new(),
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

    pub fn credentials(mut self, credentials: Vec<(String, String, String)>) -> Result<Self> {
        let mut vars = HashMap::new();
        for (registry, key, value) in credentials.into_iter() {
            if !vars.contains_key(&registry) {
                vars.insert(registry.clone(), HashMap::new());
            }
            let existing = vars
                .get_mut(&registry)
                .and_then(|reg| reg.insert(key.clone(), value.clone()));
            if existing.is_some() {
                Err(OroClientError::CredentialsConfigError(format!(
                    "Key \"{}\" already exists for registry {}",
                    key, registry
                )))?
            }
        }
        for (registry, config) in vars.into_iter() {
            self.credentials.insert(registry, config.try_into()?);
        }
        Ok(self)
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub fn cache(mut self, cache: impl AsRef<Path>) -> Self {
        self.cache = Some(PathBuf::from(cache.as_ref()));
        self
    }

    pub fn build(self) -> OroClient {
        #[cfg(target_arch = "wasm32")]
        let client_uncached = Client::new();

        #[cfg(not(target_arch = "wasm32"))]
        let client_uncached = ClientBuilder::new()
            .user_agent("orogene")
            .pool_max_idle_per_host(20)
            .timeout(std::time::Duration::from_secs(60 * 5))
            .build()
            .expect("Failed to build HTTP client.");

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
            credentials: Arc::new(self.credentials),
            #[cfg(not(target_arch = "wasm32"))]
            client: client_builder.build(),
            // wasm client is never cached
            #[cfg(target_arch = "wasm32")]
            client: client_uncached.clone(),
            client_uncached,
        }
    }
}

#[derive(Clone, Debug)]
pub struct OroClient {
    pub(crate) registry: Arc<Url>,
    pub(crate) credentials: Arc<HashMap<String, Credentials>>,
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
            credentials: Arc::new(HashMap::new()),
            client: self.client.clone(),
            client_uncached: self.client_uncached.clone(),
        }
    }
}

impl Default for OroClient {
    fn default() -> Self {
        OroClientBuilder::new()
            .registry(Url::parse("https://registry.npmjs.org").unwrap())
            .build()
    }
}
