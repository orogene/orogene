use anyhow::anyhow;
use async_compression::futures::bufread::GzipDecoder;
use async_std::prelude::*;
use async_tar::Archive;
use clap::Clap;
use surf::Client;

#[derive(Clap)]
struct Opts {
    pkg: String,
    version: String,
}

#[async_std::main]
async fn main() -> anyhow::Result<()> {
    let opts = Opts::parse();
    let uri = format!("https://registry.npmjs.org/{}/-/{}-{}.tgz", opts.pkg, opts.pkg, opts.version);
    let reader = surf::get(uri).await.map_err(|err| anyhow!("bad time"))?;
    let decoder = GzipDecoder::new(reader);
    let mut ar = Archive::new(decoder);
    let mut entries = ar.entries()?;
    while let Some(file) = entries.next().await {
        let f = file?;
        f.path()?.display();
    }
    Ok(())
}
