use anyhow;
use clap::Clap;

use client::OroClient;

mod cache;
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
    for i in 0..10 {
        let path = format!("./cacache/{}", i);
        tasks.push(async { cache::from_tarball(path, client.get(&uri).await?).await });
    }
    futures::future::try_join_all(tasks).await?;
    Ok(())
}
