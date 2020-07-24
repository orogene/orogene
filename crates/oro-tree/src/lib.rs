use memmap::MmapOptions;
use serde::{Deserialize, de::{Deserializer, Error as SerdeError}};
use std::fs::File;
use std::{collections::HashMap, path::Path};
use thiserror::Error;
use ssri::Integrity;
use semver::{Version, VersionReq};

fn default_as_false() -> bool {
    false
}

fn parse_integrity<'de, D>(deserializer: D) -> Result<Integrity, D::Error> where D: Deserializer<'de> {
    let s: &str = Deserialize::deserialize(deserializer)?;
    s.parse().map_err(D::Error::custom)
}

fn parse_version<'de, D>(deserializer: D) -> Result<Version, D::Error> where D: Deserializer<'de> {
    let s: &str = Deserialize::deserialize(deserializer)?;
    s.parse().map_err(D::Error::custom)
}

fn parse_version_req<'de, D>(deserializer: D) -> Result<VersionReq, D::Error> where D: Deserializer<'de> {
    let s: &str = Deserialize::deserialize(deserializer)?;
    s.parse().map_err(D::Error::custom)
}

fn deserialize_requires<'de, D>(deserializer: D) -> Result<HashMap<String, VersionReq>, D::Error>
where
    D: Deserializer<'de>,
{
    #[derive(Deserialize)]
    struct Wrapper(#[serde(deserialize_with = "parse_version_req")] VersionReq);

    let v = HashMap::<String, Wrapper>::deserialize(deserializer)?;
    Ok(v.into_iter().map(|(k, Wrapper(v))| (k, v)).collect())
}

#[derive(Deserialize, Debug)]
pub struct Dependency {

    #[serde(deserialize_with = "parse_version")]
    version: Version,

    #[serde(deserialize_with = "parse_integrity")]
    integrity: Integrity,

    #[serde(default = "default_as_false")]
    dev: bool,

    #[serde(default = "default_as_false")]
    bundled: bool,

    #[serde(default = "default_as_false")]
    optional: bool,

    resolved: Option<String>,

    #[serde(default, deserialize_with = "deserialize_requires")]
    requires: HashMap<String, VersionReq>,

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
