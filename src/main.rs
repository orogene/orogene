use anyhow::Result;

use orogene::Orogene;

#[async_std::main]
async fn main() -> Result<()> {
    Orogene::load().await
}

// TODO: put this somewhere for testing?
//
// use anyhow;
// use clap::Clap;
// use futures::stream::{futures_unordered::FuturesUnordered, StreamExt};
// use oro_client::OroClient;
// use rogga::cache;

// mod lib;
// #[derive(Clap)]
// struct Opts {
//     pkg: String,
//     version: String,
//     iterations: usize,
// }

// #[async_std::main]
// async fn main() -> anyhow::Result<()> {
//     let opts = Opts::parse();
//     let uri = format!("{}/-/{}-{}.tgz", opts.pkg, opts.pkg, opts.version);
//     let client = OroClient::new("https://registry.npmjs.org");
//     let mut futs = FuturesUnordered::new();
//     for i in 0..opts.iterations {
//         let path = format!("./cacache/{}", i);
//         futs.push(async {
//             Ok::<_, anyhow::Error>(cache::from_tarball(path, client.get(&uri).await?).await?)
//         });
//     }
//     while let Some(result) = futs.next().await {
//         result?;
//     }
//     Ok(())
// }
