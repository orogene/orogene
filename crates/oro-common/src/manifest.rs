use std::{collections::HashMap, path::PathBuf};

use derive_builder::Builder;
use indexmap::IndexMap;
use node_semver::{Range, Version};
use serde::{Deserialize, Deserializer, Serialize};
use serde_json::Value;

use crate::{CorgiVersionMetadata, VersionMetadata};

#[derive(Clone, Default, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CorgiManifest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<Version>,
    #[serde(default, skip_serializing_if = "IndexMap::is_empty")]
    pub dependencies: IndexMap<String, String>,
    #[serde(default, skip_serializing_if = "IndexMap::is_empty")]
    pub dev_dependencies: IndexMap<String, String>,
    #[serde(default, skip_serializing_if = "IndexMap::is_empty")]
    pub optional_dependencies: IndexMap<String, String>,
    #[serde(default, skip_serializing_if = "IndexMap::is_empty")]
    pub peer_dependencies: IndexMap<String, String>,
    #[serde(default, alias = "bundleDependencies", alias = "bundledDependencies")]
    pub bundled_dependencies: Option<BundledDependencies>,
}

#[derive(Builder, Default, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Manifest {
    /// The name of the package.
    ///
    /// If this is missing, it usually indicates that this package exists only to
    /// describe a workspace, similar to Cargo's notion of a "virtual manifest".
    #[builder(setter(into, strip_option), default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,

    /// The version of the package.
    ///
    /// Package managers generally require this to be populated to actually publish
    /// the package, but will tolerate its absence during local development.
    #[builder(setter(strip_option), default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<Version>,

    #[builder(setter(into, strip_option), default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    #[builder(setter(into, strip_option), default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub homepage: Option<String>,

    #[serde(default, alias = "licence", skip_serializing_if = "Option::is_none")]
    #[builder(setter(into, strip_option), default)]
    pub license: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    #[builder(setter(strip_option), default)]
    pub bugs: Option<Bugs>,

    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[builder(default)]
    pub keywords: Vec<String>,

    /// Information about the names and locations of binaries this package provides.
    ///
    /// Use [`crate::BuildManifest::from_manifest`][] to get a normalized version
    /// of this field (and other related fields).
    #[builder(setter(strip_option), default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bin: Option<Bin>,

    #[serde(skip_serializing_if = "Option::is_none")]
    #[builder(setter(strip_option), default)]
    pub author: Option<PersonField>,

    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[builder(default)]
    pub contributors: Vec<PersonField>,

    #[serde(skip_serializing_if = "Option::is_none")]
    #[builder(default)]
    pub files: Option<Vec<String>>,

    #[builder(setter(into, strip_option), default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub main: Option<String>,

    #[builder(setter(strip_option), default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub man: Option<Man>,

    #[serde(skip, default)]
    #[builder(default)]
    pub directories: Option<Directories>,

    #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
    #[builder(setter(into, strip_option), default)]
    pub module_type: Option<String>,

    #[builder(setter(strip_option), default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exports: Option<Exports>,

    #[builder(setter(strip_option), default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub imports: Option<Imports>,

    /// Information about the repository this project is hosted in.
    ///
    /// [`Repository::Str`][] can contain many different formats (or plain garbage),
    /// we recommend trying to `.parse()` it as oro-package-spec's GitInfo type,
    /// as it understands most of the relevant formats.
    #[builder(setter(strip_option), default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub repository: Option<Repository>,

    /// Information about build scripts the package uses.
    ///
    /// Use [`crate::BuildManifest::from_manifest`][] to get a normalized version
    /// of this field (and other related fields).
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    #[builder(default)]
    pub scripts: HashMap<String, String>,

    #[builder(setter(strip_option), default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub config: Option<Value>,

    // NOTE: using object_or_bust here because lodash has `"engines": []` in
    // some versions? This is obviously obnoxious, but we're playing
    // whack-a-mole here.
    #[serde(
        default,
        deserialize_with = "object_or_bust",
        skip_serializing_if = "HashMap::is_empty"
    )]
    #[builder(default)]
    pub engines: HashMap<String, Range>,

    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[builder(default)]
    pub os: Vec<String>,

    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[builder(default)]
    pub cpu: Vec<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    #[builder(setter(strip_option), default)]
    pub private: Option<bool>,

    #[serde(
        default,
        rename = "publishConfig",
        skip_serializing_if = "HashMap::is_empty"
    )]
    #[builder(default)]
    pub publish_config: HashMap<String, Value>,

    // Deps
    #[serde(default, skip_serializing_if = "IndexMap::is_empty")]
    #[builder(default)]
    pub dependencies: IndexMap<String, String>,

    #[serde(default, skip_serializing_if = "IndexMap::is_empty")]
    #[builder(default)]
    pub dev_dependencies: IndexMap<String, String>,

    #[serde(default, skip_serializing_if = "IndexMap::is_empty")]
    #[builder(default)]
    pub optional_dependencies: IndexMap<String, String>,

    #[serde(default, skip_serializing_if = "IndexMap::is_empty")]
    #[builder(default)]
    pub peer_dependencies: IndexMap<String, String>,

    #[serde(
        default,
        alias = "bundleDependencies",
        alias = "bundledDependencies",
        skip_serializing_if = "empty_bundled_dependencies"
    )]
    #[builder(default)]
    pub bundled_dependencies: Option<BundledDependencies>,

    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[builder(default)]
    pub workspaces: Vec<String>,

    #[serde(flatten, default, skip_serializing_if = "HashMap::is_empty")]
    #[builder(default)]
    pub _rest: HashMap<String, Value>,
}

