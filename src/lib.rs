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
//! ## Benchmarks
//!
//! Even at this early stage, orogene is **very** fast. These benchmarks are
//! all on ubuntu linux running under wsl2, with an ext4 filesystem.
//!
//! All benchmarks are ordered from fastest to slowest (lower is better):
//!
//! ### Warm Cache
//!
//! This test shows performance when running off a warm cache, with an
//! existing lockfile. This scenario is common in CI scenarios with caching
//! enabled, as well as local scenarios where `node_modules` is wiped out in
//! order to "start over" (and potentially when switching branches).
//!
//! Of note here is the contrast between the subsecond (!) installation by
//! orogene, versus the much more noticeable install times of literally
//! everything else.
//!
//! | Package Manager | Mean [ms] | Min [ms] | Max [ms] | Relative |
//! |:---|---:|---:|---:|---:|
//! | `orogene` | 417.3 ± 43.3 | 374.6 | 524.8 | 1.00 |
//! | `bun` | 1535.2 ± 72.5 | 1442.3 | 1628.9 | 3.68 ± 0.42 |
//! | `pnpm` | 8285.1 ± 529.0 | 7680.4 | 9169.9 | 19.85 ± 2.42 |
//! | `yarn` | 20616.7 ± 1726.5 | 18928.6 | 24401.5 | 49.41 ± 6.59 |
//! | `npm` | 29132.0 ± 4569.2 | 25113.4 | 38634.2 | 69.81 ± 13.13 |
//!
//! ### Cold Cache
//!
//! This test shows performance when running off a cold cache, but with an
//! existing lockfile. This scenario is common in CI scenarios that don't
//! cache the package manager caches between runs, and for initial installs by
//! teammates on relatively "clean" machines.
//!
//! | Package Manager | Mean [s] | Min [s] | Max [s] | Relative |
//! |:---|---:|---:|---:|---:|
//! | `bun` | 5.203 ± 1.926 | 3.555 | 9.616 | 1.00 |
//! | `orogene` | 8.346 ± 0.416 | 7.938 | 9.135 | 1.60 ± 0.60 |
//! | `pnpm` | 27.653 ± 0.467 | 26.915 | 28.294 | 5.31 ± 1.97 |
//! | `npm` | 31.613 ± 0.464 | 30.930 | 32.192 | 6.08 ± 2.25 |
//! | `yarn` | 72.815 ± 1.285 | 71.275 | 74.932 | 13.99 ± 5.19 |
//!
//! ## Memory Usage
//!
//! Another big advantage of Orogene is significantly lower memory usage
//! compared to other package managers, with each scenario below showing the
//! peak memory usage (resident set size) for each scenario (collected with
//! /usr/bin/time -v):
//!
//! | Package Manager | no lockfile, no cache | lockfile, cold cache | lockfile, warm cache | existing node_modules |
//! |:---|---:|----:|---:|----:|
//! | `orogene` | 266.8 mb | 155.2 mb | 38.6 mb | 35.5 mb |
//! | `bun` | 2,708.7 mb | 792.1 mb | 34.5 mb | 25.8 mb |
//! | `pnpm` | 950.9 mb | 638.4 mb | 260.1 mb | 168.7 mb |
//! | `npm` | 1,048.9 mb | 448.2 mb | 833.7 mb | 121.7 mb |
//! | `yarn` | 751.1 mb | 334.4 mb | 251.9 mb | 129.3 mb |
//!
//! ### Caveat Emptor
//!
//! At the speeds at which orogene operates, these benchmarks can vary widely
//! because they depend on the underlying filesystem's performance. For
//! example, the gaps might be much smaller on Windows or (sometimes) macOS.
//! They may even vary between different filesystems on Linux/FreeBSD. Note
//! that orogene uses different installation strategies based on support for
//! e.g. reflinking (btrfs, APFS, xfs).
//!
//! ## Contributing
//!
//! For information and help on how to contribute to Orogene, please see
//! [CONTRIBUTING.md](https://github.com/orogene/orogene/blob/main/CONTRIBUTING.md).
//!
//! ## License
//!
//! Orogene and all its sub-crates are licensed under the terms of the [Apache
//! 2.0 License](https://github.com/orogene/orogene/blob/main/LICENSE).

