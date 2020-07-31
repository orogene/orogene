use std::collections::HashMap;

use async_compression::futures::bufread::GzipDecoder;
use async_std::prelude::*;
use async_tar::Archive;
use cacache::WriteOpts;
use futures::{self, io::AsyncRead};
use ssri::Integrity;

use crate::error::{Error, Internal, Result};
use crate::integrity::AsyncIntegrity;

struct TarFile<R: async_std::io::Read + std::marker::Unpin>(async_tar::Entry<R>);

impl<R: async_std::io::Read + std::marker::Unpin> AsyncRead for TarFile<R> {
    fn poll_read(
        mut self: std::pin::Pin<&mut Self>,
        ctxt: &mut std::task::Context<'_>,
        buf: &mut [u8]
    ) -> std::task::Poll<std::result::Result<usize, std::io::Error>> {
        std::pin::Pin::new(&mut self.0).poll_read(ctxt, buf)
    }
}

impl<R: async_std::io::Read + std::marker::Unpin> cacache::FileLike for TarFile<R> {
    fn path(&self) -> cacache::Result<String> {
        Ok(self.0.path().unwrap().display().to_string())
    }

    fn size(&self) -> cacache::Result<usize> {
        let header = self.0.header();
        let mode = header.mode().unwrap();
        Ok(header.size().unwrap() as usize)
    }

    fn mode(&self) -> cacache::Result<u32> {
        let header = self.0.header();
        Ok(header.mode().unwrap())
    }
}

pub async fn from_tarball<P, R: 'static>(cache: P, tarball: R) -> Result<Integrity>
where
    P: AsRef<std::path::Path>,
    R: AsyncRead + Unpin + Send + Sync,
{
    use async_std::io::{self, BufReader};
    let path = std::path::PathBuf::from(cache.as_ref());
    let sri_builder = AsyncIntegrity::new(BufReader::new(tarball));
    let decoder = GzipDecoder::new(BufReader::new(sri_builder));
    let ar = Archive::new(decoder);

    Ok(cacache::write_entries(cache, ar.entries().to_internal()?.map(
        |result| result.map(|entry| TarFile(entry))
    )).await.to_internal()?)
}

pub async fn to_node_modules<P, R>(cache: P, tarball: R) -> Result<()>
where
    P: AsRef<std::path::Path>,
    R: AsyncRead + Unpin + Send + Sync,
{
    use async_std::io::{self, BufReader};
    let cache = std::path::PathBuf::from(cache.as_ref());
    let takeme = cache.clone();
    async_std::task::spawn_blocking(move || mkdirp::mkdirp(&takeme).to_internal()).await?;

    let decoder = GzipDecoder::new(BufReader::new(tarball));
    let ar = Archive::new(decoder);
    let mut entries = ar.clone().entries().to_internal()?;

    while let Some(file) = entries.next().await {
        let f = file.to_internal()?;
        let header = f.header();
        let path = cache.join(header.path().to_internal()?.as_ref());
        if let async_tar::EntryType::Regular = header.entry_type() {
            let takeme = path.clone();

            async_std::task::spawn_blocking(move || {
                mkdirp::mkdirp(&takeme.parent().unwrap())
                    .to_internal()
                    .with_context(|| String::from("Trying to create a file's parent dir"))
            })
            .await?;
            let mut writer = async_std::fs::OpenOptions::new()
                .write(true)
                .create(true)
                .open(&path)
                .await
                .to_internal()
                .with_context(|| format!("Trying to write {}", path.display()))?;

            io::copy(f, async_std::io::BufWriter::new(&mut writer))
                .await
                .to_internal()?;
        }
    }

    std::mem::drop(entries);
    let mut reader = ar
        .into_inner()
        .map_err(|_| Error::MiscError("Failed to get inner Read".into()))
        .to_internal()?
        .into_inner()
        .into_inner();
    let mut buf = Vec::new();
    reader.read_to_end(&mut buf).await.to_internal()?;

    log::trace!("Finished caching tarball contents from stream");
    Ok(())
}

pub async fn tarball_itself<P, R>(cache: P, tarball: R) -> Result<Integrity>
where
    P: AsRef<std::path::Path>,
    R: AsyncRead + Unpin + Send + Sync,
{
    use async_std::io::{self, BufReader};
    let path = std::path::PathBuf::from(cache.as_ref());

    let reader = BufReader::new(tarball);
    let mut writer = WriteOpts::new().open_hash(&path).await.to_internal()?;

    io::copy(reader, async_std::io::BufWriter::new(&mut writer))
        .await
        .to_internal()?;

    let sri = writer.commit().await.to_internal()?;

    Ok(
        cacache::write(&path, format!("orogene::pkg::{}", sri.to_string()), b"")
            .await
            .to_internal()?,
    )
}

pub async fn tarball_to_mem<P, R>(cache: P, tarball: R) -> Result<Integrity>
where
    P: AsRef<std::path::Path>,
    R: AsyncRead + Unpin + Send + Sync,
{
    use async_std::io::BufReader;
    let path = std::path::PathBuf::from(cache.as_ref());

    let mut reader = BufReader::new(tarball);
    let mut buf = Vec::new();
    reader.read_to_end(&mut buf).await.to_internal()?;
    let sri = Integrity::from(&buf);

    Ok(
        cacache::write(&path, format!("orogene::pkg::{}", sri.to_string()), b"")
            .await
            .to_internal()?,
    )
}