impl From<CorgiManifest> for Manifest {
    fn from(value: CorgiManifest) -> Self {
        Manifest {
            name: value.name,
            version: value.version,
            dependencies: value.dependencies,
            dev_dependencies: value.dev_dependencies,
            optional_dependencies: value.optional_dependencies,
            peer_dependencies: value.peer_dependencies,
            bundled_dependencies: value.bundled_dependencies,
            ..Default::default()
        }
    }
}

impl From<Manifest> for CorgiManifest {
    fn from(value: Manifest) -> Self {
        CorgiManifest {
            name: value.name,
            version: value.version,
            dependencies: value.dependencies,
            dev_dependencies: value.dev_dependencies,
            optional_dependencies: value.optional_dependencies,
            peer_dependencies: value.peer_dependencies,
            bundled_dependencies: value.bundled_dependencies,
        }
    }
}

impl From<CorgiManifest> for CorgiVersionMetadata {
    fn from(value: CorgiManifest) -> Self {
        CorgiVersionMetadata {
            manifest: value,
            ..Default::default()
        }
    }
}

impl From<Manifest> for VersionMetadata {
    fn from(value: Manifest) -> Self {
        VersionMetadata {
            manifest: value,
            ..Default::default()
        }
    }
}

fn object_or_bust<'de, D, K, V>(deserializer: D) -> std::result::Result<HashMap<K, V>, D::Error>
where
    D: Deserializer<'de>,
    K: std::hash::Hash + Eq + Deserialize<'de>,
    V: Deserialize<'de>,
{
    let val: ObjectOrBust<K, V> = Deserialize::deserialize(deserializer)?;
    if let ObjectOrBust::Object(map) = val {
        Ok(map)
    } else {
        Ok(HashMap::new())
    }
}

#[derive(Deserialize)]
#[serde(untagged)]
enum ObjectOrBust<K, V>
where
    K: std::hash::Hash + Eq,
{
    Object(HashMap<K, V>),
    Value(serde_json::Value),
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(untagged)]
pub enum BundledDependencies {
    All(bool),
    Some(Vec<String>),
}

