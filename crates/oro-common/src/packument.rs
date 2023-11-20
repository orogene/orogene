use derive_builder::Builder;
use node_semver::Version;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde_json::Value;
use std::{collections::HashMap, fmt::Display};
use url::Url;

use crate::{CorgiManifest, Manifest, PersonField};

/// A serializable representation of a Packument -- the toplevel metadata
/// object containing information about package versions, dist-tags, etc.
///
/// This version is a reduced-size packument that only contains fields from
/// "Corgi" packuments (or will only (de)serialize those specific fields).
#[derive(Clone, Default, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CorgiPackument {
    #[serde(default)]
    pub versions: HashMap<Version, CorgiVersionMetadata>,
    #[serde(default, rename = "dist-tags")]
    pub tags: HashMap<String, Version>,
}

/// A serializable representation of a Packument -- the toplevel metadata
/// object containing information about package versions, dist-tags, etc.
#[derive(Builder, Default, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Packument {
    #[serde(default, rename = "_id", skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(default)]
    pub versions: HashMap<Version, VersionMetadata>,
    #[serde(default)]
    pub time: HashMap<String, String>,
    #[serde(default, rename = "dist-tags")]
    pub tags: HashMap<String, Version>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub access: Option<Access>,
    #[serde(
        default,
        rename = "_attachments",
        skip_serializing_if = "HashMap::is_empty"
    )]
    pub attachments: HashMap<String, Attachments>,
    #[serde(flatten)]
    pub rest: HashMap<String, Value>,
}

impl From<CorgiPackument> for Packument {
    fn from(value: CorgiPackument) -> Self {
        Packument {
            versions: value
                .versions
                .into_iter()
                .map(|(k, v)| (k, v.into()))
                .collect(),
            tags: value.tags,
            ..Default::default()
        }
    }
}

impl From<Packument> for CorgiPackument {
    fn from(value: Packument) -> Self {
        CorgiPackument {
            versions: value
                .versions
                .into_iter()
                .map(|(k, v)| (k, v.into()))
                .collect(),
            tags: value.tags,
        }
    }
}

/// A manifest for an individual package version.
///
/// This version is a reduced-size VersionMetadata that only contains fields
/// from "Corgi" packuments (or will only (de)serialize those specific
/// fields).
#[derive(Clone, Default, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CorgiVersionMetadata {
    #[serde(default)]
    pub dist: CorgiDist,
    #[serde(rename = "_hasShrinkwrap", skip_serializing_if = "Option::is_none")]
    pub has_shrinkwrap: Option<bool>,
    #[serde(flatten)]
    pub manifest: CorgiManifest,
    #[serde(
        default,
        deserialize_with = "deserialize_deprecation_info",
        skip_serializing_if = "Option::is_none"
    )]
    pub deprecated: Option<DeprecationInfo>,
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
    #[serde(
        default,
        deserialize_with = "deserialize_deprecation_info",
        skip_serializing_if = "Option::is_none"
    )]
    pub deprecated: Option<DeprecationInfo>,

    #[serde(default, rename = "_id", skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,

    #[serde(flatten)]
    pub manifest: Manifest,
}

impl From<CorgiVersionMetadata> for VersionMetadata {
    fn from(value: CorgiVersionMetadata) -> Self {
        VersionMetadata {
            dist: value.dist.into(),
            has_shrinkwrap: value.has_shrinkwrap,
            manifest: value.manifest.into(),
            ..Default::default()
        }
    }
}

impl From<VersionMetadata> for CorgiVersionMetadata {
    fn from(value: VersionMetadata) -> Self {
        CorgiVersionMetadata {
            dist: value.dist.into(),
            has_shrinkwrap: value.has_shrinkwrap,
            manifest: value.manifest.into(),
            deprecated: value.deprecated,
        }
    }
}

impl From<CorgiVersionMetadata> for CorgiManifest {
    fn from(value: CorgiVersionMetadata) -> Self {
        value.manifest
    }
}

impl From<VersionMetadata> for Manifest {
    fn from(value: VersionMetadata) -> Self {
        value.manifest
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
enum StringOrBool {
    String(String),
    Bool(bool),
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

#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
pub enum Access {
    Restricted,
    Public,
}

impl Serialize for Access {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self {
            Self::Public => serializer.serialize_str("public"),
            Self::Restricted => serializer.serialize_str("restricted"),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Attachments {
    pub content_type: String,
    pub data: String,
    pub length: usize,
}

/// Represents the deprecation state of a package.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DeprecationInfo {
    Reason(String),
    UnknownReason,
}

impl Display for DeprecationInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Reason(s) => write!(f, "{:?}", s),
            Self::UnknownReason => write!(f, "Unknown Reason"),
        }
    }
}

impl Serialize for DeprecationInfo {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self {
            DeprecationInfo::Reason(s) => serializer.serialize_str(s),
            DeprecationInfo::UnknownReason => serializer.serialize_bool(true),
        }
    }
}

fn deserialize_deprecation_info<'de, D>(
    deserializer: D,
) -> std::result::Result<Option<DeprecationInfo>, D::Error>
where
    D: Deserializer<'de>,
{
    let val: StringOrBool = Deserialize::deserialize(deserializer)?;
    Ok(match val {
        StringOrBool::String(s) => Some(DeprecationInfo::Reason(s)),
        StringOrBool::Bool(b) => {
            if b {
                Some(DeprecationInfo::UnknownReason)
            } else {
                None
            }
        }
    })
}

/// Distribution information for a particular package version.
///
/// This version is a reduced-size CorgiDist that only contains fields from
/// "Corgi" packuments (or will only (de)serialize those specific fields).
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CorgiDist {
    pub shasum: Option<String>,
    pub tarball: Option<Url>,
    pub integrity: Option<String>,
    #[serde(rename = "npm-signature")]
    pub npm_signature: Option<String>,
}

/// Distribution information for a particular package version.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Dist {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub shasum: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tarball: Option<Url>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub integrity: Option<String>,
    #[serde(rename = "fileCount", skip_serializing_if = "Option::is_none")]
    pub file_count: Option<usize>,
    #[serde(rename = "unpackedSize", skip_serializing_if = "Option::is_none")]
    pub unpacked_size: Option<usize>,
    #[serde(rename = "npm-signature", skip_serializing_if = "Option::is_none")]
    pub npm_signature: Option<String>,

    #[serde(flatten)]
    pub rest: HashMap<String, Value>,
}

impl From<CorgiDist> for Dist {
    fn from(value: CorgiDist) -> Self {
        Dist {
            shasum: value.shasum,
            tarball: value.tarball,
            integrity: value.integrity,
            npm_signature: value.npm_signature,
            ..Default::default()
        }
    }
}

impl From<Dist> for CorgiDist {
    fn from(value: Dist) -> Self {
        CorgiDist {
            shasum: value.shasum,
            tarball: value.tarball,
            integrity: value.integrity,
            npm_signature: value.npm_signature,
        }
    }
}
