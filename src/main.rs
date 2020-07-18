use std::collections::HashMap;

use anyhow;
use async_compression::futures::bufread::GzipDecoder;
use async_std::prelude::*;
use async_tar::Archive;
use bincode;
use cacache::WriteOpts;
use clap::Clap;
use futures;

use client::{OroClient, Response};
use integrity::AsyncIntegrity;

mod client;
mod integrity;

#[derive(Clap)]
struct Opts {
    pkg: String,
    version: String,
}

#[async_std::main]
async fn main() -> anyhow::Result<()> {
    let opts = Opts::parse();
    let uri = format!("{}/-/{}-{}.tgz", opts.pkg, opts.pkg, opts.version);
    let client = OroClient::new("https://registry.npmjs.org");
    let mut tasks = Vec::new();
    for i in 0..1 {
        let path = format!("./cacache/{}", i);
        tasks.push(async { cache_contents(path, client.get(&uri).await?).await });
    }
    futures::future::try_join_all(tasks).await?;
    Ok(())
}

async fn cache_contents(path: impl AsRef<std::path::Path>, resp: Response) -> anyhow::Result<()> {
    use async_std::fs;
    use async_std::io::{self, BufReader};
    let path = std::path::PathBuf::from(path.as_ref());

    fs::create_dir_all(&path).await?;

    let sri_builder = AsyncIntegrity::new(resp);
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
        entry_hash.insert(key, sri);
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