fn empty_bundled_dependencies(bundled: &Option<BundledDependencies>) -> bool {
    match bundled {
        None => true,
        Some(BundledDependencies::All(all)) => !all,
        Some(BundledDependencies::Some(deps)) => deps.is_empty(),
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(untagged)]
pub enum Bugs {
    Str(String),
    Obj {
        url: Option<String>,
        email: Option<String>,
    },
}

/// Represents a human!
#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(untagged)]
pub enum PersonField {
    Str(String),
    Obj(Person),
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
pub struct Person {
    pub name: Option<String>,
    pub email: Option<String>,
    pub url: Option<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Deserialize, Serialize)]
pub struct Directories {
    pub bin: Option<PathBuf>,
    pub man: Option<PathBuf>,
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(untagged)]
pub enum Bin {
    Str(String),
    Hash(HashMap<String, PathBuf>),
    Array(Vec<PathBuf>),
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(untagged)]
pub enum Man {
    Str(String),
    Vec(Vec<String>),
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(untagged)]
pub enum Exports {
    Str(String),
    Vec(Vec<String>),
    Obj(HashMap<String, Exports>),
    Other(Value),
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(untagged)]
pub enum Imports {
    Str(String),
    Vec(Vec<String>),
    Obj(HashMap<String, Imports>),
    Other(Value),
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(untagged)]
pub enum Repository {
    Str(String),
    Obj {
        #[serde(rename = "type")]
        repo_type: Option<String>,
        url: Option<String>,
        directory: Option<String>,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    use miette::{IntoDiagnostic, Result};
    use pretty_assertions::assert_eq;

    #[test]
    fn basic_from_json() -> Result<()> {
        let string = r#"
{
    "name": "hello",
    "version": "1.2.3",
    "description": "description",
    "homepage": "https://foo.dev",
    "devDependencies": {
        "foo": "^3.2.1"
    }
}
        "#;
        let mut deps = IndexMap::new();
        deps.insert(String::from("foo"), String::from("^3.2.1"));
        let parsed = serde_json::from_str::<Manifest>(string).into_diagnostic()?;
        assert_eq!(
            parsed,
            ManifestBuilder::default()
                .name("hello")
                .version("1.2.3".parse()?)
                .description("description")
                .homepage("https://foo.dev")
                .dev_dependencies(deps)
                .build()
                .unwrap()
        );
        Ok(())
    }

    #[test]
    fn empty() -> Result<()> {
        let string = "{}";
        let parsed = serde_json::from_str::<Manifest>(string).into_diagnostic()?;
        assert_eq!(parsed, ManifestBuilder::default().build().unwrap());
        Ok(())
    }

    #[test]
    fn string_props() -> Result<()> {
        let string = r#"
{
    "name": "hello",
    "description": "description",
    "homepage": "https://foo.dev",
    "license": "Parity-7.0",
    "main": "index.js",
    "keywords": ["foo", "bar"],
    "files": ["*.js"],
    "os": ["windows", "darwin"],
    "cpu": ["x64"],
    "bundleDependencies": [
        "mydep"
    ],
    "workspaces": [
        "packages/*"
    ]
}
        "#;
        let parsed = serde_json::from_str::<Manifest>(string).into_diagnostic()?;
        assert_eq!(
            parsed,
            ManifestBuilder::default()
                .name("hello")
                .description("description")
                .homepage("https://foo.dev")
                .license("Parity-7.0")
                .main("index.js")
                .keywords(vec!["foo".into(), "bar".into()])
                .files(Some(vec!["*.js".into()]))
                .os(vec!["windows".into(), "darwin".into()])
                .cpu(vec!["x64".into()])
                .bundled_dependencies(Some(BundledDependencies::Some(vec!["mydep".into()])))
                .workspaces(vec!["packages/*".into()])
                .build()
                .unwrap()
        );
        Ok(())
    }

    #[test]
    fn array_engines() -> Result<()> {
        let string = r#"
{
    "engines": []
}
        "#;
        let parsed = serde_json::from_str::<Manifest>(string).into_diagnostic()?;
        assert_eq!(
            parsed,
            ManifestBuilder::default()
                .engines(HashMap::new())
                .build()
                .unwrap()
        );
        Ok(())
    }

    #[test]
    fn licence_alias() -> Result<()> {
        let string = r#"
{
    "licence": "Parity-7.0"
}
        "#;
        let parsed = serde_json::from_str::<Manifest>(string).into_diagnostic()?;
        assert_eq!(
            parsed,
            ManifestBuilder::default()
                .license("Parity-7.0")
                .build()
                .unwrap()
        );
        Ok(())
    }

    #[test]
    fn parse_version() -> Result<()> {
        let string = r#"
{
    "version": "1.2.3"
}
        "#;
        let parsed = serde_json::from_str::<Manifest>(string).into_diagnostic()?;
        assert_eq!(
            parsed,
            ManifestBuilder::default()
                .version("1.2.3".parse()?)
                .build()
                .unwrap()
        );

        let string = r#"
{
    "version": "invalid"
}
        "#;
        let parsed = serde_json::from_str::<Manifest>(string);
        assert!(parsed.is_err());
        Ok(())
    }

    #[test]
    fn bool_props() -> Result<()> {
        let string = r#"
{
    "private": true
}
        "#;
        let parsed = serde_json::from_str::<Manifest>(string).into_diagnostic()?;
        assert_eq!(
            parsed,
            ManifestBuilder::default().private(true).build().unwrap()
        );
        Ok(())
    }
}
