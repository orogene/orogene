use memmap::MmapOptions;
use node_semver::Version;
use serde::{
    de::{Deserializer, Error as SerdeError},
    Deserialize,
};
use ssri::Integrity;
use std::fs::File;
use std::{collections::HashMap, path::Path};
use thiserror::Error;
use url::Url;

fn parse_integrity<'de, D>(deserializer: D) -> Result<Option<Integrity>, D::Error>
where
    D: Deserializer<'de>,
{
    let s: &str = Deserialize::deserialize(deserializer)?;
    s.parse().map(Option::Some).map_err(D::Error::custom)
}

#[derive(Deserialize, Debug)]
pub struct Package {
    pub version: String,
    #[serde(default)]
    #[serde(deserialize_with = "parse_integrity")]
    pub integrity: Option<Integrity>,
    #[serde(default)]
    pub dev: bool,
    #[serde(default)]
    pub bundled: bool,
    #[serde(default)]
    pub optional: bool,
    #[serde(default)]
    pub resolved: Option<Url>,
    #[serde(default)]
    pub requires: HashMap<String, String>,
    #[serde(default)]
    pub dependencies: HashMap<String, Package>,
}

#[derive(Deserialize, Debug)]
pub struct PkgLock {
    pub name: String,
    pub version: Version,
    #[serde(default)]
    pub requires: bool,
    #[serde(default)]
    pub dependencies: HashMap<String, Package>,
}

#[derive(Error, Debug)]
pub enum Error {
    #[error(transparent)]
    FileNotFound {
        #[from]
        source: std::io::Error,
    },
    #[error(transparent)]
    InvalidJson {
        #[from]
        source: serde_json::error::Error,
    },
}

pub fn read<P: AsRef<Path>>(path: P) -> Result<PkgLock, Error> {
    let file = File::open(path)?;
    let mmap = unsafe { MmapOptions::new().map(&file)? };
    let package = serde_json::from_slice(&mmap[..])?;
    Ok(package)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_parses_a_small_package_lock_json_file() {
        assert!(read("fixtures/small-package-lock.json").is_ok());
    }
}
