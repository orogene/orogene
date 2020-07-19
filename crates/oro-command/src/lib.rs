use anyhow::Result;
use async_trait::async_trait;
pub use clap::ArgMatches;
pub use oro_config::OroConfig;

pub use oro_command_derive::*;

// TODO: write a derive for this shit.
#[async_trait]
pub trait OroCommand {
    async fn execute(self) -> Result<()>;
}

pub trait OroCommandLayerConfig {
    fn layer_config(&mut self, _matches: ArgMatches, _config: OroConfig) -> Result<()> {
        Ok(())
    }
}
