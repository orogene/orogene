use std::path::PathBuf;

use async_trait::async_trait;
use clap::{ArgMatches, CommandFactory, FromArgMatches as _, Parser, Subcommand};
use directories::ProjectDirs;
use miette::{IntoDiagnostic, Result};
use oro_command::OroCommand;
use oro_config::{OroConfig, OroConfigLayer, OroConfigOptions};
use tracing_subscriber::{filter::LevelFilter, fmt, prelude::*, EnvFilter};

use cmd_ping::PingCmd;
use cmd_resolve::ResolveCmd;
use cmd_view::ViewCmd;

#[derive(Debug, Parser)]
#[command(author, version, about, long_about = None)]
#[command(propagate_version = true)]
pub struct Orogene {
    /// Package path to operate on
    #[arg(global = true, long = "root")]
    root: Option<PathBuf>,

    /// File to read configuration values from.
    #[arg(global = true, long)]
    config: Option<PathBuf>,

    /// Log output level/directive. Supports plain loglevels (off, error,
    /// warn, info, debug, trace) as well as more advanced directives in the
    /// format `target[span{field=value}]=level`.
    #[clap(global = true, long, default_value = "warn")]
    loglevel: String,

    /// Disable all output
    #[arg(global = true, long, short)]
    quiet: bool,

    /// Format output as JSON.
    #[arg(global = true, long)]
    json: bool,

    #[command(subcommand)]
    subcommand: OroCmd,
}

impl Orogene {
    fn setup_logging(&self) -> Result<()> {
        tracing_subscriber::registry()
            .with(fmt::layer())
            .with(
                EnvFilter::builder()
                    .with_default_directive(if self.quiet {
                        LevelFilter::OFF.into()
                    } else {
                        self.loglevel.parse().into_diagnostic()?
                    })
                    .from_env_lossy(),
            )
            .init();
        Ok(())
    }

    pub async fn load() -> Result<()> {
        let start = std::time::Instant::now();
        let matches = Orogene::command().get_matches();
        let mut oro = Orogene::from_arg_matches(&matches).into_diagnostic()?;
        let cfg = if let Some(file) = &oro.config {
            OroConfigOptions::new()
                .global_config_file(Some(file.clone()))
                .load()?
        } else {
            OroConfigOptions::new()
                .global_config_file(
                    ProjectDirs::from("", "", "orogene")
                        .map(|d| d.config_dir().to_owned().join("ororc.toml")),
                )
                .pkg_root(oro.root.clone())
                .load()?
        };
        oro.layer_config(&matches, &cfg)?;
        oro.setup_logging()?;
        oro.execute().await?;
        tracing::info!("Ran in {}s", start.elapsed().as_millis() as f32 / 1000.0);
        Ok(())
    }
}

#[derive(Debug, Subcommand)]
pub enum OroCmd {
    /// Ping the registry.
    Ping(PingCmd),

    /// Resolve a package tree and save the lockfile to the project directory.
    Resolve(ResolveCmd),

    /// Get information about a package.
    View(ViewCmd),
}

#[async_trait]
impl OroCommand for Orogene {
    async fn execute(self) -> Result<()> {
        tracing::info!("Running command: {:#?}", self.subcommand);
        match self.subcommand {
            OroCmd::Ping(ping) => ping.execute().await,
            OroCmd::Resolve(resolve) => resolve.execute().await,
            OroCmd::View(view) => view.execute().await,
        }
    }
}

impl OroConfigLayer for Orogene {
    fn layer_config(&mut self, args: &ArgMatches, conf: &OroConfig) -> Result<()> {
        match self.subcommand {
            OroCmd::Ping(ref mut ping) => {
                ping.layer_config(args.subcommand_matches("ping").unwrap(), conf)
            }
            OroCmd::Resolve(ref mut resolve) => {
                resolve.layer_config(args.subcommand_matches("resolve").unwrap(), conf)
            }
            OroCmd::View(ref mut view) => {
                view.layer_config(args.subcommand_matches("view").unwrap(), conf)
            }
        }
    }
}
