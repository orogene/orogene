use std::collections::HashMap;
use std::net::SocketAddr;
use std::pin::Pin;
use std::{fmt::Debug, sync::Arc};

use async_h1::client;
use async_native_tls::TlsStream;
use async_std::net::TcpStream;
use async_std::sync::Mutex;
use async_trait::async_trait;
use deadpool::managed::{Manager, Object, Pool, RecycleResult};
use futures::future::BoxFuture;
use futures::io::{AsyncRead, AsyncWrite};
use futures::task::{Poll, Context};
use http_client::{Error, HttpClient, Request, Response};
use http_types::StatusCode;

pub struct TcpConnWrapper {
    conn: Object<TcpStream, std::io::Error>,
}
impl TcpConnWrapper {
    pub fn new(conn: Object<TcpStream, std::io::Error>) -> Self {
        Self { conn }
    }
}

impl AsyncRead for TcpConnWrapper {
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut Context,
        buf: &mut [u8],
    ) -> Poll<Result<usize, std::io::Error>> {
        Pin::new(&mut *self.conn).poll_read(cx, buf)
    }
}

impl AsyncWrite for TcpConnWrapper {
    fn poll_write(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<std::io::Result<usize>> {
        let amt = futures::ready!(Pin::new(&mut *self.conn).poll_write(cx, buf))?;
        Poll::Ready(Ok(amt))
    }

    fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        Pin::new(&mut *self.conn).poll_flush(cx)
    }

    fn poll_close(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        Pin::new(&mut *self.conn).poll_close(cx)
    }
}


#[derive(Clone, Debug)]
pub struct TcpConnection {
    addr: SocketAddr,
}
impl TcpConnection {
    pub fn new(addr: SocketAddr) -> Self {
        Self { addr }
    }
}

#[async_trait]
impl Manager<TcpStream, std::io::Error> for TcpConnection {
    async fn create(&self) -> Result<TcpStream, std::io::Error> {
        Ok(TcpStream::connect(self.addr).await?)
    }

    async fn recycle(&self, _conn: &mut TcpStream) -> RecycleResult<std::io::Error> {
        Ok(())
    }
}

#[derive(Clone, Debug)]
pub struct TlsConnection {
    host: String,
    addr: SocketAddr,
}
impl TlsConnection {
    pub fn new(host: String, addr: SocketAddr) -> Self {
        Self { host, addr }
    }
}

pub struct TlsConnWrapper {
    conn: Object<TlsStream<TcpStream>, Error>,
}
impl TlsConnWrapper {
    pub fn new(conn: Object<TlsStream<TcpStream>, Error>) -> Self {
        Self { conn }
    }
}

impl AsyncRead for TlsConnWrapper {
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut Context,
        buf: &mut [u8],
    ) -> Poll<Result<usize, std::io::Error>> {
        Pin::new(&mut *self.conn).poll_read(cx, buf)
    }
}

impl AsyncWrite for TlsConnWrapper {
    fn poll_write(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<std::io::Result<usize>> {
        let amt = futures::ready!(Pin::new(&mut *self.conn).poll_write(cx, buf))?;
        Poll::Ready(Ok(amt))
    }

    fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        Pin::new(&mut *self.conn).poll_flush(cx)
    }

    fn poll_close(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        Pin::new(&mut *self.conn).poll_close(cx)
    }
}
#[async_trait]
impl Manager<TlsStream<TcpStream>, Error> for TlsConnection {
    async fn create(&self) -> Result<TlsStream<TcpStream>, Error> {
        log::trace!("Making new TLS connection to {}", self.host);
        let raw_stream = async_std::net::TcpStream::connect(self.addr).await?;
        let stream = async_native_tls::connect(&self.host, raw_stream).await?;
        Ok(stream)
    }

    async fn recycle(&self, _conn: &mut TlsStream<TcpStream>) -> RecycleResult<Error> {
        log::trace!("Recycling connection to {}", self.host);
        Ok(())
    }
}
/// Async-h1 based HTTP Client.
#[derive(Clone)]
pub struct H1Client {
    http_pool: Arc<Mutex<HashMap<SocketAddr, Pool<TcpStream, std::io::Error>>>>,
    https_pool: Arc<Mutex<HashMap<SocketAddr, Pool<TlsStream<TcpStream>, Error>>>>,
}

impl Debug for H1Client {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("H1Client")
    }
}

impl Default for H1Client {
    fn default() -> Self {
        Self::new()
    }
}

impl H1Client {
    /// Create a new instance.
    pub fn new() -> Self {
        Self {
            http_pool: Arc::new(Mutex::new(HashMap::new())),
            https_pool: Arc::new(Mutex::new(HashMap::new())),
        }
    }
}

impl HttpClient for H1Client {
    fn send(&self, mut req: Request) -> BoxFuture<'static, Result<Response, Error>> {
        let http_pool = self.http_pool.clone();
        let https_pool = self.https_pool.clone();
        Box::pin(async move {
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

            log::trace!("> Scheme: {}", scheme);

            match scheme {
                "http" => {
                    let mut hash = http_pool.lock().await;
                    let pool = if let Some(pool) = hash.get(&addr) {
                        pool
                    } else {
                        let manager = TcpConnection::new(addr);
                        let pool = Pool::<TcpStream, std::io::Error>::new(manager, 25);
                        hash.insert(addr, pool);
                        hash.get(&addr).expect("oh COME ON")
                    };
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
                        let pool = Pool::<TlsStream<TcpStream>, Error>::new(manager, 100);
                        hash.insert(addr, pool);
                        hash.get(&addr).expect("oh COME ON")
                    };
                    let stream = pool.get().await.unwrap(); // TODO: remove unwrap
                    // println!("Got https stream to {} (status: {:#?})", host, pool.status());
                    req.set_peer_addr(stream.get_ref().peer_addr().ok());
                    req.set_local_addr(stream.get_ref().local_addr().ok());

                    client::connect(TlsConnWrapper::new(stream), req).await
                }
                _ => unreachable!(),
            }
        })
    }
}
