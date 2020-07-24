use anyhow::Result;
use async_trait::async_trait;
use clap::Clap;
use oro_classic_resolver::ClassicResolver;
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
        let manifest = Rogga::new(&self.registry)
            .arg_package(&self.pkg)?
            .resolve_with(ClassicResolver::new())
            .await?
            .manifest()
            .await?;
        if self.json {
            println!("{}", serde_json::to_string_pretty(&manifest)?);
        } else {
            println!("{:#?}", manifest);
        }
        Ok(())
    }
}