use std::path::{Path, PathBuf};

use async_trait::async_trait;
use clap::{ArgMatches, Args, CommandFactory, FromArgMatches as _, Parser, Subcommand};
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
    #[arg(help_heading = "Global Options", global = true, long)]
    root: Option<PathBuf>,

    /// Registry used for unscoped packages.
    ///
    /// Defaults to https://registry.npmjs.org.
    #[arg(help_heading = "Global Options", global = true, long)]
    registry: Option<Url>,

    /// Location of disk cache.
    ///
    /// Default location varies by platform.
    #[arg(help_heading = "Global Options", global = true, long)]
    cache: Option<PathBuf>,

    /// File to read configuration values from.
    ///
    /// When specified, global configuration loading is disabled and
    /// configuration values will only be read from this location.
    #[clap(help_heading = "Global Options", global = true, long)]
    config: Option<PathBuf>,

    /// Log output level/directive.
    ///
    /// Supports plain loglevels (off, error, warn, info, debug, trace) as
    /// well as more advanced directives in the format
    /// `target[span{field=value}]=level`.
    #[clap(help_heading = "Global Options", global = true, long)]
    loglevel: Option<String>,

    /// Disable all output.
    #[arg(help_heading = "Global Options", global = true, long, short)]
    quiet: bool,

    /// Format output as JSON.
    #[arg(help_heading = "Global Options", global = true, long)]
    json: bool,

    /// Disable progress bar display.
    #[arg(help_heading = "Global Options", global = true, long)]
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

    #[clap(hide = true)]
    HelpMarkdown(HelpMarkdownCmd),
}

#[async_trait]
impl OroCommand for Orogene {
    async fn execute(self) -> Result<()> {
        log_command_line();
        match self.subcommand {
            OroCmd::Ping(cmd) => cmd.execute().await,
            OroCmd::Restore(cmd) => cmd.execute().await,
            OroCmd::View(cmd) => cmd.execute().await,
            OroCmd::HelpMarkdown(cmd) => cmd.execute().await,
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
            OroCmd::HelpMarkdown(ref mut cmd) => {
                cmd.layer_config(args.subcommand_matches("help-markdown").unwrap(), conf)
            }
        }
    }
}

#[derive(Debug, Args, OroConfigLayer)]
pub struct HelpMarkdownCmd {
    #[arg()]
    command_name: String,
}

#[async_trait]
impl OroCommand for HelpMarkdownCmd {
    // Based on:
    // https://github.com/axodotdev/cargo-dist/blob/b79a12e0942021ec304c5dcbf5e0cfcda3e6a4bb/cargo-dist/src/main.rs#L320
    async fn execute(self) -> Result<()> {
        let mut app = Orogene::command();

        // HACK: This is a hack that forces clap to print global options for
        // subcommands when calling `write_long_help` on them.
        let mut _help_buf = Vec::new();
        app.write_long_help(&mut _help_buf).into_diagnostic()?;

        for subcmd in app.get_subcommands_mut() {
            let name = subcmd.get_name();

            if name != self.command_name {
                continue;
            }

            println!("# oro {name}");
            println!();

            let mut help_buf = Vec::new();
            subcmd.write_long_help(&mut help_buf).into_diagnostic()?;
            let help = String::from_utf8(help_buf).into_diagnostic()?;

            for line in help.lines() {
                if let Some(usage) = line.strip_prefix("Usage: ") {
                    println!("### Usage:");
                    println!();
                    println!("```");
                    println!("oro {usage}");
                    println!("```");
                    continue;
                }

                if let Some(heading) = line.strip_suffix(':') {
                    if !line.starts_with(' ') {
                        println!("### {heading}");
                        println!();
                        continue;
                    }
                }

                let line = line.trim();

                if line.starts_with("- ") {
                } else if line.starts_with('-') || line.starts_with('<') {
                    println!("#### `{line}`");
                    println!();
                    continue;
                }

                if line.starts_with('[') {
                    println!("\\{line}  ");
                    continue;
                }

                println!("{line}");
            }

            println!();

            return Ok(());
        }
        Err(miette::miette!("Command not found: {self.command_name}"))
    }
}
