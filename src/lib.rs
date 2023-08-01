//! > Makes `node_modules/` happen. Fast. No fuss.
//!
//! [![release](https://img.shields.io/github/v/release/orogene/orogene?display_name=tag&include_prereleases)](https://github.com/orogene/orogene/releases/latest)
//! [![npm](https://img.shields.io/npm/v/oro)](https://www.npmjs.com/package/oro)
//! [![crates.io](https://img.shields.io/crates/v/orogene.svg)](https://crates.io/crates/orogene)
//! [![CI](https://img.shields.io/github/checks-status/orogene/orogene/main)](https://github.com/orogene/orogene/actions/workflows/ci.yml?query=branch%3Amain)
//! [![Project
//! Roadmap](https://img.shields.io/badge/Roadmap-Orogene%20v1.0-informational)](https://github.com/orgs/orogene/projects/2/views/1)
//! [![chat](https://img.shields.io/matrix/orogene:matrix.org?label=Matrix%20chat)](https://matrix.to/#/#orogene:matrix.org)
//!
//! Orogene is a next-generation package manager for tools that use
//! `node_modules/`, such as bundlers, CLI tools, and Node.js-based
//! applications. It's fast, robust, and meant to be easily integrated into
//! your workflows such that you never have to worry about whether your
//! `node_modules/` is up to date.
//!
//! > *Note*: Orogene is still under heavy development and may not yet be
//! > suitable for production use. It is missing some features that you might
//! > expect. Check [the roadmap](https://github.com/orgs/orogene/projects/2)
//! > to see where we're headed and [talk to
//! > us](https://github.com/orogene/orogene/discussions/categories/pain-points)
//! > about what you want/need!.
//!
//! ## Getting Started
//!
//! You can install Orogene in various ways:
//!
//! npx:
//! ```sh
//! $ npx oro ping
//! ```
//!
//! NPM:
//! ```sh
//! $ npm install -g oro
//! ```
//!
//! Cargo:
//! ```sh
//! $ cargo install orogene
//! ```
//!
//! You can also find install scripts and archive downloads in [the latest
//! release](https://github.com/orogene/orogene/releases/latest).
//!
//! ## Usage
//!
//! For usage documentation, see [the Orogene
//! docs](https://orogene.dev/book/), or run `$ oro help`.
//!
//! If you just want to do something similar to `$ npm install`, you can run
//! `$ oro apply` in your project and go from there.
//!
//! ## Performance
//!
//! Orogene is very fast and uses significantly fewer resources than other
//! package managers, in both memory and disk space. It's able to install some
//! non-trivial projects in sub-second time:
//!
//! ![Warm cache comparison]
//!
//! For details and more benchmarks, see [the benchmarks page].
//!
//! ## Contributing
//!
//! For information and help on how to contribute to Orogene, please see [our
//! contribution guide].
//!
//! ## License
//!
//! Orogene and all its sub-crates are licensed under the terms of the [Apache
//! 2.0 License].
//!
//! [Warm cache comparison]:
//!     https://orogene.dev/assets/benchmarks-warm-cache.png
//! [the benchmarks page]: https://orogene.dev/BENCHMARKS.html
//! [our contribution guide]:
//!     https://github.com/orogene/orogene/blob/main/CONTRIBUTING.md
//! [Apache 2.0 License]: https://github.com/orogene/orogene/blob/main/LICENSE

use std::{
    borrow::Cow,
    collections::VecDeque,
    ffi::OsString,
    panic::PanicInfo,
    path::{Path, PathBuf},
    sync::Arc,
};

use async_trait::async_trait;
use clap::{Args, Command, CommandFactory, FromArgMatches as _, Parser, Subcommand};
use dialoguer::{theme::ColorfulTheme, Confirm};
use directories::ProjectDirs;
use is_terminal::IsTerminal;
use kdl::{KdlDocument, KdlNode, KdlValue};
use miette::{IntoDiagnostic, Result};
use oro_config::{OroConfig, OroConfigLayerExt, OroConfigOptions};
use tracing_appender::non_blocking::WorkerGuard;
use tracing_indicatif::IndicatifLayer;
use tracing_subscriber::{
    filter::{Directive, LevelFilter, Targets},
    fmt,
    prelude::*,
    EnvFilter,
};
use url::Url;

use commands::OroCommand;

pub use error::OroError;

mod apply_args;
mod commands;
mod error;
mod nassun_args;

