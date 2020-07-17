use std::collections::HashMap;

use anyhow;
use async_compression::futures::bufread::GzipDecoder;
use async_std::prelude::*;
use async_tar::Archive;
use cacache::Writer;
use clap::Clap;
use futures;

use client::{OroClient, Response};

mod client;

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
    for i in 0..20 {
        let path = format!("./cacache/{}", i);
        tasks.push(async { cache_contents(path, client.get(&uri).await?).await });
    }
    futures::future::try_join_all(tasks).await?;
    Ok(())
}

async fn cache_contents(path: impl AsRef<std::path::Path>, resp: Response) -> anyhow::Result<()> {
    use async_std::fs;
    use async_std::io::BufReader;
    let path = std::path::PathBuf::from(path.as_ref());

    fs::create_dir_all(&path).await?;

    let decoder = GzipDecoder::new(BufReader::new(resp));
    let mut ar = Archive::new(decoder);
    let mut entries = ar.entries()?;
    let mut n: u32 = 0;
    let start = std::time::Instant::now();

    while let Some(file) = entries.next().await {
        n += 1;
        let mut f = file?;
        let size = f.header().size()?;
        let path = path.clone();
        if size > 0 && size < 10 * 1024 * 1024 {
            use std::fs::OpenOptions;
            let mut buf = Vec::new();
            f.read_to_end(&mut buf).await?;
            async_std::task::spawn_blocking(move || {
                let fd = OpenOptions::new()
                    .read(true)
                    .write(true)
                    .create(true)
                    .open(path.join(format!("{}", n)))?;
                fd.set_len(size)?;
                let mut mmap = unsafe { memmap::MmapMut::map_mut(&fd)? };
                mmap.copy_from_slice(&buf);
                mmap.flush_async()?;
                Ok::<(), anyhow::Error>(())
            })
            .await?;
        } else {
            use async_std::fs::OpenOptions;
            let fd = OpenOptions::new()
                .write(true)
                .create(true)
                .open(path.join(format!("{}", n)))
                .await?;
            async_std::io::copy(f, fd).await?;
        }
    }
    Ok(())
}
