use memmap::MmapOptions;
use serde::Deserialize;
use std::fs::File;
use std::{collections::HashMap, path::Path};
use thiserror::Error;

fn default_as_false() -> bool {
    false
}

#[derive(Deserialize, Debug)]
pub struct Dependency {
    version: String,
    integrity: String,

    #[serde(default = "default_as_false")]
    dev: bool,

    #[serde(default = "default_as_false")]
    bundled: bool,

    #[serde(default = "default_as_false")]
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
    version: String,
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
