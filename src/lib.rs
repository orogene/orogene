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
use cmd_restore::RestoreCmd;
use cmd_view::ViewCmd;

#[derive(Debug, Parser)]
#[command(author, version, about, long_about = None)]
#[command(propagate_version = true)]
pub struct Orogene {
    /// Package path to operate on
    #[arg(global = true, long = "root")]
    root: Option<PathBuf>,

    /// Location of disk cache
    #[arg(global = true, long)]
    cache: Option<PathBuf>,

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

    fn build_config(&self) -> Result<OroConfig> {
        let dirs = ProjectDirs::from("", "", "orogene");
        let cfg_builder = OroConfigOptions::new();
        // TODO: Figure out a way to set defaults globally, but that only get
        // used if CLI args fall through? idk what's going on.
        //     .set_default("registry", "https://registry.npmjs.org")?
        //     .set_default("root", ".")?;
        // if let Some(cache) = dirs.as_ref().map(|d| d.cache_dir().to_owned()) {
        //     cfg_builder = cfg_builder.set_default("cache", &cache.to_string_lossy().to_string())?;
        // }

        let cfg = if let Some(file) = &self.config {
            cfg_builder.global_config_file(Some(file.clone())).load()?
        } else {
            cfg_builder
                .global_config_file(
                    dirs.as_ref()
                        .map(|d| d.config_dir().to_owned().join("ororc.toml")),
                )
                .pkg_root(self.root.clone())
                .load()?
        };

        Ok(cfg)
    }

    pub async fn load() -> Result<()> {
        let start = std::time::Instant::now();
        let matches = Orogene::command().get_matches();
        let mut oro = Orogene::from_arg_matches(&matches).into_diagnostic()?;
        oro.layer_config(&matches, &oro.build_config()?)?;
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

    /// Resolves and extracts a node_modules/ tree.
    Restore(RestoreCmd),

    /// Get information about a package.
    View(ViewCmd),
}

#[async_trait]
impl OroCommand for Orogene {
    async fn execute(self) -> Result<()> {
        tracing::info!("Running command: {:#?}", self.subcommand);
        match self.subcommand {
            OroCmd::Ping(cmd) => cmd.execute().await,
            OroCmd::Resolve(cmd) => cmd.execute().await,
            OroCmd::Restore(cmd) => cmd.execute().await,
            OroCmd::View(cmd) => cmd.execute().await,
        }
    }
}

impl OroConfigLayer for Orogene {
    fn layer_config(&mut self, args: &ArgMatches, conf: &OroConfig) -> Result<()> {
        match self.subcommand {
            OroCmd::Ping(ref mut cmd) => {
                cmd.layer_config(args.subcommand_matches("ping").unwrap(), conf)
            }
            OroCmd::Resolve(ref mut cmd) => {
                cmd.layer_config(args.subcommand_matches("resolve").unwrap(), conf)
            }
            OroCmd::Restore(ref mut cmd) => {
                cmd.layer_config(args.subcommand_matches("restore").unwrap(), conf)
            }
            OroCmd::View(ref mut cmd) => {
                cmd.layer_config(args.subcommand_matches("view").unwrap(), conf)
            }
        }
    }
}
