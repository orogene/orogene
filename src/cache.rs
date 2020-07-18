use std::collections::HashMap;

use anyhow;
use async_compression::futures::bufread::GzipDecoder;
use async_std::prelude::*;
use async_tar::Archive;
use bincode;
use cacache::WriteOpts;
use futures::{self, io::AsyncBufRead};

use crate::integrity::AsyncIntegrity;

pub async fn from_tarball<P, R>(cache: P, tarball: R) -> anyhow::Result<()>
where
    P: AsRef<std::path::Path>,
    R: AsyncBufRead + Unpin + Send + Sync,
{
    use async_std::fs;
    use async_std::io::{self, BufReader};
    let path = std::path::PathBuf::from(cache.as_ref());

    fs::create_dir_all(&path).await?;

    let sri_builder = AsyncIntegrity::new(tarball);
    let decoder = GzipDecoder::new(BufReader::new(sri_builder));
    let mut ar = Archive::new(decoder);
    let mut entries = ar.entries()?;
    let mut entry_hash = HashMap::new();

    while let Some(file) = entries.next().await {
        let f = file?;
        let size = f.header().size()?;
        let path = path.clone();
        let key = f.path()?.display().to_string();
        let mut writer = WriteOpts::new()
            .size(size as usize)
            .open_hash(&path)
            .await?;
        io::copy(f, async_std::io::BufWriter::new(&mut writer)).await?;
        let sri = writer.commit().await?;
        entry_hash.insert(key, (sri, size));
    }
    std::mem::drop(entries);
    let sri = ar
        .into_inner()
        .map_err(|_| anyhow::anyhow!("failed to get inner reader"))?
        .into_inner()
        .into_inner()
        .into_inner()
        .result();
    cacache::write(
        &path,
        format!("orogene::pkg::{}", sri),
        bincode::serialize(&entry_hash)?,
    )
    .await?;
    Ok(())
}
