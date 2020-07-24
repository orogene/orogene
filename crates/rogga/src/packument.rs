use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Packument {
    pub author: Option<Human>,
    pub name: Option<String>,
    pub description: Option<String>,
    pub versions: HashMap<semver::Version, Version>,
    #[serde(default)]
    pub time: HashMap<String, DateTime<Utc>>,
    #[serde(rename = "dist-tags")]
    pub tags: HashMap<String, semver::Version>,
    #[serde(default)]
    pub maintainers: Vec<Human>,
    #[serde(default)]
    pub users: HashMap<String, bool>,

    #[serde(flatten)]
    pub rest: HashMap<String, Value>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Version {
    pub name: String,
    pub version: semver::Version,
    pub description: Option<String>,
    pub license: Option<String>,
    pub licence: Option<String>,
    #[serde(default)]
    pub dependencies: HashMap<String, String>,
    #[serde(default, rename = "devDependencies")]
    pub dev_dependencies: HashMap<String, String>,
    #[serde(default, rename = "optionalDependencies")]
    pub optional_dependencies: HashMap<String, String>,
    #[serde(default, rename = "peerDependencies")]
    pub peer_dependencies: HashMap<String, String>,
    pub dist: Dist,
    #[serde(rename = "_hasShrinkwrap")]
    pub has_shrinkwrap: Option<bool>,
    #[serde(default)]
    pub keywords: Vec<String>,

    #[serde(flatten)]
    pub rest: HashMap<String, Value>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Human {
    pub name: String,
    pub email: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Dist {
    pub shasum: String,
    pub tarball: String,

    pub integrity: Option<String>,
    #[serde(rename = "fileCount")]
    pub file_count: Option<usize>,
    #[serde(rename = "unpackedSize")]
    pub unpacked_size: Option<usize>,
    #[serde(rename = "npm-signature")]
    pub npm_signature: Option<String>,

    #[serde(flatten)]
    pub rest: HashMap<String, Value>,
}
