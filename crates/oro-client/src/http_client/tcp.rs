use std::fmt::Debug;
use std::net::SocketAddr;
use std::pin::Pin;

use async_std::net::TcpStream;
use async_trait::async_trait;
use deadpool::managed::{Manager, Object, RecycleResult};
use futures::io::{AsyncRead, AsyncWrite};
use futures::task::{Context, Poll};

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
