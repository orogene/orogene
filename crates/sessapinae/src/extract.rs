use std::mem;
use std::path::{Path, PathBuf};

use async_compression::futures::bufread::GzipDecoder;
use async_tar::Archive;
use oro_common::smol::io::{AsyncReadExt, BufReader};
use oro_common::smol::stream::StreamExt;
use oro_common::{
    futures::AsyncRead,
    smol::{self, fs, io},
    tracing,
};

use crate::error::SessError;

pub async fn extract_to_dir<P, R>(tarball: R, dir: P) -> Result<(), SessError>
where
    P: AsRef<Path>,
    R: AsyncRead + Unpin + Send + Sync,
{
    let dir = PathBuf::from(dir.as_ref());
    let takeme = dir.clone();
    smol::unblock(move || mkdirp::mkdirp(&takeme))
        .await
        .map_err(|e| SessError::ExtractIoError(e, Some(dir.clone())))?;

    let decoder = GzipDecoder::new(BufReader::new(tarball));
    let ar = Archive::new(decoder);
    let mut entries = ar
        .clone()
        .entries()
        .map_err(|e| SessError::ExtractIoError(e, None))?;

    while let Some(file) = entries.next().await {
        let f = file.map_err(|e| SessError::ExtractIoError(e, None))?;
        let header = f.header();
        let path = dir.join(
            header
                .path()
                .map_err(|e| SessError::ExtractIoError(e, None))?
                .as_ref(),
        );
        if let async_tar::EntryType::Regular = header.entry_type() {
            let takeme = path.clone();

            smol::unblock(move || mkdirp::mkdirp(&takeme.parent().unwrap()))
                .await
                .map_err(|e| SessError::ExtractIoError(e, Some(path.parent().unwrap().into())))?;
            let mut writer = fs::OpenOptions::new()
                .write(true)
                .create(true)
                .open(&path)
                .await
                .map_err(|e| SessError::ExtractIoError(e, Some(path.clone())))?;

            io::copy(f, io::BufWriter::new(&mut writer))
                .await
                .map_err(|e| SessError::ExtractIoError(e, Some(path.clone())))?;
        }
    }

    mem::drop(entries);
    let mut reader = ar
        .into_inner()
        .map_err(|_| SessError::MiscError("Failed to get inner Read".into()))?
        .into_inner()
        .into_inner();
    let mut buf = Vec::new();
    reader
        .read_to_end(&mut buf)
        .await
        .map_err(|e| SessError::ExtractIoError(e, None))?;

    tracing::trace!("Finished caching tarball contents from stream");
    Ok(())
}
