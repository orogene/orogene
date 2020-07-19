use anyhow::Result;
use async_trait::async_trait;
pub use clap::ArgMatches;
pub use oro_config::OroConfig;

// TODO: write a derive for this shit.
#[async_trait]
pub trait OroCommand {
    fn layer_config(&mut self, _matches: ArgMatches, _config: OroConfig) -> Result<()> {
        Ok(())
    }
    async fn execute(self) -> Result<()>;
}
