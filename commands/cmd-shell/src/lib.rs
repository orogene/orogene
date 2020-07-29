use anyhow::{Context, Result};
use async_trait::async_trait;
use clap::{ArgMatches, Clap};
use oro_command::OroCommand;
use oro_command::OroCommandLayerConfig;
use oro_config::OroConfig;
use oro_error_code::OroErrCode as Code;
use std::path::PathBuf;
use std::str::FromStr;

#[derive(Debug, Clap)]
pub struct ShellCmd {
    #[clap(long, default_value = "node")]
    node: String,

    #[clap(long)]
    data_dir: Option<PathBuf>,

    #[clap(multiple = true)]
    args: Vec<String>,

    #[clap(from_global)]
    loglevel: log::LevelFilter,
    #[clap(from_global)]
    json: bool,
    #[clap(from_global)]
    quiet: bool,
}

#[async_trait]
impl OroCommand for ShellCmd {
    async fn execute(self) -> Result<()> {
        dbg!(&self);
        Ok(())
    }
}

impl OroCommandLayerConfig for ShellCmd {
    fn layer_config(&mut self, args: ArgMatches, config: OroConfig) -> Result<()> {
        if args.occurrences_of("node") == 0 {
            if let Ok(val) = config.get_str("node") {
                self.node = String::from_str(&val)?;
            }
        }
        if args.occurrences_of("json") == 0 {
            if let Ok(val) = config.get_bool("json") {
                self.json = val;
            }
        }
        if args.occurrences_of("quiet") == 0 {
            if let Ok(val) = config.get_bool("quiet") {
                self.quiet = val;
            }
        }
        if args.occurrences_of("loglevel") == 0 {
            if let Ok(val) = config.get_str("loglevel") {
                self.loglevel = log::LevelFilter::from_str(&val)
                    .with_context(|| Code::OR1006("loglevel".into()))?;
            }
        }
        Ok(())
    }
}
