use chrono::{DateTime, Utc};
use http_types::Url;
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
    pub homepage: Option<Url>,
    pub bin: Option<Bin>,
    #[serde(rename = "_npmUser")]
    pub npm_user: Option<Human>,
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
    pub deprecated: Option<String>,

    #[serde(flatten)]
    pub rest: HashMap<String, Value>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Bin {
    Str(String),
    Hash(HashMap<String, String>),
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Human {
    pub name: String,
    pub email: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Dist {
    pub shasum: String,
    pub tarball: Url,

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
