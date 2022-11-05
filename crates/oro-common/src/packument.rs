use derive_builder::Builder;
use node_semver::Version;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use url::Url;

use crate::{Manifest, PersonField};

/// A serializable representation of a Packument -- the toplevel metadata
/// object containing information about package versions, dist-tags, etc.
#[derive(Builder, Default, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Packument {
    #[serde(default)]
    pub versions: HashMap<Version, VersionMetadata>,
    #[serde(default)]
    pub time: HashMap<String, String>,
    #[serde(default, rename = "dist-tags")]
    pub tags: HashMap<String, Version>,
    #[serde(flatten)]
    pub rest: HashMap<String, Value>,
}

/// A manifest for an individual package version.
#[derive(Clone, Default, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct VersionMetadata {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub maintainers: Vec<PersonField>,
    #[serde(rename = "_npmUser", skip_serializing_if = "Option::is_none")]
    pub npm_user: Option<NpmUser>,
    #[serde(default)]
    pub dist: Dist,
    #[serde(rename = "_hasShrinkwrap", skip_serializing_if = "Option::is_none")]
    pub has_shrinkwrap: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub deprecated: Option<String>,

    #[serde(flatten)]
    pub manifest: Manifest,
}

/// Representation for the `bin` field in package manifests.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Bin {
    Str(String),
    Hash(HashMap<String, String>),
}

/// Represents a human!
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct NpmUser {
    pub name: String,
    pub email: Option<String>,
}

/// Distribution information for a particular package version.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Dist {
    pub shasum: Option<String>,
    pub tarball: Option<Url>,

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
