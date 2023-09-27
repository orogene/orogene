#[cfg(not(target_arch = "wasm32"))]
use std::path::{Path, PathBuf};
use std::{collections::HashMap, sync::Arc};

#[cfg(not(target_arch = "wasm32"))]
use http_cache_reqwest::{CACacheManager, Cache, CacheMode, HttpCache};
#[cfg(target_arch = "wasm32")]
use reqwest::Client;
#[cfg(not(target_arch = "wasm32"))]
use reqwest::ClientBuilder;
#[cfg(not(target_arch = "wasm32"))]
use reqwest::{NoProxy, Proxy};
use reqwest_middleware::ClientWithMiddleware;
use reqwest_retry::{policies::ExponentialBackoff, RetryTransientMiddleware};
use url::Url;

#[cfg(not(target_arch = "wasm32"))]
use crate::OroClientError;
use crate::{
    auth_middleware::{self, AuthMiddleware},
    credentials::Credentials,
};

#[derive(Clone, Debug)]
pub struct OroClientBuilder {
    registry: Url,
    retries: u32,
    credentials: HashMap<String, Credentials>,
    #[cfg(not(target_arch = "wasm32"))]
    cache: Option<PathBuf>,
    #[cfg(not(target_arch = "wasm32"))]
    proxy: bool,
    #[cfg(not(target_arch = "wasm32"))]
    proxy_url: Option<Proxy>,
    #[cfg(not(target_arch = "wasm32"))]
    no_proxy_domain: Option<String>,
}

impl Default for OroClientBuilder {
    fn default() -> Self {
        Self {
            registry: Url::parse("https://registry.npmjs.org").unwrap(),
            credentials: HashMap::new(),
            #[cfg(not(target_arch = "wasm32"))]
            cache: None,
            #[cfg(not(target_arch = "wasm32"))]
            proxy: false,
            #[cfg(not(target_arch = "wasm32"))]
            proxy_url: None,
            #[cfg(not(target_arch = "wasm32"))]
            no_proxy_domain: None,
            #[cfg(not(test))]
            retries: 2,
            #[cfg(test)]
            retries: 0,
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

    pub fn basic_auth(mut self, registry: Url, username: String, password: Option<String>) -> Self {
        self.credentials.insert(
            auth_middleware::nerf_dart(&registry),
            Credentials::Basic { username, password },
        );
        self
    }

    pub fn token_auth(mut self, registry: Url, token: String) -> Self {
        self.credentials.insert(
            auth_middleware::nerf_dart(&registry),
            Credentials::Token(token),
        );
        self
    }

    pub fn legacy_auth(mut self, registry: Url, legacy_auth_token: String) -> Self {
        self.credentials.insert(
            auth_middleware::nerf_dart(&registry),
            Credentials::EncodedBasic(legacy_auth_token),
        );
        self
    }

    pub fn retries(mut self, retries: u32) -> Self {
        self.retries = retries;
        self
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub fn cache(mut self, cache: impl AsRef<Path>) -> Self {
        self.cache = Some(PathBuf::from(cache.as_ref()));
        self
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub fn proxy(mut self, proxy: bool) -> Self {
        self.proxy = proxy;
        self
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub fn proxy_url(mut self, proxy_url: impl AsRef<str>) -> Result<Self, OroClientError> {
        match Url::parse(proxy_url.as_ref()) {
            Ok(url_info) => {
                let username = url_info.username();
                let password = url_info.password();
                let mut proxy = Proxy::all(url_info.as_ref())?;

                if let Some(password_str) = password {
                    proxy = proxy.basic_auth(username, password_str);
                }

                proxy = proxy.no_proxy(self.get_no_proxy_domain());
                self.proxy_url = Some(proxy);
                self.proxy = true;
                Ok(self)
            }
            Err(e) => Err(OroClientError::UrlParseError(e)),
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub fn no_proxy_domain(mut self, no_proxy_domain: impl AsRef<str>) -> Self {
        self.no_proxy_domain = Some(no_proxy_domain.as_ref().into());
        self
    }

    pub fn build(self) -> OroClient {
        #[cfg(target_arch = "wasm32")]
        let client_raw = Client::new();

        #[cfg(not(target_arch = "wasm32"))]
        let client_raw = {
            let mut client_core = ClientBuilder::new()
                .user_agent("orogene")
                .pool_max_idle_per_host(20)
                .timeout(std::time::Duration::from_secs(60 * 5));

            if let Some(url) = self.proxy_url {
                client_core = client_core.proxy(url);
            }

            if !self.proxy {
                client_core = client_core.no_proxy();
            }

            client_core.build().expect("Fail to build HTTP client.")
        };

        let retry_policy = ExponentialBackoff::builder().build_with_max_retries(self.retries);
        let retry_strategy = RetryTransientMiddleware::new_with_policy(retry_policy);
        let credentials = Arc::new(self.credentials);

        #[allow(unused_mut)]
        let mut client_builder = reqwest_middleware::ClientBuilder::new(client_raw.clone())
            .with(retry_strategy)
            .with(AuthMiddleware(credentials.clone()));

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

        let retry_policy = ExponentialBackoff::builder().build_with_max_retries(self.retries);
        let retry_strategy = RetryTransientMiddleware::new_with_policy(retry_policy);

        let client_uncached_builder = reqwest_middleware::ClientBuilder::new(client_raw)
            .with(retry_strategy)
            .with(AuthMiddleware(credentials));

        OroClient {
            registry: Arc::new(self.registry),
            client: client_builder.build(),
            client_uncached: client_uncached_builder.build(),
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn get_no_proxy_domain(&self) -> Option<NoProxy> {
        if let Some(ref no_proxy_conf) = self.no_proxy_domain {
            if !no_proxy_conf.is_empty() {
                return NoProxy::from_string(no_proxy_conf);
            }
        }

        NoProxy::from_env().or(None)
    }
}

#[derive(Clone, Debug)]
pub struct OroClient {
    pub(crate) registry: Arc<Url>,
    pub(crate) client: ClientWithMiddleware,
    pub(crate) client_uncached: ClientWithMiddleware,
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
