//! Configuration loader for Orogene config files.

use std::{
    collections::{HashMap, HashSet},
    ffi::OsString,
    path::PathBuf,
};

pub use clap::{ArgMatches, Command};
pub use config::Config as OroConfig;
use config::{builder::DefaultState, ConfigBuilder, Environment, File};
use kdl_source::KdlFormat;
use miette::Result;

use error::OroConfigError;

mod error;
mod kdl_source;

pub trait OroConfigLayerExt {
    fn with_negations(self) -> Self;
    fn layered_matches(self, config: &OroConfig) -> Result<ArgMatches>;
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
                clap::Arg::new(negated.clone())
                    .long(negated)
                    .global(opt.is_global_set())
                    .hide(true)
                    .action(clap::ArgAction::SetTrue)
                    .overrides_with(opt.get_id())
            })
            .collect::<Vec<_>>();
        // Add the negations
        self.args(negations)
    }

    fn layered_matches(mut self, config: &OroConfig) -> Result<ArgMatches> {
        // Add the negations
        self = self.with_negations();
        let mut short_opts = HashMap::new();
        let mut long_opts = HashSet::new();
        for opt in self.get_arguments() {
            if let Some(short) = opt.get_short() {
                short_opts.insert(short, (*opt).clone());
            }
            if let Some(long) = opt.get_long() {
                long_opts.insert(long.to_string());
            }
        }
        let mut args = std::env::args_os().collect::<Vec<_>>();
        let matches = self.clone().get_matches_from(&args);
        for opt in long_opts {
            if matches.value_source(&opt) != Some(clap::parser::ValueSource::CommandLine) {
                if let Ok(value) = config.get_string(&opt) {
                    if !args.contains(&OsString::from(format!("--no-{}", opt))) {
                        args.push(OsString::from(format!("--{}", opt)));
                        args.push(OsString::from(value));
                    }
                }
            }
        }
        // Check for missing flags and inject config options into
        Ok(self.get_matches_from(args))
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
        let file = dir.path().join("ororc.toml");
        fs::write(&file, "store = \"hello world\"").into_diagnostic()?;
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
