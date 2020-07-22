use anyhow::Result;
use async_trait::async_trait;
use clap::Clap;
use oro_command::OroCommand;
use rogga::Rogga;
use url::Url;

#[derive(Debug, Clap, OroCommand)]
pub struct ViewCmd {
    #[clap(
        about = "Registry to get package data from.",
        default_value = "https://registry.npmjs.org",
        long
    )]
    registry: Url,
    #[clap(from_global)]
    json: bool,
    #[clap(about = "Package spec to look up")]
    pkg: String,
}

#[async_trait]
impl OroCommand for ViewCmd {
    async fn execute(self) -> Result<()> {
        let rogga = Rogga::new(&self.registry);
        let req = rogga.arg_package(&self.pkg)?;
        let packument = req.packument().await?;
        println!("{:#?}", packument);
        Ok(())
    }
}
