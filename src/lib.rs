//! > Yet another `node_modules/` package manager, I guess.
//!
//! [![crates.io](https://img.shields.io/crates/v/orogene.svg)](https://crates.io/crates/orogene)
//! [![GitHub checks
//! state](https://img.shields.io/github/checks-status/orogene/orogene/main)](https://github.com/orogene/orogene/actions/workflows/ci.yml?query=branch%3Amain)
//! [![Project
//! Roadmap](https://img.shields.io/badge/Roadmap-Project%20Roadmap-informational)](https://github.com/orgs/orogene/projects/2/views/1)
//!
//! Orogene is a next-generation package manager for tools that use
//! `node_modules/`, such as bundlers, CLI tools, and Node.js-based
//! applications. It's fast, robust, and meant to be easily integrated into
//! your workflows such that you never have to worry about whether your
//! `node_modules/` is up to date.
//!
//! > *Note*: Orogene is still under heavy development and shouldn't be
//! > considered much more than a tech demo or proof of concept. Do not use in
//! > production yet.
//!
//! ## Building
//!
//! ### Requirements
//!
//! You will need a Rust toolchain installed. See [the official Rust docs for
//! instructions](https://www.rust-lang.org/tools/install). And
//! [git](https://git-scm.com/downloads). Next, get a checkout of the source:
//!
//! ```text
//! git clone https://github.com/orogene/orogene.git
//! cd orogene
//! ```
//!
//! ### Building
//!
//! Your first build:
//!
//! ```text
//! cargo build
//! ```
//!
//! The first time you run this, this downloads all the dependencies you will
//! need to build orogene automatically. This step might take a minute or two,
//! but it will only be run once.
//!
//! Then it compiles all the dependencies as well as the orogene source files.
//!
//! It should end with something like:
//!
//! ```text
//! …
//! Finished dev [unoptimized + debuginfo] target(s) in 1m 22s
//! ```
//!
//! When you’ve made changes to the orogene source code, run `cargo build`
//! again, and it will only compile the changed files quickly:
//!
//! ```text
//! cargo build
//!    Compiling orogene v0.1.0 (/Users/jan/Work/rust/orogene)
//!     Finished dev [unoptimized + debuginfo] target(s) in 2.41s
//! ```
//!
//! ### Running
//!
//! After building successfully, you can run your build with `cargo run`. In
//! the default configuration, this will run an `oro` executable built for
//! your local system in `./target/debug`. When you run it, it shows you a
//! helpful page of instructions of what you can do with it. Give it a try:
//!
//! ```text
//!     Finished dev [unoptimized + debuginfo] target(s) in 0.14s
//!      Running `target/debug/oro`
//! `node_modules/` package manager and utility toolkit.
//!
//! Usage: oro [OPTIONS] <COMMAND>
//!
//! Commands:
//!   ping     Ping the registry
//!   resolve  Resolve a package tree and save the lockfile to the project directory
//!   restore  Resolves and extracts a node_modules/ tree
//!   view     Get information about a package
//!   help     Print this message or the help of the given subcommand(s)
//!
//! Options:
//!       --root <ROOT>          Package path to operate on
//!       --registry <REGISTRY>  Registry used for unscoped packages
//!       --cache <CACHE>        Location of disk cache
//!       --config <CONFIG>      File to read configuration values from
//!       --loglevel <LOGLEVEL>  Log output level/directive
//!   -q, --quiet                Disable all output
//!       --json                 Format output as JSON
//!   -h, --help                 Print help (see more with '--help')
//!   -V, --version              Print version
//! ```
//!
//! That’s it for now, happy hacking!

use std::path::{Path, PathBuf};

use async_trait::async_trait;
use clap::{ArgMatches, CommandFactory, FromArgMatches as _, Parser, Subcommand};
use directories::ProjectDirs;
use miette::{IntoDiagnostic, Result};
use oro_config::{OroConfig, OroConfigLayer, OroConfigOptions};
use tracing_appender::non_blocking::WorkerGuard;
use tracing_indicatif::IndicatifLayer;
use tracing_subscriber::{
    filter::{Directive, LevelFilter, Targets},
    fmt,
    prelude::*,
    EnvFilter,
};
use url::Url;

