use anyhow;
use clap::Clap;
use futures::stream::{futures_unordered::FuturesUnordered, StreamExt};

use client::OroClient;

mod cache;
mod client;
mod integrity;

#[derive(Clap)]
struct Opts {
    pkg: String,
    version: String,
    iterations: usize,
}

#[async_std::main]
async fn main() -> anyhow::Result<()> {
    let opts = Opts::parse();
    let uri = format!("{}/-/{}-{}.tgz", opts.pkg, opts.pkg, opts.version);
    let client = OroClient::new("https://registry.npmjs.org");
    let mut futs = FuturesUnordered::new();
    for i in 0..opts.iterations {
        let path = format!("./cacache/{}", i);
        futs.push(async { cache::from_tarball(path, client.get(&uri).await?).await });
    }
    while let Some(result) = futs.next().await {
        result?;
    }
    Ok(())
}
