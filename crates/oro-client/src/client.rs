#[cfg(not(target_arch = "wasm32"))]
use std::path::{Path, PathBuf};
use std::sync::Arc;

#[cfg(not(target_arch = "wasm32"))]
use http_cache_reqwest::{CACacheManager, Cache, CacheMode, HttpCache};
use reqwest::Client;
#[cfg(not(target_arch = "wasm32"))]
use reqwest::ClientBuilder;
#[cfg(not(target_arch = "wasm32"))]
use reqwest_middleware::ClientWithMiddleware;
use reqwest::{Proxy, NoProxy};
use url::Url;

use crate::OroClientError;

#[derive(Clone, Debug)]
pub struct OroClientBuilder {
    registry: Url,
    #[cfg(not(target_arch = "wasm32"))]
    cache: Option<PathBuf>,
    #[cfg(not(target_arch = "wasm32"))]
    proxy: bool,
    #[cfg(not(target_arch = "wasm32"))]
    proxy_url: Option<&'static str>,
    #[cfg(not(target_arch = "wasm32"))]
    no_proxy: Option<&'static str>,
}

impl Default for OroClientBuilder {
    fn default() -> Self {
        Self {
            registry: Url::parse("https://registry.npmjs.org").unwrap(),
            #[cfg(not(target_arch = "wasm32"))]
            cache: None,
            #[cfg(not(target_arch = "wasm32"))]
            proxy: false,
            #[cfg(not(target_arch = "wasm32"))]
            proxy_url: None,
            #[cfg(not(target_arch = "wasm32"))]
            no_proxy: Some("NO_PROXY"),
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

    pub fn set_proxy(mut self, proxy: bool) -> Self {
        self.proxy = proxy;
        self
    }

    pub fn set_proxy_url<'a>(mut self, proxy_url: &'static str) -> Self {
        let other = self.clone();
        self.proxy_url = Some(proxy_url);
        other.set_proxy(true);
        self
    }

    pub fn set_no_proxy<'a>(mut self, no_proxy: &'static str) -> Self {
        self.no_proxy = Some(no_proxy);
        self
    }

    pub fn build(self) -> OroClient {
        #[cfg(target_arch = "wasm32")]
        let mut client_uncached = Client::new();

        #[cfg(not(target_arch = "wasm32"))]
        let mut client_core = ClientBuilder::new()
            .user_agent("orogene")
            .pool_max_idle_per_host(20)
            .timeout(std::time::Duration::from_secs(60 * 5));

        #[cfg(not(target_arch = "wasm32"))]
        if let Some(url) = self.proxy_url {
            client_core = client_core.proxy(self.set_request_proxy(url).unwrap());
        }

        #[cfg(not(target_arch = "wasm32"))]
        if !self.proxy {
            client_core = client_core.no_proxy();
        }

        #[cfg(not(target_arch = "wasm32"))]
        let client_uncached = client_core.build().expect("Fail to build HTTP client.");

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
            // wasm client is never cached
            #[cfg(target_arch = "wasm32")]
            client: client_uncached.clone(),
            client_uncached,
        }
    }


    fn get_no_proxy(&self) -> Option<NoProxy> {
        if let Some(no_proxy_conf) = self.no_proxy {
            if no_proxy_conf != "NO_PROXY" || no_proxy_conf != "" {
                Some(NoProxy::from_string(no_proxy_conf));
            }
        }

        NoProxy::from_env().or(None)
    }

    fn set_request_proxy(&self, url: &str) -> Result<Proxy, OroClientError> {
        let url_info = Url::parse(url).expect("Fail to parse proxy url");
        let username = url_info.username();
        let password = url_info.password();
        let mut proxy = Proxy::all(url_info.as_ref())?;

        if let Some(password_str) = password {
            proxy = proxy.basic_auth(username, password_str);
        }

        proxy = proxy.no_proxy(self.get_no_proxy());
        Ok(proxy)
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
        OroClientBuilder::new()
            .registry(Url::parse("https://registry.npmjs.org").unwrap())
            .build()
    }
}