use commands::{ping::PingCmd, restore::RestoreCmd, view::ViewCmd, OroCommand};

mod commands;

const MAX_RETAINED_LOGS: usize = 5;

#[derive(Debug, Parser)]
#[command(author, version, about, long_about = None)]
#[command(propagate_version = true)]
pub struct Orogene {
    /// Package path to operate on.
    #[arg(global = true, long)]
    root: Option<PathBuf>,

    /// Registry used for unscoped packages.
    ///
    /// Defaults to https://registry.npmjs.org.
    #[arg(global = true, long)]
    registry: Option<Url>,

    /// Location of disk cache.
    ///
    /// Default location varies by platform.
    #[arg(global = true, long)]
    cache: Option<PathBuf>,

    /// File to read configuration values from.
    ///
    /// When specified, global configuration loading is disabled and
    /// configuration values will only be read from this location.
    #[arg(global = true, long)]
    config: Option<PathBuf>,

    /// Log output level/directive.
    ///
    /// Supports plain loglevels (off, error, warn, info, debug, trace) as
    /// well as more advanced directives in the format
    /// `target[span{field=value}]=level`.
    #[clap(global = true, long)]
    loglevel: Option<String>,

    /// Disable all output.
    #[arg(global = true, long, short)]
    quiet: bool,

    /// Format output as JSON.
    #[arg(global = true, long)]
    json: bool,

    /// Disable progress bar display.
    #[arg(global = true, long)]
    no_progress: bool,

    #[command(subcommand)]
    subcommand: OroCmd,
}

impl Orogene {
    fn setup_logging(&self) -> Result<Option<WorkerGuard>> {
        let builder = EnvFilter::builder();
        let filter = if self.quiet {
            builder
                .with_default_directive(LevelFilter::OFF.into())
                .from_env_lossy()
        } else {
            let dir_str = self.loglevel.clone().unwrap_or_else(|| "warn".to_owned());
            let directives = dir_str
                .split(',')
                .filter(|s| !s.is_empty())
                .filter_map(|s| {
                    let dir: Result<Directive, _> = s.parse();
                    match dir {
                        Ok(dir) => Some(dir),
                        Err(_) => None,
                    }
                });
            let mut filter = builder.from_env_lossy();
            for directive in directives {
                filter = filter.add_directive(directive);
            }
            filter
        };

        let ilayer = IndicatifLayer::new().with_max_progress_bars(1, None);
        let builder = tracing_subscriber::registry();

        if let Some(cache) = self.cache.as_deref() {
            let targets = Targets::new()
                .with_target("hyper", LevelFilter::WARN)
                .with_target("reqwest", LevelFilter::WARN)
                .with_target("tokio_util", LevelFilter::WARN)
                .with_target("async_io", LevelFilter::WARN)
                .with_target("want", LevelFilter::WARN)
                .with_target("async_std", LevelFilter::WARN)
                .with_target("mio", LevelFilter::WARN)
                .with_default(LevelFilter::TRACE);

            clean_old_logs(cache)?;

            let file_appender =
                tracing_appender::rolling::never(cache.join("_logs"), log_file_name());
            let (non_blocking, guard) = tracing_appender::non_blocking(file_appender);

            if self.quiet || self.no_progress {
                builder
                    .with(tracing_subscriber::fmt::layer().with_filter(filter))
                    .with(fmt::layer().with_writer(non_blocking).with_filter(targets))
                    .init();
            } else {
                builder
                    .with(
                        tracing_subscriber::fmt::layer()
                            .with_writer(ilayer.get_stderr_writer())
                            .with_filter(filter),
                    )
                    .with(ilayer)
                    .with(fmt::layer().with_writer(non_blocking).with_filter(targets))
                    .init();
            };

            Ok(Some(guard))
        } else {
            if self.quiet || self.no_progress {
                builder
                    .with(tracing_subscriber::fmt::layer().with_filter(filter))
                    .init();
            } else {
                builder
                    .with(
                        tracing_subscriber::fmt::layer()
                            .with_writer(ilayer.get_stderr_writer())
                            .with_filter(filter),
                    )
                    .with(ilayer)
                    .init();
            };
            Ok(None)
        }
    }

