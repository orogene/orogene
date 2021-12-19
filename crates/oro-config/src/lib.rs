use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

pub use clap::ArgMatches;
pub use config::Config as OroConfig;
pub use config::Value as ConfigValue;
use config::{ConfigError, Environment, Source};
use kdl::{KdlNode, KdlValue};
use oro_common::miette::{self, Diagnostic, Result};
use oro_common::thiserror::{self, Error};

pub use oro_config_derive::*;

pub trait OroConfigLayer {
    fn layer_config(&mut self, _matches: &ArgMatches, _config: &OroConfig) -> Result<()> {
        Ok(())
    }
}

#[derive(Debug, Diagnostic, Error)]
pub enum OroConfigError {
    #[error(transparent)]
    #[diagnostic(code(config::error))]
    ConfigError(#[from] ConfigError),

    #[error("Error while parsing config file at {1}:\n\t{0}")]
    #[diagnostic(code(config::parse_error))]
    ConfigParseError(Box<dyn std::error::Error + Send + Sync>, String),
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

    pub fn load(self) -> Result<OroConfig, OroConfigError> {
        let mut c = OroConfig::new();
        if self.global {
            if let Some(config_file) = self.global_config_file {
                let path = config_file.display().to_string();
                if let Ok(str) = fs::read_to_string(&path[..]) {
                    let src = kdl::parse_document(str)
                        .map_err(|e| OroConfigError::ConfigParseError(Box::new(e), path))?;
                    c.merge(KdlDocument(src))
                        .map_err(OroConfigError::ConfigError)?;
                }
            }
        }
        if self.env {
            c.merge(Environment::with_prefix("orogene_config"))
                .map_err(OroConfigError::ConfigError)?;
        }
        if let Some(root) = self.pkg_root {
            if let Ok(str) = fs::read_to_string(&root.join("orogene.kdl")) {
                let src = kdl::parse_document(str).map_err(|e| {
                    OroConfigError::ConfigParseError(Box::new(e), root.display().to_string())
                })?;
                c.merge(KdlDocument(src))
                    .map_err(OroConfigError::ConfigError)?;
            }
            if let Ok(str) = fs::read_to_string(&root.join(".orogene.kdl")) {
                let src = kdl::parse_document(str)
                    .map_err(|e| OroConfigError::ConfigParseError(Box::new(e), root.display().to_string()))?;
                c.merge(KdlDocument(src))
                    .map_err(OroConfigError::ConfigError)?;
            }
        }
        Ok(c)
    }
}

#[derive(Debug, Clone)]
struct KdlDocument(Vec<KdlNode>);

impl KdlDocument {
    fn read_kdl_val(val: &KdlValue) -> ConfigValue {
        use KdlValue::*;
        match val {
            Int(x) => ConfigValue::new(None, *x),
            Float(x) => ConfigValue::new(None, *x),
            String(x) => ConfigValue::new(None, x.clone()),
            Boolean(x) => ConfigValue::new(None, *x),
            Null => None::<i64>.into(),
        }
    }

    fn node_value(node: &KdlNode) -> ConfigValue {
        if node.values.len() == 1 {
            KdlDocument::read_kdl_val(&node.values[0])
        } else if !node.values.is_empty() {
            node.values
                .iter()
                .map(KdlDocument::read_kdl_val)
                .collect::<Vec<_>>()
                .into()
        } else if !node.properties.is_empty() {
            let mut inner = HashMap::new();
            for (prop, val) in &node.properties {
                inner.insert(prop.clone(), KdlDocument::read_kdl_val(val));
            }
            inner.into()
        } else if !node.children.is_empty() {
            KdlDocument::children_table(&node.children).into()
        } else {
            None::<i64>.into()
        }
    }

    fn children_table(children: &[KdlNode]) -> HashMap<String, ConfigValue> {
        let mut table = HashMap::new();
        for child in children {
            table.insert(child.name.clone(), KdlDocument::node_value(child));
        }
        table
    }
}

impl Source for KdlDocument {
    fn clone_into_box(&self) -> Box<dyn Source + Send + Sync> {
        Box::new(self.clone())
    }

    fn collect(&self) -> Result<HashMap<String, ConfigValue>, ConfigError> {
        let mut hash = HashMap::new();
        for node in &self.0 {
            hash.insert(node.name.clone(), KdlDocument::node_value(node));
        }
        Ok(hash)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::env;
    use std::fs;

    use anyhow::Result;
    use pretty_assertions::assert_eq;
    use tempfile::tempdir;

    #[test]
    fn env_configs() -> Result<()> {
        let dir = tempdir()?;
        env::set_var("OROGENE_CONFIG_STORE", dir.path().display().to_string());
        let config = OroConfigOptions::new().global(false).load()?;
        env::remove_var("OROGENE_CONFIG_STORE");
        assert_eq!(config.get_str("store")?, dir.path().display().to_string());
        Ok(())
    }

    #[test]
    fn global_config() -> Result<()> {
        let dir = tempdir()?;
        let file = dir.path().join("orogene.kdl");
        fs::write(&file, "store \"hello world\"")?;
        let config = OroConfigOptions::new()
            .env(false)
            .global_config_file(Some(file))
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
