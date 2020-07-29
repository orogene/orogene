use std::collections::HashMap;

use async_compression::futures::bufread::GzipDecoder;
use async_std::prelude::*;
use async_tar::Archive;
use cacache::WriteOpts;
use futures::{self, io::AsyncRead};
use ssri::Integrity;

use crate::error::{Error, Internal, Result};
use crate::integrity::AsyncIntegrity;

pub async fn from_tarball<P, R>(cache: P, tarball: R) -> Result<Integrity>
where
    P: AsRef<std::path::Path>,
    R: AsyncRead + Unpin + Send + Sync,
{
    use async_std::io::{self, BufReader};
    let path = std::path::PathBuf::from(cache.as_ref());

    let sri_builder = AsyncIntegrity::new(BufReader::new(tarball));
    let decoder = GzipDecoder::new(BufReader::new(sri_builder));
    let ar = Archive::new(decoder);
    let mut entries = ar.clone().entries().to_internal()?;
    let mut entry_hash = HashMap::new();

    while let Some(file) = entries.next().await {
        let f = file.to_internal()?;
        let header = f.header();
        let mode = header.mode().to_internal()?;
        let size = header.size().to_internal()?;
        let path = path.clone();
        let key = f.path().to_internal()?.display().to_string();

        let mut writer = WriteOpts::new()
            .size(size as usize)
            .open_hash(&path)
            .await
            .to_internal()?;

        io::copy(f, async_std::io::BufWriter::new(&mut writer))
            .await
            .to_internal()?;

        let sri = writer.commit().await.to_internal()?;
        entry_hash.insert(key, (sri, size, mode));
    }

    std::mem::drop(entries);
    let (sri, mut reader) = ar
        .into_inner()
        .map_err(|_| Error::MiscError("Failed to get inner Read".into()))
        .to_internal()?
        .into_inner()
        .into_inner()
        .inner_result();
    let mut buf = Vec::new();
    reader.read_to_end(&mut buf).await.to_internal()?;

    log::trace!("Finished caching tarball contents from stream");
    Ok(cacache::write(
        &path,
        format!("orogene::pkg::{}", sri.to_string()),
        bincode::serialize(&entry_hash).to_internal()?,
    )
    .await
    .to_internal()?)
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
