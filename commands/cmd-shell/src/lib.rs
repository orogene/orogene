use anyhow::Result;
use async_trait::async_trait;
use clap::Clap;
use oro_command::OroCommand;
use std::path::PathBuf;

#[derive(Debug, Clap, OroCommand)]
pub struct ShellCmd {
    #[clap(long, default_value = "node")]
    node: String,

    #[clap(from_global)]
    data_dir: Option<PathBuf>,

    #[clap(from_global)]
    loglevel: log::LevelFilter,

    #[clap(from_global)]
    json: bool,

    #[clap(from_global)]
    quiet: bool,

    #[clap(multiple = true)]
    #[oro_config(ignore)]
    args: Vec<String>,
}

#[async_trait]
impl OroCommand for ShellCmd {
    async fn execute(self) -> Result<()> {
        dbg!(&self);
        Ok(())
    }
}
