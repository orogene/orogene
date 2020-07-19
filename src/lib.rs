use std::path::PathBuf;

use anyhow::Result;
use async_trait::async_trait;
use clap::{Clap, FromArgMatches, IntoApp};
use oro_command::{ArgMatches, OroCommand, OroCommandLayerConfig};
use oro_config::{OroConfig, OroConfigOptions};

use cmd_ping::PingCmd;

pub use oro_error_code::OroErrCode as Code;

#[derive(Debug, Clap)]
#[clap(
    author = "Kat Marchán <kzm@zkat.tech>",
    about = "Manage your NPM packages.",
    setting = clap::AppSettings::ColoredHelp,
    setting = clap::AppSettings::DisableHelpSubcommand,
    setting = clap::AppSettings::DeriveDisplayOrder,
)]
pub struct Orogene {
    #[clap(about = "File to read configuration values from.", long, global = true)]
    config: Option<PathBuf>,
    #[clap(subcommand)]
    subcommand: OroCmd,
}

impl Orogene {
    pub async fn load() -> Result<()> {
        let clp = Orogene::into_app();
        let matches = clp.get_matches();
        let mut oro = Orogene::from_arg_matches(&matches);
        let cfg = if let Some(file) = &oro.config {
            OroConfigOptions::new()
                .global_config_file(Some(file.clone()))
                .load()?
        } else {
            OroConfigOptions::new().load()?
        };
        oro.layer_config(matches, cfg)?;
        oro.execute().await?;
        Ok(())
    }
}

#[derive(Debug, Clap)]
pub enum OroCmd {
    #[clap(
        about = "Ping the registry",
        setting = clap::AppSettings::ColoredHelp,
        setting = clap::AppSettings::DisableHelpSubcommand,
        setting = clap::AppSettings::DeriveDisplayOrder,
    )]
    Ping(PingCmd),
    // #[clap(
    //     about = "Execute a new wrapped `node` shell.",
    //     alias = "sh",
    //     setting = clap::AppSettings::TrailingVarArg
    // )]
    // Shell(ShellCmd),
}

#[async_trait]
impl OroCommand for Orogene {
    async fn execute(self) -> Result<()> {
        match self.subcommand {
            // DsCmd::Config(cfg) => cfg.execute().await,
            OroCmd::Ping(ping) => ping.execute().await,
            // DsCmd::Shell(shell) => shell.execute().await,
        }
    }
}

#[async_trait]
impl OroCommandLayerConfig for Orogene {
    fn layer_config(&mut self, args: ArgMatches, conf: OroConfig) -> Result<()> {
        match self.subcommand {
            // DsCmd::Config(ref mut cfg) => {
            //     cfg.layer_config(args.subcommand_matches("config").unwrap().clone(), conf)
            // }
            OroCmd::Ping(ref mut ping) => {
                ping.layer_config(args.subcommand_matches("ping").unwrap().clone(), conf)
            } // DsCmd::Shell(ref mut shell) => {
              //     shell.layer_config(args.subcommand_matches("shell").unwrap().clone(), conf)
              // }
        }
    }
}
