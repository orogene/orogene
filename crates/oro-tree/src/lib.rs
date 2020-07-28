use memmap::MmapOptions;
use semver::Version;
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
pub struct Dependency {
    version: Version,

    #[serde(default)]
    #[serde(deserialize_with = "parse_integrity")]
    integrity: Option<Integrity>,

    #[serde(default)]
    dev: bool,

    #[serde(default)]
    bundled: bool,

    #[serde(default)]
    optional: bool,

    #[serde(default)]
    resolved: Option<Url>,

    #[serde(default)]
    requires: HashMap<String, String>,

    #[serde(default)]
    dependencies: HashMap<String, Dependency>,
}

#[derive(Deserialize, Debug)]
pub struct Package {
    name: String,

    version: Version,

    #[serde(default)]
    requires: bool,

    #[serde(default)]
    dependencies: HashMap<String, Dependency>,
}

#[derive(Error, Debug)]
pub enum Error {
    #[error("file was not found")]
    FileNotFound {
        #[from]
        source: std::io::Error,
    },
    #[error("json was invalid")]
    InvalidJson {
        #[from]
        source: serde_json::error::Error,
    },
}

pub fn read<P: AsRef<Path>>(path: P) -> Result<Package, Error> {
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