const MAX_RETAINED_LOGS: usize = 5;

#[derive(Debug, Parser)]
#[command(author, version, about, long_about = None)]
#[command(propagate_version = true)]
pub struct Orogene {
    /// Path to the project to operate on.
    ///
    /// By default, Orogene will look up from the current working directory
    /// until it finds a directory with a `package.json` file or a
    /// `node_modules/` directory.
    #[arg(
        help_heading = "Global Options",
        global = true,
        long,
        // NB(zkat): this is actually a dummy value. The real root default is
        // handled by the config system, and is a special case.
        default_value = "."
    )]
    root: PathBuf,

    /// Registry used for unscoped packages.
    #[arg(
        help_heading = "Global Options",
        global = true,
        long,
        default_value = "https://registry.npmjs.org"
    )]
    registry: Url,

    /// Registry to use for a specific `@scope`, using `--scoped-registry
    /// @scope=https://foo.com` format.
    ///
    /// Can be provided multiple times to specify multiple scoped registries.
    #[arg(
        help_heading = "Global Options",
        global = true,
        alias = "scoped-registries",
        long = "scoped-registry",
        value_parser = parse_key_value::<String, Url>
    )]
    scoped_registries: Vec<(String, Url)>,

    /// Credentials to apply to registries when they're accessed. You can
    /// provide credentials for multiple registries at a time, and different
    /// credential fields for a registry.
    ///
    /// The syntax is `--credentials my.registry.com:username=foo
    /// --credentials my.registry.com:password=sekrit`.
    #[arg(
        help_heading = "Global Options",
        global = true,
        long,
        value_parser = parse_nested_key_value::<String, String, String>
    )]
    credentials: Vec<(String, String, String)>,

    /// Location of disk cache.
    ///
    /// Default location varies by platform.
    #[arg(help_heading = "Global Options", global = true, long)]
    cache: Option<PathBuf>,

    /// File to read configuration values from.
    ///
    /// When specified, global configuration loading is disabled and
    /// configuration values will only be read from this location.
    #[arg(help_heading = "Global Options", global = true, long)]
    config: Option<PathBuf>,

    /// Log output level/directive.
    ///
    /// Supports plain loglevels (off, error, warn, info, debug, trace) as
    /// well as more advanced directives in the format
    /// `target[span{field=value}]=level`.
    #[arg(
        help_heading = "Global Options",
        global = true,
        long,
        default_value = "info"
    )]
    loglevel: String,

    /// Disable all output.
    #[arg(help_heading = "Global Options", global = true, long, short)]
    quiet: bool,

    /// Format output as JSON.
    #[arg(help_heading = "Global Options", global = true, long)]
    json: bool,

    /// Disable the progress bars.
    #[arg(
        help_heading = "Global Options",
        global = true,
        long = "no-progress",
        action = clap::ArgAction::SetFalse,
    )]
    progress: bool,

    /// Disable printing emoji.
    ///
    /// By default, this will show emoji when outputting to a TTY that
    /// supports unicode.
    #[arg(
        help_heading = "Global Options",
        global = true,
        long = "no-emoji",
        action = clap::ArgAction::SetFalse,
        default_value_t = supports_unicode::on(supports_unicode::Stream::Stderr)
    )]
    emoji: bool,

    /// Skip first-time setup.
    #[arg(
        help_heading = "Global Options",
        global = true,
        long = "no-first-time",
        action = clap::ArgAction::SetFalse,
    )]
    first_time: bool,

    /// Disable telemetry.
    ///
    /// Telemetry for Orogene is opt-in, anonymous, and is used to help the
    /// team improve the product. It is usually configured on first run, but
    /// you can use this flag to force-disable it either in an individual CLI
    /// call, or in a project-local oro.kdl.
    #[arg(
        help_heading = "Global Options",
        global = true,
        long = "no-telemetry",
        action = clap::ArgAction::SetFalse,
    )]
    telemetry: bool,

    /// Sentry DSN (access token) where telemetry will be sent (if enabled).
    #[arg(help_heading = "Global Options", global = true, long)]
    sentry_dsn: Option<String>,

    #[command(subcommand)]
    subcommand: OroCmd,

    /// Use proxy to delegate the network.
    ///
    /// Proxy is opt-in, it uses for outgoing http/https request.
    /// If enabled, should set proxy-url too.
    #[arg(
        help_heading = "Global Options",
        global = true,
        long,
        default_value_t = false
    )]
    proxy: bool,

    /// A proxy to use for outgoing http requests.
    #[arg(
        help_heading = "Global Options",
        global = true,
        long = "proxy-url",
        default_value = None
    )]
    proxy_url: Option<String>,

    /// Use commas to separate multiple entries, e.g. `.host1.com,.host2.com`.
    ///
    /// Can also be configured through the `NO_PROXY` environment variable, like `NO_PROXY=.host1.com`.
    #[arg(
        help_heading = "Global Options",
        global = true,
        long = "no-proxy-domain",
        default_value = None
    )]
    no_proxy_domain: Option<String>,

    /// Package will retry when network failed.
    #[arg(
        help_heading = "Global Options",
        global = true,
        long,
        default_value_t = 2
    )]
    fetch_retries: u32,
}

