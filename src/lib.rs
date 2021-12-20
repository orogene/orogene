use std::env;
use std::path::PathBuf;

use oro_command::OroCommand;
use oro_command::{
    clap::{self, ArgMatches, Clap, FromArgMatches, IntoApp},
    directories::ProjectDirs,
    oro_config::{OroConfig, OroConfigLayer, OroConfigOptions},
};
use oro_common::{
    async_trait::async_trait,
    miette::{Context, Result},
    tracing,
};

#[derive(Debug, Clap)]
#[clap(
    author = "Kat Marchán <kzm@zkat.tech>",
    about = "Your do-all-the-things toolkit for JavaScript.",
    version = clap::crate_version!(),
    setting = clap::AppSettings::ColoredHelp,
    setting = clap::AppSettings::DisableHelpSubcommand,
    setting = clap::AppSettings::DeriveDisplayOrder,
    setting = clap::AppSettings::InferSubcommands,
)]
pub struct Orogene {
    #[clap(global = true, long = "root", about = "Package path to operate on.")]
    root: Option<PathBuf>,
    #[clap(global = true, about = "File to read configuration values from.", long)]
    config: Option<PathBuf>,
    #[clap(
        global = true,
        about = "Log verbosity level (off, error, warn, info, debug, trace)",
        long,
        short,
        default_value = "warn"
    )]
    verbosity: tracing::Level,
    #[clap(global = true, about = "Disable all output", long, short = 'q')]
    quiet: bool,
    #[clap(global = true, long, about = "Format output as JSON.")]
    json: bool,
    #[clap(subcommand)]
    subcommand: OroCmd,
}

impl Orogene {
    fn setup_logging(&self) -> Result<()> {
        let mut collector = tracing_subscriber::fmt()
            .with_writer(std::io::stderr)
            .without_time();
        if self.quiet {
            collector = collector.with_max_level(tracing_subscriber::filter::LevelFilter::OFF);
        } else {
            collector = collector.with_max_level(self.verbosity);
        }
        // TODO: Switch to try_init (ugh, `Box<dyn Error>` issues)
        if self.json {
            collector.json().init();
        } else {
            collector.init();
        }

        Ok(())
    }

    pub async fn load() -> Result<()> {
        let start = std::time::Instant::now();
        let clp = Orogene::into_app();
        let matches = clp.get_matches();
        let mut orogene = Orogene::from_arg_matches(&matches);
        let cfg = if let Some(file) = &orogene.config {
            OroConfigOptions::new()
                .global_config_file(Some(file.clone()))
                .load()?
        } else {
            OroConfigOptions::new()
                .global_config_file(
                    ProjectDirs::from("", "", "orogene")
                        .map(|d| d.config_dir().to_owned().join("orogene.kdl")),
                )
                .pkg_root(orogene.root.clone())
                .load()?
        };
        orogene.layer_config(&matches, &cfg)?;
        orogene.setup_logging().context("Failed to setup logging")?;
        orogene.execute().await?;
        tracing::info!("Ran in {}s", start.elapsed().as_millis() as f32 / 1000.0);
        Ok(())
    }
}

#[derive(Debug, Clap)]
pub enum OroCmd {
    #[clap(
        about = "Ping the NPM registry.",
        setting = clap::AppSettings::ColoredHelp,
        setting = clap::AppSettings::DisableHelpSubcommand,
        setting = clap::AppSettings::DeriveDisplayOrder,
    )]
    Ping(oro_cmd_ping::PingCmd),
    #[clap(
        about = "View package information.",
        setting = clap::AppSettings::ColoredHelp,
        setting = clap::AppSettings::DisableHelpSubcommand,
        setting = clap::AppSettings::DeriveDisplayOrder,
    )]
    View(oro_cmd_view::ViewCmd),
}

#[async_trait]
impl OroCommand for Orogene {
    async fn execute(self) -> Result<()> {
        tracing::debug!("Running command: {:#?}", self.subcommand);
        use OroCmd::*;
        match self.subcommand {
            Ping(cmd) => cmd.execute().await,
            View(cmd) => cmd.execute().await,
        }
    }
}

impl OroConfigLayer for Orogene {
    fn layer_config(&mut self, args: &ArgMatches, conf: &OroConfig) -> Result<()> {
        use OroCmd::*;
        let (cmd, match_name): (&mut dyn OroConfigLayer, &str) = match self.subcommand {
            Ping(ref mut cmd) => (cmd, "ping"),
            View(ref mut cmd) => (cmd, "view"),
        };
        cmd.layer_config(args.subcommand_matches(match_name).unwrap(), conf)
    }
}
