use std::mem;
use std::path::{Path, PathBuf};

use async_compression::futures::bufread::GzipDecoder;
use async_std::io::{self, BufReader};
use async_std::prelude::*;
use async_tar::Archive;
use futures::AsyncRead;

use crate::error::{Result, RoggaError};

pub async fn extract_to_dir<P, R>(tarball: R, dir: P) -> Result<()>
where
    P: AsRef<Path>,
    R: AsyncRead + Unpin + Send + Sync,
{
    let dir = PathBuf::from(dir.as_ref());
    let takeme = dir.clone();
    async_std::task::spawn_blocking(move || {
        mkdirp::mkdirp(&takeme).map_err(|e| RoggaError::ExtractIoError(e, Some(takeme.clone())))
    })
    .await?;

    let decoder = GzipDecoder::new(BufReader::new(tarball));
    let ar = Archive::new(decoder);
    let mut entries = ar
        .clone()
        .entries()
        .map_err(|e| RoggaError::ExtractIoError(e, None))?;

    while let Some(file) = entries.next().await {
        let f = file.map_err(|e| RoggaError::ExtractIoError(e, None))?;
        let header = f.header();
        let path = dir.join(
            header
                .path()
                .map_err(|e| RoggaError::ExtractIoError(e, None))?
                .as_ref(),
        );
        if let async_tar::EntryType::Regular = header.entry_type() {
            let takeme = path.clone();

            async_std::task::spawn_blocking(move || {
                mkdirp::mkdirp(&takeme.parent().unwrap()).map_err(|e| {
                    RoggaError::ExtractIoError(e, Some(takeme.parent().unwrap().into()))
                })
            })
            .await?;
            let mut writer = async_std::fs::OpenOptions::new()
                .write(true)
                .create(true)
                .open(&path)
                .await
                .map_err(|e| RoggaError::ExtractIoError(e, Some(path.clone())))?;

            io::copy(f, async_std::io::BufWriter::new(&mut writer))
                .await
                .map_err(|e| RoggaError::ExtractIoError(e, Some(path.clone())))?;
        }
    }

    mem::drop(entries);
    let mut reader = ar
        .into_inner()
        .map_err(|_| RoggaError::MiscError("Failed to get inner Read".into()))?
        .into_inner()
        .into_inner();
    let mut buf = Vec::new();
    reader
        .read_to_end(&mut buf)
        .await
        .map_err(|e| RoggaError::ExtractIoError(e, None))?;

    tracing::trace!("Finished caching tarball contents from stream");
    Ok(())
}
