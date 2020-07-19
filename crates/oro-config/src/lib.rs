use std::path::PathBuf;

pub use config::Config as OroConfig;
use config::{ConfigError, Environment, File};
use directories::ProjectDirs;

pub struct OroConfigOptions {
    global: bool,
    env: bool,
    global_config_file: Option<PathBuf>,
}

impl Default for OroConfigOptions {
    fn default() -> Self {
        OroConfigOptions {
            global: true,
            env: true,
            // TODO: Move this to main app.
            global_config_file: ProjectDirs::from("", "", "orogene")
                .map(|d| d.config_dir().to_owned().join("ororc.toml")),
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

    pub fn global_config_file(mut self, file: Option<PathBuf>) -> Self {
        self.global_config_file = file;
        self
    }

    pub fn load(self) -> Result<OroConfig, ConfigError> {
        let mut c = OroConfig::new();
        if self.global {
            if let Some(config_file) = self.global_config_file {
                let path = config_file.display().to_string();
                c.merge(File::with_name(&path[..]).required(false))?;
            }
        }
        if self.env {
            c.merge(Environment::with_prefix("oro_config"))?;
        }
        Ok(c)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::env;
    use std::fs;

    use anyhow::Result;
    use tempfile::tempdir;

    #[test]
    fn env_configs() -> Result<()> {
        let dir = tempdir()?;
        env::set_var("ORO_CONFIG_STORE", dir.path().display().to_string());
        let config = OroConfigOptions::new().global(false).load()?;
        env::remove_var("ORO_CONFIG_STORE");
        assert_eq!(config.get_str("store")?, dir.path().display().to_string());
        Ok(())
    }

    #[test]
    fn global_config() -> Result<()> {
        let dir = tempdir()?;
        let file = dir.path().join("ororc.toml");
        fs::write(&file, "store = \"hello world\"")?;
        let config = OroConfigOptions::new()
            .env(false)
            .global_config_file(Some(file.to_owned()))
            .load()?;
        assert_eq!(config.get_str("store")?, String::from("hello world"));
        Ok(())
    }

    #[test]
    fn missing_config() -> Result<()> {
        let config = OroConfigOptions::new().global(false).env(false).load()?;
        assert!(config.get_str("store").is_err());
        Ok(())
    }
}
