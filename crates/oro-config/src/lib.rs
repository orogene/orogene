use std::path::PathBuf;

pub use clap::ArgMatches;
pub use config::Config as OroConfig;
use config::{ConfigError, Environment, File};
use miette::{Diagnostic, Result};
use thiserror::Error;

pub use oro_config_derive::*;

pub trait OroConfigLayer {
    fn layer_config(&mut self, _matches: &ArgMatches, _config: &OroConfig) -> Result<()> {
        Ok(())
    }
}

#[derive(Debug, Error, Diagnostic)]
pub enum OroConfigError {
    #[error(transparent)]
    #[diagnostic(code(config::error))]
    ConfigError(#[from] ConfigError),

    #[error(transparent)]
    #[diagnostic(code(config::error))]
    ConfigParseError(#[from] Box<dyn std::error::Error + Send + Sync>),
}

pub struct OroConfigOptions {
    global: bool,
    env: bool,
    pkg_root: Option<PathBuf>,
    global_config_file: Option<PathBuf>,
}

impl Default for OroConfigOptions {
    fn default() -> Self {
        OroConfigOptions {
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

    pub fn load(self) -> Result<OroConfig> {
        let mut c = OroConfig::new();
        if self.global {
            if let Some(config_file) = self.global_config_file {
                let path = config_file.display().to_string();
                c.merge(File::with_name(&path[..]).required(false))
                    .map_err(OroConfigError::ConfigError)?;
            }
        }
        if self.env {
            c.merge(Environment::with_prefix("oro_config"))
                .map_err(OroConfigError::ConfigError)?;
        }
        if let Some(root) = self.pkg_root {
            c.merge(File::with_name(&root.join("ororc").display().to_string()).required(false))
                .map_err(OroConfigError::ConfigError)?;
            c.merge(File::with_name(&root.join(".ororc").display().to_string()).required(false))
                .map_err(OroConfigError::ConfigError)?;
            c.merge(
                File::with_name(&root.join("ororc.toml").display().to_string()).required(false),
            )
            .map_err(OroConfigError::ConfigError)?;
            c.merge(
                File::with_name(&root.join(".ororc.toml").display().to_string()).required(false),
            )
            .map_err(OroConfigError::ConfigError)?;
        }
        Ok(c)
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
            config.get_str("store").into_diagnostic()?,
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
            config.get_str("store").into_diagnostic()?,
            String::from("hello world")
        );
        Ok(())
    }

    #[test]
    fn missing_config() -> Result<()> {
        let config = OroConfigOptions::new().global(false).env(false).load()?;
        assert!(config.get_str("store").is_err());
        Ok(())
    }
}