impl Orogene {
    fn setup_logging(&self, log_file: Option<&Path>) -> Result<Option<WorkerGuard>> {
        let builder = EnvFilter::builder();
        let filter = if self.quiet {
            builder
                .with_default_directive(LevelFilter::OFF.into())
                .from_env_lossy()
        } else {
            let dir_str = self.loglevel.clone();
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

        let ilayer = IndicatifLayer::new();
        let builder = tracing_subscriber::registry();

        if let Some(log_file) = &log_file {
            let targets = Targets::new()
                .with_target("hyper", LevelFilter::WARN)
                .with_target("reqwest", LevelFilter::WARN)
                .with_target("tokio_util", LevelFilter::WARN)
                .with_target("async_io", LevelFilter::WARN)
                .with_target("want", LevelFilter::WARN)
                .with_target("async_std", LevelFilter::WARN)
                .with_target("mio", LevelFilter::WARN)
                .with_target("polling", LevelFilter::WARN)
                .with_default(LevelFilter::TRACE);

            let logs_dir = log_file.parent().expect("must have parent");
            clean_old_logs(logs_dir)?;

            let file_appender = tracing_appender::rolling::never(
                logs_dir,
                log_file.file_name().expect("must have file name"),
            );
            let (non_blocking, guard) = tracing_appender::non_blocking(file_appender);

            if self.quiet || !self.progress {
                builder
                    .with(
                        tracing_subscriber::fmt::layer()
                            .without_time()
                            .with_target(false)
                            .with_filter(filter),
                    )
                    .with(
                        fmt::layer()
                            .with_timer(tracing_subscriber::fmt::time::uptime())
                            .with_writer(non_blocking)
                            .with_target(false)
                            .with_ansi(false)
                            .with_filter(targets),
                    )
                    .init();
            } else {
                builder
                    .with(
                        tracing_subscriber::fmt::layer()
                            .without_time()
                            .with_writer(ilayer.get_stderr_writer())
                            .with_target(false)
                            .with_filter(filter),
                    )
                    .with(ilayer.with_filter(LevelFilter::DEBUG))
                    .with(
                        fmt::layer()
                            .with_timer(tracing_subscriber::fmt::time::uptime())
                            .with_writer(non_blocking)
                            .with_target(false)
                            .with_ansi(false)
                            .with_filter(targets),
                    )
                    .init();
            };

            Ok(Some(guard))
        } else {
            if self.quiet || !self.progress {
                builder
                    .with(
                        tracing_subscriber::fmt::layer()
                            .without_time()
                            .with_target(false)
                            .with_filter(filter),
                    )
                    .init();
            } else {
                builder
                    .with(
                        tracing_subscriber::fmt::layer()
                            .without_time()
                            .with_target(false)
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
            .set_default("root", &root.to_string_lossy())?
            .env(true);
        if let Some(cache) = dirs.as_ref().map(|d| d.cache_dir().to_owned()) {
            cfg_builder = cfg_builder.set_default("cache", &cache.to_string_lossy())?;
        }

        let cfg = if let Some(file) = &self.config {
            cfg_builder.global_config_file(Some(file.clone())).load()?
        } else {
            cfg_builder
                .global_config_file(dirs.map(|d| d.config_dir().to_owned().join("oro.kdl")))
                .pkg_root(Some(self.root.clone()))
                .load()?
        };

        Ok(cfg)
    }

    fn current_command() -> Command {
        // First, we do a fake parse. All we really want is to get the subcommand, here.
        let matches = Orogene::command().ignore_errors(true).get_matches();
        let mut matches_ref = &matches;

        // Next, we "recursively" follow the subcommand chain until we bottom
        // out, then keep that path.
        let mut subcmd_path = VecDeque::new();
        while let Some((sub_cmd, new_m)) = matches_ref.subcommand() {
            subcmd_path.push_back(sub_cmd);
            matches_ref = new_m;
        }

        // Then, we find the subcommand starting from our toplevel Command.
        let mut command = Orogene::command().with_negations();
        let mut subcmd = &mut command;
        while let Some(name) = subcmd_path.pop_front() {
            subcmd = subcmd
                .find_subcommand_mut(name)
                .expect("This should definitely exist?");
            *subcmd = subcmd.clone().with_negations();
        }

        command
    }

    fn layer_command_args(
        command: &Command,
        args: &mut Vec<OsString>,
        config: &OroConfig,
    ) -> Result<()> {
        // First, we do a fake parse. All we really want is to get the subcommand, here.
        let matches = command.clone().ignore_errors(true).get_matches();
        let mut matches_ref = &matches;

        // Next, we "recursively" follow the subcommand chain until we bottom
        // out, then keep that path.
        let mut subcmd_path = VecDeque::new();
        while let Some((sub_cmd, new_m)) = matches_ref.subcommand() {
            subcmd_path.push_back(sub_cmd);
            matches_ref = new_m;
        }

        // Then, we find the subcommand starting from our toplevel Command.
        let mut command = command.clone();
        let mut subcmd = &mut command;
        subcmd.layered_args(args, config)?;
        while let Some(name) = subcmd_path.pop_front() {
            subcmd = subcmd
                .find_subcommand_mut(name)
                .expect("This should definitely exist?");
            subcmd.layered_args(args, config)?;
        }
        Ok(())
    }

    fn first_time_setup(&mut self) -> Result<()> {
        // We skip first-time-setup operations in CI entirely.
        if self.first_time && !is_ci::cached() {
            tracing::info!("Performing first-time setup...");
            if let Some(dirs) = ProjectDirs::from("", "", "orogene") {
                let config_dir = dirs.config_dir();
                if !config_dir.exists() {
                    std::fs::create_dir_all(config_dir).unwrap();
                }
                let config_path = config_dir.join("oro.kdl");
                let mut config: KdlDocument = std::fs::read_to_string(&config_path)
                    .unwrap_or_default()
                    .parse()?;
                let telemetry_exists = config.query("options > telemetry")?.is_some();
                if config.get("options").is_none() {
                    config.nodes_mut().push(KdlNode::new("options"));
                }
                if std::io::stdout().is_terminal() {
                    if let Some(opts) = config.get_mut("options") {
                        self.telemetry = self.prompt_telemetry_opt_in()?;
                        if !telemetry_exists {
                            let mut node = KdlNode::new("telemetry");
                            node.push(KdlValue::Bool(self.telemetry));
                            opts.ensure_children();
                            if let Some(doc) = opts.children_mut().as_mut() {
                                doc.nodes_mut().push(node)
                            }
                        }
                        if let Some(opt) = config
                            .get_mut("options")
                            .unwrap()
                            .children_mut()
                            .as_mut()
                            .unwrap()
                            .get_mut("telemetry")
                        {
                            if let Some(val) = opt.get_mut(0) {
                                *val = self.telemetry.into();
                            } else {
                                opt.push(KdlValue::Bool(self.telemetry));
                            }
                        }
                    }
                }
                if config.query("options > first-time")?.is_none() {
                    let mut node = KdlNode::new("first-time");
                    node.push(KdlValue::Bool(false));
                    let opts = config.get_mut("options").unwrap();
                    opts.ensure_children();
                    if let Some(doc) = opts.children_mut().as_mut() {
                        doc.nodes_mut().push(node)
                    }
                }
                if let Some(opt) = config
                    .get_mut("options")
                    .unwrap()
                    .children_mut()
                    .as_mut()
                    .unwrap()
                    .get_mut("first-time")
                {
                    if let Some(val) = opt.get_mut(0) {
                        *val = false.into();
                    } else {
                        opt.push(KdlValue::Bool(false));
                    }
                }
                std::fs::write(config_path, config.to_string()).into_diagnostic()?;
            }
        }
        Ok(())
    }

    fn prompt_telemetry_opt_in(&self) -> Result<bool> {
        tracing::info!("Orogene is able to collect anonymous usage statistics and");
        tracing::info!("crash reports to help the team improve the tool.");
        tracing::info!(
            "Anonymous, aggregate metrics are publicly available (see `oro telemetry`),"
        );
        tracing::info!("and no personally identifiable information is collected.");
        tracing::info!("This is entirely opt-in, but we would appreciate it if you considered it!");
        Confirm::with_theme(&ColorfulTheme::default())
            .with_prompt("Do you wish to enable anonymous telemetry?")
            .interact()
            .into_diagnostic()
    }

    fn setup_telemetry(
        &self,
        log_file: Option<PathBuf>,
    ) -> Result<Option<sentry::ClientInitGuard>> {
        if !self.telemetry {
            return Ok(None);
        }

        if let Some(dsn) = self
            .sentry_dsn
            .as_deref()
            .or_else(|| option_env!("OROGENE_SENTRY_DSN"))
        {
            let ret = sentry::init(
                sentry::ClientOptions {
                    dsn: Some(dsn.parse().into_diagnostic()?),
                    release: sentry::release_name!(),
                    server_name: None,
                    sample_rate: 0.1,
                    user_agent: Cow::from(format!(
                        "orogene@{} ({}/{})",
                        env!("CARGO_PKG_VERSION"),
                        std::env::consts::OS,
                        std::env::consts::ARCH,
                    )),
                    default_integrations: false,
                    before_send: Some(Arc::new(|mut event| {
                        event.server_name = None; // Don't send server name
                        Some(event)
                    })),
                    ..Default::default()
                }
                .add_integration(
                    sentry::integrations::backtrace::AttachStacktraceIntegration::default(),
                )
                .add_integration(
                    sentry::integrations::panic::PanicIntegration::default().add_extractor(
                        move |info: &PanicInfo| {
                            if let Some(log_file) = log_file.as_deref() {
                                sentry::configure_scope(|s| {
                                    s.add_attachment(sentry::protocol::Attachment {
                                        filename: log_file
                                            .file_name()
                                            .map(|f| f.to_string_lossy().to_string())
                                            .unwrap_or_else(|| "oro-debug.log".into()),
                                        content_type: Some("text/plain".into()),
                                        buffer: std::fs::read(log_file).unwrap_or_default(),
                                        ty: None,
                                    });
                                });
                            }
                            let msg = sentry::integrations::panic::message_from_panic_info(info);
                            Some(sentry::protocol::Event {
                                exception: vec![sentry::protocol::Exception {
                                    ty: "panic".into(),
                                    mechanism: Some(sentry::protocol::Mechanism {
                                        ty: "panic".into(),
                                        handled: Some(false),
                                        ..Default::default()
                                    }),
                                    value: Some(msg.to_string()),
                                    stacktrace: sentry::integrations::backtrace::current_stacktrace(
                                    ),
                                    ..Default::default()
                                }]
                                .into(),
                                level: sentry::Level::Fatal,
                                ..Default::default()
                            })
                        },
                    ),
                )
                .add_integration(sentry::integrations::contexts::ContextIntegration::new())
                .add_integration(
                    sentry::integrations::backtrace::ProcessStacktraceIntegration::default(),
                ),
            );
            Ok(Some(ret))
        } else {
            Ok(None)
        }
    }

    pub async fn load() -> Result<()> {
        let start = std::time::Instant::now();
        // We have to instantiate Orogene twice: once to pick up "base" config
        // options, like `root` and `config`, which affect our overall config
        // parsing, and then a second time to pick up config options from the
        // config file(s). The first instantiation also ignores errors,
        // because what we really need to apply the negations to is the
        // subcommand we're interested in.
        let command = Self::current_command();
        let matches = command.clone().get_matches();
        let oro = Orogene::from_arg_matches(&matches).into_diagnostic()?;
        let config = oro.build_config()?;
        let mut args = std::env::args_os().collect::<Vec<_>>();
        Self::layer_command_args(&command, &mut args, &config)?;
        let mut oro =
            Orogene::from_arg_matches(&command.get_matches_from(&args)).into_diagnostic()?;
        let log_file = oro
            .cache
            .clone()
            .or_else(|| config.get::<String>("cache").ok().map(PathBuf::from))
            .map(|c| c.join("_logs").join(log_file_name()));
        let _logging_guard = oro.setup_logging(log_file.as_deref())?;
        oro.first_time_setup()?;
        let _telemetry_guard = oro.setup_telemetry(log_file.clone())?;
        oro.execute().await.map_err(|e| {
            // We toss this in a debug so execution errors show up in our
            // debug logs. Unfortunately, we can't do the same for other
            // errors in this method because they all happen before the debug
            // log is even set up.
            tracing::debug!("{e:?}");
            if let Some(log_file) = log_file.as_deref() {
                tracing::warn!("A debug log was written to {}", log_file.display());
                sentry::configure_scope(|s| {
                    s.add_attachment(sentry::protocol::Attachment {
                        filename: log_file
                            .file_name()
                            .map(|f| f.to_string_lossy().to_string())
                            .unwrap_or_else(|| "oro-debug.log".into()),
                        content_type: Some("text/plain".into()),
                        buffer: std::fs::read(log_file).unwrap_or_default(),
                        ty: None,
                    });
                });
            }
            let dyn_err: &dyn std::error::Error = e.as_ref();
            sentry::capture_error(dyn_err);
            e
        })?;
        tracing::debug!("Ran in {}s", start.elapsed().as_millis() as f32 / 1000.0);
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

fn clean_old_logs(logs_dir: &Path) -> Result<()> {
    if let Ok(readdir) = logs_dir.read_dir() {
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
    tracing::debug!("Running command: {cmd}");
}

fn parse_key_value<T, U>(
    s: &str,
) -> Result<(T, U), Box<dyn std::error::Error + Send + Sync + 'static>>
where
    T: std::str::FromStr,
    T::Err: std::error::Error + Send + Sync + 'static,
    U: std::str::FromStr,
    U::Err: std::error::Error + Send + Sync + 'static,
{
    let pos = s
        .find('=')
        .ok_or_else(|| format!("invalid KEY=VALUE pair: no `=` found in `{s}`"))?;

    Ok((s[..pos].parse()?, s[pos + 1..].parse()?))
}

fn parse_nested_key_value<T, U, V>(
    s: &str,
) -> Result<(T, U, V), Box<dyn std::error::Error + Send + Sync + 'static>>
where
    T: std::str::FromStr,
    T::Err: std::error::Error + Send + Sync + 'static,
    U: std::str::FromStr,
    U::Err: std::error::Error + Send + Sync + 'static,
    V: std::str::FromStr,
    V::Err: std::error::Error + Send + Sync + 'static,
{
    let colon_pos = s
        .find(':')
        .ok_or_else(|| format!("invalid TOP_KEY:NESTED_KEY=VALUE entry: no `:` found in `{s}`",))?;
    let eq_pos = s
        .find('=')
        .ok_or_else(|| format!("invalid TOP_KEY:NESTED_KEY=VALUE entry: no `=` found in `{s}`"))?;

    Ok((
        s[..colon_pos].parse()?,
        s[colon_pos + 1..eq_pos].parse()?,
        s[eq_pos + 1..].parse()?,
    ))
}

#[derive(Debug, Subcommand)]
pub enum OroCmd {
    Add(commands::add::AddCmd),

    Apply(commands::apply::ApplyCmd),

    Ping(commands::ping::PingCmd),

    Reapply(commands::reapply::ReapplyCmd),

    Remove(commands::remove::RemoveCmd),

    View(commands::view::ViewCmd),

    #[clap(hide = true)]
    HelpMarkdown(HelpMarkdownCmd),
}

#[async_trait]
impl OroCommand for Orogene {
    async fn execute(self) -> Result<()> {
        log_command_line();
        match self.subcommand {
            OroCmd::Add(cmd) => cmd.execute().await,
            OroCmd::Apply(cmd) => cmd.execute().await,
            OroCmd::Ping(cmd) => cmd.execute().await,
            OroCmd::Reapply(cmd) => cmd.execute().await,
            OroCmd::Remove(cmd) => cmd.execute().await,
            OroCmd::View(cmd) => cmd.execute().await,
            OroCmd::HelpMarkdown(cmd) => cmd.execute().await,
        }
    }
}

/// Used for generating markdown documentation for Orogene commands.
#[derive(Debug, Args)]
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
                    let aliases = subcmd.get_visible_aliases().collect::<Vec<_>>();
                    if !aliases.is_empty() {
                        println!();
                        println!(
                            "[alias{}: {}]",
                            if aliases.len() == 1 { "" } else { "es" },
                            aliases.join(", ")
                        );
                    }
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
                    println!("\\{line}");
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
