use isahc::config::Configurable;

pub use isahc::*;

#[derive(Debug, Clone)]
pub struct ApiClient {
    inner: isahc::HttpClient,
}

impl ApiClient {
    pub fn new() -> Self {
        let client = isahc::HttpClient::builder()
            .max_connections(15)
            .connection_cache_size(15)
            .default_header("User-Agent", "Orogene/0.1.0") // TODO: Look up version
            .redirect_policy(isahc::config::RedirectPolicy::Limit(5))
            .build()
            .expect("Failed to build HTTP client");
        Self { inner: client }
    }

    #[inline]
    pub fn send<B>(&self, request: Request<B>) -> ResponseFuture<'_>
    where
        B: Into<AsyncBody>,
    {
        self.inner.send_async(request)
    }
}

impl Default for ApiClient {
    fn default() -> Self {
        Self::new()
    }
}
