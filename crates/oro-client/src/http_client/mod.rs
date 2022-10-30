use std::collections::HashMap;
use std::net::SocketAddr;
use std::{fmt::Debug, sync::Arc};

use async_h1::client;
use async_native_tls::TlsStream;
use async_std::net::TcpStream;
use async_std::sync::Mutex;
use async_trait::async_trait;
use deadpool::managed::Pool;
use http_types::StatusCode;
use surf::http::{Request, Response};
use surf::{Error, HttpClient};

use tcp::{TcpConnWrapper, TcpConnection};
use tls::{TlsConnWrapper, TlsConnection};

mod tcp;
mod tls;

// TODO: Move this to a parameter. This current number is based on a few
// random benchmarks and see whatever gave decent perf vs resource use.
static MAX_CONCURRENT_CONNECTIONS: usize = 50;

type HttpPool = HashMap<SocketAddr, Pool<TcpStream, std::io::Error>>;
type HttpsPool = HashMap<SocketAddr, Pool<TlsStream<TcpStream>, Error>>;

/// Async-h1 based connection-pooling HTTP client.
#[derive(Clone)]
pub struct PoolingClient {
    http_pool: Arc<Mutex<HttpPool>>,
    https_pool: Arc<Mutex<HttpsPool>>,
}

impl Debug for PoolingClient {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("H1Client")
    }
}

impl Default for PoolingClient {
    fn default() -> Self {
        Self::new()
    }
}

impl PoolingClient {
    /// Create a new instance.
    pub fn new() -> Self {
        Self {
            http_pool: Arc::new(Mutex::new(HashMap::new())),
            https_pool: Arc::new(Mutex::new(HashMap::new())),
        }
    }
}

#[async_trait]
impl HttpClient for PoolingClient {
    async fn send(&self, mut req: Request) -> Result<Response, Error> {
        let http_pool = self.http_pool.clone();
        let https_pool = self.https_pool.clone();
        req.insert_header("Connection", "keep-alive");

        // Insert host
        let host = req
            .url()
            .host_str()
            .ok_or_else(|| Error::from_str(StatusCode::BadRequest, "missing hostname"))?
            .to_string();

        let scheme = req.url().scheme();
        if scheme != "http" && scheme != "https" {
            return Err(Error::from_str(
                StatusCode::BadRequest,
                format!("invalid url scheme '{}'", scheme),
            ));
        }

        let addr = req
            .url()
            .socket_addrs(|| match req.url().scheme() {
                "http" => Some(80),
                "https" => Some(443),
                _ => None,
            })?
            .into_iter()
            .next()
            .ok_or_else(|| Error::from_str(StatusCode::BadRequest, "missing valid address"))?;

        tracing::trace!("> Scheme: {}", scheme);

        match scheme {
            "http" => {
                let mut hash = http_pool.lock().await;
                let pool = if let Some(pool) = hash.get(&addr) {
                    pool
                } else {
                    let manager = TcpConnection::new(addr);
                    let pool =
                        Pool::<TcpStream, std::io::Error>::new(manager, MAX_CONCURRENT_CONNECTIONS);
                    hash.insert(addr, pool);
                    hash.get(&addr).expect("oh COME ON")
                };
                let pool = pool.clone();
                std::mem::drop(hash);
                let stream = pool.get().await?;
                req.set_peer_addr(stream.peer_addr().ok());
                req.set_local_addr(stream.local_addr().ok());
                client::connect(TcpConnWrapper::new(stream), req).await
            }
            "https" => {
                let mut hash = https_pool.lock().await;
                let pool = if let Some(pool) = hash.get(&addr) {
                    pool
                } else {
                    let manager = TlsConnection::new(host.clone(), addr);
                    let pool = Pool::<TlsStream<TcpStream>, Error>::new(
                        manager,
                        MAX_CONCURRENT_CONNECTIONS,
                    );
                    hash.insert(addr, pool);
                    hash.get(&addr).expect("oh COME ON")
                };
                let pool = pool.clone();
                std::mem::drop(hash);
                let stream = pool.get().await.unwrap(); // TODO: remove unwrap
                req.set_peer_addr(stream.get_ref().peer_addr().ok());
                req.set_local_addr(stream.get_ref().local_addr().ok());

                client::connect(TlsConnWrapper::new(stream), req).await
            }
            _ => unreachable!(),
        }
    }
}
