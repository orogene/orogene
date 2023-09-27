//! Configuration loader for Orogene config files.

use std::{collections::HashSet, ffi::OsString, path::PathBuf};

pub use clap::{ArgMatches, Command};
pub use config::Config as OroConfig;
use config::{builder::DefaultState, ConfigBuilder, Environment, File, ValueKind};
use kdl_source::KdlFormat;
use miette::Result;

use error::OroConfigError;

mod error;
mod kdl_source;

pub trait OroConfigLayerExt {
    fn with_negations(self) -> Self;
    fn layered_args(&self, args: &mut Vec<OsString>, config: &OroConfig) -> Result<()>;
}

impl OroConfigLayerExt for Command {
    fn with_negations(self) -> Self {
        let negated = self
            .get_arguments()
            .filter(|opt| opt.get_long().is_some())
            .map(|opt| format!("no-{}", opt.get_long().expect("long option")))
            .collect::<Vec<_>>();
        let negations = self
            .get_arguments()
            .filter(|opt| opt.get_long().is_some())
            .zip(negated)
            .map(|(opt, negated)| {
                // This is a bit tricky. For arguments that we want to have
                // `--no-foo` for, but we want `foo` to default to true, we
                // need to set the `long` flag _on the original_ to `no-foo`,
                // and then this one will "reverse" it.
                let long = if negated.starts_with("no-no-") {
                    negated.replace("no-no-", "")
                } else {
                    negated.clone()
                };
                clap::Arg::new(negated)
                    .long(long)
                    .global(opt.is_global_set())
                    .hide(true)
                    .action(clap::ArgAction::SetTrue)
                    .overrides_with(opt.get_id())
            })
            .collect::<Vec<_>>();
        // Add the negations
        self.args(negations)
    }

    fn layered_args(&self, args: &mut Vec<OsString>, config: &OroConfig) -> Result<()> {
        let mut long_opts = HashSet::new();
        for opt in self.get_arguments() {
            if opt.get_long().is_some() {
                long_opts.insert(opt.get_id().to_string());
            }
        }
        let matches = self
            .clone()
            .ignore_errors(true)
            .get_matches_from(&args.clone());
        for opt in long_opts {
            // TODO: _prepend_ args unconditionally if they're coming from
            // config, so multi-args get parsed right. Right now, if you have
            // something in your config, it'll get completely overridden by
            // the command line.
            if matches.value_source(&opt) != Some(clap::parser::ValueSource::CommandLine) {
                let opt = opt.replace('_', "-");
                if !args.contains(&OsString::from(format!("--no-{opt}"))) {
                    if let Ok(bool) = config.get_bool(&opt) {
                        if bool {
                            args.push(OsString::from(format!("--{}", opt)));
                        } else {
                            args.push(OsString::from(format!("--no-{}", opt)));
                        }
                    } else if let Ok(value) = config.get_string(&opt) {
                        args.push(OsString::from(format!("--{}", opt)));
                        args.push(OsString::from(value));
                    } else if let Ok(value) = config.get_table(&opt) {
                        for (key, val) in value {
                            match &val.kind {
                                ValueKind::Table(map) => {
                                    for (k, v) in map {
                                        args.push(OsString::from(format!("--{}", opt)));
                                        args.push(OsString::from(format!("{{{key}}}{k}={v}")));
                                    }
                                }
                                // TODO: error if val.kind is an Array
                                _ => {
                                    args.push(OsString::from(format!("--{}", opt)));
                                    args.push(OsString::from(format!("{key}={val}")));
                                }
                            }
                        }
                    } else if let Ok(value) = config.get_array(&opt) {
                        for val in value {
                            if let Ok(val) = val.into_string() {
                                args.push(OsString::from(format!("--{}", opt)));
                                args.push(OsString::from(val));
                            }
                        }
                    }
                }
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct OroConfigOptions {
    builder: ConfigBuilder<DefaultState>,
    global: bool,
    env: bool,
    pkg_root: Option<PathBuf>,
    global_config_file: Option<PathBuf>,
}

impl Default for OroConfigOptions {
    fn default() -> Self {
        OroConfigOptions {
            builder: OroConfig::builder(),
            global: true,
            env: true,
            pkg_root: None,
            global_config_file: None,
        }
    }
}

impl OroConfigOptions {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn global(mut self, global: bool) -> Self {
        self.global = global;
        self
    }

    pub fn env(mut self, env: bool) -> Self {
        self.env = env;
        self
    }

    pub fn pkg_root(mut self, root: Option<PathBuf>) -> Self {
        self.pkg_root = root;
        self
    }

    pub fn global_config_file(mut self, file: Option<PathBuf>) -> Self {
        self.global_config_file = file;
        self
    }

    pub fn set_default(mut self, key: &str, value: &str) -> Result<Self, OroConfigError> {
        self.builder = self.builder.set_default(key, value)?;
        Ok(self)
    }

    pub fn load(self) -> Result<OroConfig> {
        let mut builder = self.builder;
        if self.global {
            if let Some(config_file) = self.global_config_file {
                let path = config_file.display().to_string();
                builder = builder.add_source(File::new(&path, KdlFormat).required(false));
            }
        }
        if self.env {
            builder = builder.add_source(Environment::with_prefix("oro_config"));
        }
        if let Some(root) = self.pkg_root {
            builder = builder.add_source(
                File::new(&root.join("oro.kdl").display().to_string(), KdlFormat).required(false),
            );
        }
        Ok(builder.build().map_err(OroConfigError::ConfigError)?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::env;
    use std::fs;

    use miette::{IntoDiagnostic, Result};
    use pretty_assertions::assert_eq;
    use tempfile::tempdir;

    #[test]
    fn env_configs() -> Result<()> {
        let dir = tempdir().into_diagnostic()?;
        env::set_var("ORO_CONFIG_STORE", dir.path().display().to_string());
        let config = OroConfigOptions::new().global(false).load()?;
        env::remove_var("ORO_CONFIG_STORE");
        assert_eq!(
            config.get_string("store").into_diagnostic()?,
            dir.path().display().to_string()
        );
        Ok(())
    }

    #[test]
    fn global_config() -> Result<()> {
        let dir = tempdir().into_diagnostic()?;
        let file = dir.path().join("oro.kdl");
        fs::write(&file, "options{\nstore \"hello world\"\n}").into_diagnostic()?;
        let config = OroConfigOptions::new()
            .env(false)
            .global_config_file(Some(file))
            .load()?;
        assert_eq!(
            config.get_string("store").into_diagnostic()?,
            String::from("hello world")
        );
        Ok(())
    }

    #[test]
    fn missing_config() -> Result<()> {
        let config = OroConfigOptions::new().global(false).env(false).load()?;
        assert!(config.get_string("store").is_err());
        Ok(())
    }
}