    fn build_config(&self) -> Result<OroConfig> {
        let dirs = ProjectDirs::from("", "", "orogene");
        let cwd = std::env::current_dir().into_diagnostic()?;
        let root = if let Some(root) = pkg_root(&cwd) {
            root
        } else {
            &cwd
        };

        let mut cfg_builder = OroConfigOptions::new()
            .set_default("registry", "https://registry.npmjs.org")?
            .set_default("loglevel", "warn")?
            .set_default("root", &root.to_string_lossy())?;
        if let Some(cache) = dirs.as_ref().map(|d| d.cache_dir().to_owned()) {
            cfg_builder = cfg_builder.set_default("cache", &cache.to_string_lossy())?;
        }

        let cfg = if let Some(file) = &self.config {
            cfg_builder.global_config_file(Some(file.clone())).load()?
        } else {
            cfg_builder
                .global_config_file(dirs.map(|d| d.config_dir().to_owned().join("ororc.toml")))
                .pkg_root(self.root.clone().or(Some(PathBuf::from(root))))
                .load()?
        };

        Ok(cfg)
    }

    pub async fn load() -> Result<()> {
        let start = std::time::Instant::now();
        let matches = Orogene::command().get_matches();
        let mut oro = Orogene::from_arg_matches(&matches).into_diagnostic()?;
        let config = oro.build_config()?;
        oro.layer_config(&matches, &config)?;
        let _guard = oro.setup_logging()?;
        oro.execute().await?;
        tracing::info!("Ran in {}s", start.elapsed().as_millis() as f32 / 1000.0);
        Ok(())
    }
}

fn pkg_root(start_dir: &Path) -> Option<&Path> {
    for path in start_dir.ancestors() {
        let node_modules = path.join("node_modules");
        let pkg_json = path.join("package.json");
        if node_modules.is_dir() {
            return Some(path);
        }
        if pkg_json.is_file() {
            return Some(path);
        }
    }
    None
}

fn log_file_name() -> PathBuf {
    let now = chrono::Local::now();
    let prefix = format!("oro-debug-{}", now.format("%Y-%m-%d-%H-%M-%S%.3f"));
    for i in 0.. {
        let name = PathBuf::from(format!("{}-{}.log", prefix, i));
        if !name.exists() {
            return name;
        }
    }
    PathBuf::from(format!("{}-0.log", prefix))
}

fn clean_old_logs(cache: &Path) -> Result<()> {
    if let Ok(readdir) = cache.join("_logs").read_dir() {
        let mut logs = readdir.filter_map(|e| e.ok()).collect::<Vec<_>>();
        logs.sort_by_key(|e| e.file_name());
        while logs.len() >= MAX_RETAINED_LOGS {
            let log = logs.remove(0);
            std::fs::remove_file(log.path()).into_diagnostic()?;
        }
    }
    Ok(())
}

fn log_command_line() {
    let mut args = std::env::args();
    let mut cmd = String::new();
    if let Some(arg) = args.next() {
        cmd.push_str(&arg);
    }
    for arg in args {
        cmd.push(' ');
        cmd.push_str(&arg);
    }
    tracing::info!("Running command: {cmd}");
}

#[derive(Debug, Subcommand)]
pub enum OroCmd {
    /// Ping the registry.
    Ping(PingCmd),

    /// Resolves and extracts a node_modules/ tree.
    Restore(RestoreCmd),

    /// Get information about a package.
    View(ViewCmd),
}

#[async_trait]
impl OroCommand for Orogene {
    async fn execute(self) -> Result<()> {
        log_command_line();
        match self.subcommand {
            OroCmd::Ping(cmd) => cmd.execute().await,
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
            OroCmd::Restore(ref mut cmd) => {
                cmd.layer_config(args.subcommand_matches("restore").unwrap(), conf)
            }
            OroCmd::View(ref mut cmd) => {
                cmd.layer_config(args.subcommand_matches("view").unwrap(), conf)
            }
        }
    }
}
