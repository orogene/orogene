use std::pin::Pin;
use std::task::Poll;

use async_compat::Compat;
use async_compression::futures::bufread::GzipDecoder;
use async_tar::{Archive, Entry as TarEntry};
use futures::Stream;
use tokio::io::{AsyncRead, BufReader, ReadBuf};

pub use async_tar::Header;

use crate::{error::Result, Tarball};

#[cfg(not(target_arch = "wasm32"))]
type EntriesStream = Box<dyn Stream<Item = Result<Entry>> + Unpin + Send + Sync>;
#[cfg(target_arch = "wasm32")]
type EntriesStream = Box<dyn Stream<Item = Result<Entry>> + Unpin>;
/// Stream of tarball entries.
pub struct Entries(
    pub(crate) Archive<GzipDecoder<Compat<BufReader<Tarball>>>>,
    pub(crate) EntriesStream,
);

impl Entries {
    pub fn into_inner(self) -> Archive<GzipDecoder<Compat<BufReader<Tarball>>>> {
        self.0
    }
}

impl Stream for Entries {
    type Item = Result<Entry>;

    fn poll_next(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Option<Self::Item>> {
        Poll::Ready(futures::ready!(Pin::new(&mut self.1).poll_next(cx)))
    }
}

/// Entry in a package tarball.
pub struct Entry(pub(crate) Compat<TarEntry<Archive<GzipDecoder<Compat<BufReader<Tarball>>>>>>);

impl Entry {
    /// Returns access to the header of this entry in the archive.
    ///
    /// This provides access to the metadata for this entry in the archive.
    pub fn header(&self) -> &Header {
        self.0.get_ref().header()
    }
}

impl AsyncRead for Entry {
    fn poll_read(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> std::task::Poll<std::io::Result<()>> {
        Pin::new(&mut self.0).poll_read(cx, buf)
    }
}
