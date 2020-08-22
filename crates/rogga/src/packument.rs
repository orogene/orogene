use chrono::{DateTime, Utc};
use http_types::Url;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

use oro_node_semver::Version;

/// A serializable representation of a Packument -- the toplevel metadata
/// object containing information about package versions, dist-tags, etc.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Packument {
    pub name: Option<String>,
    pub description: Option<String>,
    pub versions: HashMap<Version, Manifest>,
    pub author: Option<Human>,
    #[serde(default)]
    pub time: HashMap<String, DateTime<Utc>>,
    #[serde(default, rename = "dist-tags")]
    pub tags: HashMap<String, Version>,
    #[serde(default)]
    pub maintainers: Vec<Human>,
    #[serde(default)]
    pub users: HashMap<String, bool>,

    #[serde(flatten)]
    pub rest: HashMap<String, Value>,
}

/// A manifest for an individual package version.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Manifest {
    pub name: String,
    pub version: Version,
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

/// Representation for the `bin` field in package manifests.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Bin {
    Str(String),
    Hash(HashMap<String, String>),
}

/// Represents a human!
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Human {
    pub name: String,
    pub email: Option<String>,
}

/// Distribution information for a particular package version.
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
