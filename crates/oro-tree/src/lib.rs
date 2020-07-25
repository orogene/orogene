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

fn parse_integrity<'de, D>(deserializer: D) -> Result<Integrity, D::Error>
where
    D: Deserializer<'de>,
{
    let s: &str = Deserialize::deserialize(deserializer)?;
    s.parse().map_err(D::Error::custom)
}

fn parse_version<'de, D>(deserializer: D) -> Result<Version, D::Error>
where
    D: Deserializer<'de>,
{
    let s: &str = Deserialize::deserialize(deserializer)?;
    s.parse().map_err(D::Error::custom)
}

#[derive(Deserialize, Debug)]
pub struct Dependency {
    #[serde(deserialize_with = "parse_version")]
    version: Version,

    #[serde(deserialize_with = "parse_integrity")]
    integrity: Integrity,

    #[serde(default)]
    dev: bool,

    #[serde(default)]
    bundled: bool,

    #[serde(default)]
    optional: bool,

    resolved: Option<String>,

    #[serde(default)]
    requires: HashMap<String, String>,

    #[serde(default)]
    dependencies: HashMap<String, Dependency>,
}

#[derive(Deserialize, Debug)]
pub struct Package {
    name: String,

    #[serde(deserialize_with = "parse_version")]
    version: Version,
    requires: bool,
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
