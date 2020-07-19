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
    version = clap::crate_version!(),
    setting = clap::AppSettings::ColoredHelp,
    setting = clap::AppSettings::DisableHelpSubcommand,
    setting = clap::AppSettings::DeriveDisplayOrder,
)]
pub struct Orogene {
    #[clap(global = true, about = "File to read configuration values from.", long)]
    config: Option<PathBuf>,
    #[clap(
        global = true,
        about = "Log output level (off, error, warn, info, debug, trace)",
        long,
        default_value = "warn",
        conflicts_with = "quiet"
    )]
    loglevel: log::LevelFilter,
    #[clap(global = true, about = "Disable all output", long, short = 'q', conflicts_with = "loglevel")]
    quiet: bool,
    #[clap(global = true, long, about = "Format output as JSON.")]
    json: bool,
    #[clap(subcommand)]
    subcommand: OroCmd,
}

impl Orogene {
    fn setup_logging(&self) -> Result<(), fern::InitError> {
        let fern = fern::Dispatch::new()
            .format(|out, message, record| {
                out.finish(format_args!(
                    "oro [{}][{}] {}",
                    record.level(),
                    record.target(),
                    message,
                ))
            })
            .chain(
                fern::Dispatch::new()
                    .level(if self.quiet {
                        log::LevelFilter::Off
                    } else {
                        self.loglevel
                    })
                    .chain(std::io::stderr()),
            );
        // TODO: later
        // if let Some(logfile) = ProjectDirs::from("", "", "orogene")
        //     .map(|d| d.data_dir().to_owned().join(format!("orogene-debug-{}.log", chrono::Local::now().to_rfc3339())))
        // {
        //     fern = fern.chain(
        //         fern::Dispatch::new()
        //         .level(log::LevelFilter::Trace)
        //         .chain(fern::log_file(logfile)?)
        //     )
        // }
        fern.apply()?;
        Ok(())
    }

    pub async fn load() -> Result<()> {
        let start = std::time::Instant::now();
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
        oro.setup_logging()?;
        oro.execute().await?;
        log::info!("Ran in {}s", start.elapsed().as_millis() as f32 / 1000.0);
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
        log::info!("Running command: {:?}", self.subcommand);
        match self.subcommand {
            OroCmd::Ping(ping) => ping.execute().await,
        }
    }
}

impl OroCommandLayerConfig for Orogene {
    fn layer_config(&mut self, args: ArgMatches, conf: OroConfig) -> Result<()> {
        match self.subcommand {
            OroCmd::Ping(ref mut ping) => {
                ping.layer_config(args.subcommand_matches("ping").unwrap().clone(), conf)
            }
        }
    }
}
