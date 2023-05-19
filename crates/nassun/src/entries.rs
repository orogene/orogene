use std::{borrow::Cow, pin::Pin, task::Poll};

use async_compression::futures::bufread::GzipDecoder;
use async_std::{io::BufReader, path::Path};
use async_tar_wasm::{Archive, Entry as TarEntry};
use futures::{AsyncRead, Stream};

pub use async_tar_wasm::Header;

use crate::error::{IoContext, Result};
use crate::tarball::Tarball;

#[cfg(not(target_arch = "wasm32"))]
type EntriesStream = Box<dyn Stream<Item = Result<Entry>> + Unpin + Send + Sync>;
#[cfg(target_arch = "wasm32")]
type EntriesStream = Box<dyn Stream<Item = Result<Entry>> + Unpin>;
/// Stream of tarball entries.
pub struct Entries(
    pub(crate) Archive<GzipDecoder<BufReader<Tarball>>>,
    pub(crate) EntriesStream,
);

impl Entries {
    pub fn into_inner(self) -> Archive<GzipDecoder<BufReader<Tarball>>> {
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
pub struct Entry(pub(crate) TarEntry<Archive<GzipDecoder<BufReader<Tarball>>>>);

impl Entry {
    /// Returns access to the header of this entry in the archive.
    ///
    /// This provides access to the metadata for this entry in the archive.
    pub fn header(&self) -> &Header {
        self.0.header()
    }

    /// Returns the path name for this entry.
    ///
    /// This method may fail if the pathname is not valid Unicode and this is
    /// called on a Windows platform.
    ///
    /// Note that this function will convert any \ characters to directory
    /// separators, and it will not always return the same value as
    /// self.header().path() as some archive formats have support for longer
    /// path names described in separate entries.
    ///
    /// It is recommended to use this method instead of inspecting the header
    /// directly to ensure that various archive formats are handled correctly.
    pub fn path(&self) -> Result<Cow<'_, Path>> {
        self.0
            .path()
            .io_context(|| "Failed to read path from tarball entry".into())
    }

    /// Writes this file to the specified location.
    ///
    /// This function will write the entire contents of this file into the
    /// location specified by dst. Metadata will also be propagated to the
    /// path dst.
    ///
    /// This function will create a file at the path dst, and it is required
    /// that the intermediate directories are created. Any existing file at
    /// the location dst will be overwritten.
    #[cfg(not(target_arch = "wasm32"))]
    pub async fn unpack(&mut self, dst: impl AsRef<Path>) -> Result<()> {
        let dst = dst.as_ref();
        self.0
            .unpack(dst)
            .await
            .io_context(|| format!("Failed to unpack tarball into {}.", dst.display()))?;
        Ok(())
    }

    /// Extracts this file under the specified path, avoiding security issues.
    ///
    /// This function will write the entire contents of this file into the
    /// location obtained by appending the path of this file in the archive to
    /// dst, creating any intermediate directories if needed. Metadata will
    /// also be propagated to the path dst. Any existing file at the location
    /// dst will be overwritten.
    ///
    /// This function carefully avoids writing outside of dst. If the file has
    /// a ‘..’ in its path, this function will skip it and return false.
    #[cfg(not(target_arch = "wasm32"))]
    pub async fn unpack_in(&mut self, dst: impl AsRef<Path>) -> Result<()> {
        let dst = dst.as_ref();
        self.0
            .unpack_in(dst)
            .await
            .io_context(|| format!("Failed to unpack tarball in {}.", dst.display()))?;
        Ok(())
    }
}

impl AsyncRead for Entry {
    fn poll_read(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &mut [u8],
    ) -> std::task::Poll<std::io::Result<usize>> {
        Pin::new(&mut self.0).poll_read(cx, buf)
    }
}
