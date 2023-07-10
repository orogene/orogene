#[cfg(not(target_arch = "wasm32"))]
use std::path::{Path, PathBuf};
use std::sync::Arc;

#[cfg(not(target_arch = "wasm32"))]
use http_cache_reqwest::{CACacheManager, Cache, CacheMode, HttpCache};
use reqwest::Client;
#[cfg(not(target_arch = "wasm32"))]
use reqwest::ClientBuilder;
#[cfg(not(target_arch = "wasm32"))]
use reqwest::{NoProxy, Proxy};
#[cfg(not(target_arch = "wasm32"))]
use reqwest_middleware::ClientWithMiddleware;
use url::Url;

#[cfg(not(target_arch = "wasm32"))]
use crate::OroClientError;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OroClientProxyConfig {
    pub proxy: bool,
    pub proxy_url: Option<String>,
    pub no_proxy_domain: Option<String>,
}

impl Default for OroClientProxyConfig {
    fn default() -> Self {
        Self {
            proxy: false,
            proxy_url: None,
            no_proxy_domain: Some("NO_PROXY".to_string()),
        }
    }
}

impl OroClientProxyConfig {
    pub fn set_proxy(mut self, proxy: bool) -> Self {
        self.proxy = proxy;
        self
    }

    pub fn set_proxy_url(mut self, proxy_url: impl AsRef<str>) -> Self {
        self.proxy_url = Some(proxy_url.as_ref().into());
        self.proxy = true;
        self
    }

    pub fn set_no_proxy_domain(mut self, no_proxy_domain: impl AsRef<str>) -> Self {
        self.no_proxy_domain = Some(no_proxy_domain.as_ref().into());
        self
    }
}

#[derive(Clone, Debug)]
pub struct OroClientBuilder {
    registry: Url,
    #[cfg(not(target_arch = "wasm32"))]
    cache: Option<PathBuf>,
    #[cfg(not(target_arch = "wasm32"))]
    proxy_config: OroClientProxyConfig,
}

impl Default for OroClientBuilder {
    fn default() -> Self {
        Self {
            registry: Url::parse("https://registry.npmjs.org").unwrap(),
            #[cfg(not(target_arch = "wasm32"))]
            cache: None,
            #[cfg(not(target_arch = "wasm32"))]
            proxy_config: OroClientProxyConfig::default(),
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

    #[cfg(not(target_arch = "wasm32"))]
    pub fn set_proxy(mut self, proxy: bool) -> Self {
        self.proxy_config.proxy = proxy;
        self
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub fn set_proxy_url(mut self, proxy_url: impl AsRef<str>) -> Self {
        self.proxy_config.proxy_url = Some(proxy_url.as_ref().into());
        self.proxy_config.proxy = true;
        self
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub fn set_no_proxy(mut self, no_proxy_domain: impl AsRef<str>) -> Self {
        self.proxy_config.no_proxy_domain = Some(no_proxy_domain.as_ref().into());
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
        if let Some(ref url) = self.proxy_config.proxy_url {
            client_core = client_core.proxy(self.set_request_proxy(url).unwrap());
        }

        #[cfg(not(target_arch = "wasm32"))]
        if !self.proxy_config.proxy {
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

    #[cfg(not(target_arch = "wasm32"))]
    fn get_no_proxy(&self) -> Option<NoProxy> {
        if let Some(ref no_proxy_conf) = self.proxy_config.no_proxy_domain {
            if no_proxy_conf != "NO_PROXY" || no_proxy_conf != "" {
                Some(NoProxy::from_string(no_proxy_conf));
            }
        }

        NoProxy::from_env().or(None)
    }

    #[cfg(not(target_arch = "wasm32"))]
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
