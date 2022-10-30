use std::fmt::Debug;
use std::net::SocketAddr;
use std::pin::Pin;

use async_native_tls::TlsStream;
use async_std::net::TcpStream;
use async_trait::async_trait;
use deadpool::managed::{Manager, Object, RecycleResult};
use futures::io::{AsyncRead, AsyncWrite};
use futures::task::{Context, Poll};
use http_client::Error;

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
        tracing::trace!("Creating new socket to {:?}", self.addr);
        let raw_stream = async_std::net::TcpStream::connect(self.addr).await?;
        let stream = async_native_tls::connect(&self.host, raw_stream).await?;
        Ok(stream)
    }

    async fn recycle(&self, _conn: &mut TlsStream<TcpStream>) -> RecycleResult<Error> {
        Ok(())
    }
}
