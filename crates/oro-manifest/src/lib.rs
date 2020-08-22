use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::str::FromStr;

use derive_builder::Builder;
use error::{Internal, Result};
use oro_node_semver::Version;
use serde::{Deserialize, Serialize};
use serde_json::Value;

pub use error::Error;

mod error;

#[derive(Builder, Default, Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OroManifest {
    #[builder(setter(into, strip_option), default)]
    pub name: Option<String>,

    #[builder(setter(strip_option), default)]
    pub version: Option<Version>,

    #[builder(setter(into, strip_option), default)]
    pub description: Option<String>,

    #[builder(setter(into, strip_option), default)]
    pub homepage: Option<String>,

    #[serde(default, alias = "licence")]
    #[builder(setter(into, strip_option), default)]
    pub license: Option<String>,

    #[serde(default)]
    #[builder(setter(strip_option), default)]
    pub browser: Option<bool>,

    #[serde(default)]
    #[builder(setter(strip_option), default)]
    pub bugs: Option<Bugs>,

    #[serde(default)]
    #[builder(default)]
    pub keywords: Vec<String>,

    #[builder(setter(strip_option), default)]
    pub bin: Option<Bin>,

    #[serde(default)]
    #[builder(setter(strip_option), default)]
    pub author: Option<PersonField>,

    #[serde(default)]
    #[builder(default)]
    pub contributors: Vec<PersonField>,

    #[serde(default)]
    #[builder(default)]
    pub files: Vec<String>,

    #[builder(setter(into, strip_option), default)]
    pub main: Option<String>,

    #[builder(setter(strip_option), default)]
    pub man: Option<Man>,

    #[serde(default)]
    #[builder(default)]
    pub directories: Option<Directories>,

    #[serde(rename = "type")]
    #[builder(setter(into, strip_option), default)]
    pub module_type: Option<String>,

    #[builder(setter(strip_option), default)]
    pub exports: Option<Exports>,

    #[builder(setter(strip_option), default)]
    pub imports: Option<Imports>,

    #[builder(setter(strip_option), default)]
    pub repository: Option<Repository>,

    #[serde(default)]
    #[builder(default)]
    pub scripts: HashMap<String, String>,

    #[builder(setter(strip_option), default)]
    pub config: Option<Value>,

    #[serde(default)]
    #[builder(default)]
    // TODO: VersionReq needs to support more syntaxes before we can make this a VersionReq value
    pub engines: HashMap<String, String>,

    #[serde(default)]
    #[builder(default)]
    pub os: Vec<String>,

    #[serde(default)]
    #[builder(default)]
    pub cpu: Vec<String>,

    #[serde(default)]
    #[builder(setter(strip_option), default)]
    pub private: Option<bool>,

    #[serde(default)]
    #[builder(default)]
    pub publish_config: HashMap<String, String>,

    // Deps
    #[serde(default)]
    #[builder(default)]
    pub dependencies: HashMap<String, String>,

    #[serde(default)]
    #[builder(default)]
    pub dev_dependencies: HashMap<String, String>,

    #[serde(default)]
    #[builder(default)]
    pub optional_dependencies: HashMap<String, String>,

    #[serde(default)]
    #[builder(default)]
    pub peer_dependencies: HashMap<String, String>,

    #[serde(default, alias = "bundleDependencies", alias = "bundledDependencies")]
    #[builder(default)]
    pub bundled_dependencies: Vec<String>,

    #[serde(default)]
    #[builder(default)]
    pub workspaces: Vec<String>,

    #[serde(flatten, default)]
    #[builder(default)]
    pub _rest: HashMap<String, Value>,
}

impl OroManifest {
    pub fn from_file<F: AsRef<Path>>(file: F) -> Result<OroManifest> {
        let data = fs::read(file.as_ref()).to_internal()?;
        Ok(serde_json::from_slice::<OroManifest>(&data[..]).to_internal()?)
    }

    pub fn update_file<F: AsRef<Path>>(&self, file: F) -> Result<()> {
        let file = file.as_ref();
        let manifest = serde_json::to_value(&self).to_internal()?;
        let data = fs::read(file).to_internal()?;
        let mut pkg_json = serde_json::from_slice::<Value>(&data[..]).to_internal()?;
        match (manifest, &mut pkg_json) {
            (Value::Object(ref mani_map), Value::Object(ref mut pkg_map)) => {
                for (key, val) in mani_map.iter() {
                    match val {
                        Value::Null => continue,
                        Value::Object(map) if map.is_empty() => continue,
                        Value::Array(vec) if vec.is_empty() => continue,
                        _ => pkg_map.insert(key.clone(), val.clone()),
                    };
                }
            }
            _ => return Err(Error::InvalidPackageFile(PathBuf::from(file))),
        };
        fs::write(file, serde_json::to_vec_pretty(&pkg_json).to_internal()?).to_internal()?;
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
#[serde(untagged)]
pub enum Bugs {
    Str(String),
    Obj {
        url: Option<String>,
        email: Option<String>,
    },
}

/// Represents a human!
#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
#[serde(untagged)]
pub enum PersonField {
    Str(String),
    Obj {
        name: Option<String>,
        email: Option<String>,
        url: Option<String>,
    },
}

impl PersonField {
    pub fn parse(&self) -> Result<Person> {
        match self {
            PersonField::Obj { name, email, url } => Ok(Person {
                name: name.clone(),
                email: email.clone(),
                url: url.clone(),
            }),
            PersonField::Str(s) => parser::parse_person(s.trim()),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Person {
    pub name: Option<String>,
    pub email: Option<String>,
    pub url: Option<String>,
}

impl FromStr for Person {
    type Err = Error;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        parser::parse_person(s.trim())
    }
}

#[derive(Clone, Debug, Default, PartialEq, Deserialize, Serialize)]
pub struct Directories {
    pub bin: Option<String>,
    pub man: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
#[serde(untagged)]
pub enum Bin {
    Str(String),
    Hash(HashMap<String, String>),
}

#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
#[serde(untagged)]
pub enum Man {
    Str(String),
    Vec(Vec<String>),
}

#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
#[serde(untagged)]
pub enum Exports {
    Str(String),
    Vec(Vec<String>),
    Obj(HashMap<String, Exports>),
}

#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
#[serde(untagged)]
pub enum Imports {
    Str(String),
    Vec(Vec<String>),
    Obj(HashMap<String, Imports>),
}

#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
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

mod parser {
    use super::*;

    use nom::bytes::complete::{take_till, take_till1};
    use nom::character::complete::char;
    use nom::combinator::{all_consuming, map, opt};
    use nom::error::{context, convert_error, ParseError, VerboseError};
    use nom::sequence::{delimited, preceded, tuple};
    use nom::{Err, IResult};

    pub fn parse_person<I: AsRef<str>>(input: I) -> Result<Person> {
        let input = &input.as_ref()[..];
        match all_consuming(person::<VerboseError<&str>>)(input) {
            Ok((_, arg)) => Ok(arg),
            Err(err) => Err(Error::ParsePersonError {
                input: input.into(),
                msg: match err {
                    Err::Error(e) => convert_error(input, e),
                    Err::Failure(e) => convert_error(input, e),
                    Err::Incomplete(_) => "More data was needed".into(),
                },
            }),
        }
    }

    fn person<'a, E>(input: &'a str) -> IResult<&'a str, Person, E>
    where
        E: ParseError<&'a str>,
    {
        context(
            "person",
            map(
                tuple((
                    opt(take_till1(|c| c == '<')),
                    opt(delimited(char('<'), take_till1(|c| c == '>'), char('>'))),
                    opt(preceded(
                        take_till(|c| c == '('),
                        delimited(char('('), take_till1(|c| c == ')'), char(')')),
                    )),
                )),
                |(name, email, url): (Option<&str>, Option<&str>, Option<&str>)| Person {
                    name: name.map(|n| n.trim().into()),
                    email: email.map(|e| e.trim().into()),
                    url: url.map(|u| u.trim().into()),
                },
            ),
        )(input)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::fs;

    use anyhow::Result;
    use pretty_assertions::assert_eq;
    use tempfile::tempdir;

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
        let mut deps = HashMap::new();
        deps.insert(String::from("foo"), String::from("^3.2.1"));
        let parsed = serde_json::from_str::<OroManifest>(&string)?;
        assert_eq!(
            parsed,
            OroManifestBuilder::default()
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
        let parsed = serde_json::from_str::<OroManifest>(&string)?;
        assert_eq!(parsed, OroManifestBuilder::default().build().unwrap());
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
        let parsed = serde_json::from_str::<OroManifest>(&string)?;
        assert_eq!(
            parsed,
            OroManifestBuilder::default()
                .name("hello")
                .description("description")
                .homepage("https://foo.dev")
                .license("Parity-7.0")
                .main("index.js")
                .keywords(vec!["foo".into(), "bar".into()])
                .files(vec!["*.js".into()])
                .os(vec!["windows".into(), "darwin".into()])
                .cpu(vec!["x64".into()])
                .bundled_dependencies(vec!["mydep".into()])
                .workspaces(vec!["packages/*".into()])
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
        let parsed = serde_json::from_str::<OroManifest>(&string)?;
        assert_eq!(
            parsed,
            OroManifestBuilder::default()
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
        let parsed = serde_json::from_str::<OroManifest>(&string)?;
        assert_eq!(
            parsed,
            OroManifestBuilder::default()
                .version("1.2.3".parse()?)
                .build()
                .unwrap()
        );

        let string = r#"
{
    "version": "invalid"
}
        "#;
        let parsed = serde_json::from_str::<OroManifest>(&string);
        assert!(parsed.is_err());
        Ok(())
    }

    #[test]
    fn bool_props() -> Result<()> {
        let string = r#"
{
    "private": true,
    "browser": true
}
        "#;
        let parsed = serde_json::from_str::<OroManifest>(&string)?;
        assert_eq!(
            parsed,
            OroManifestBuilder::default()
                .private(true)
                .browser(true)
                .build()
                .unwrap()
        );
        Ok(())
    }

    #[test]
    fn people_fields() -> Result<()> {
        let string = r#"
{
    "author": "Kat Marchan <kzm@zkat.tech>",
    "contributors": ["Eddy the Cat"]
}
        "#;
        let parsed = serde_json::from_str::<OroManifest>(&string)?;
        assert_eq!(
            parsed,
            OroManifestBuilder::default()
                .author(PersonField::Str("Kat Marchan <kzm@zkat.tech>".into()))
                .contributors(vec![PersonField::Str("Eddy the Cat".into())])
                .build()
                .unwrap()
        );

        let person =
            PersonField::Str("Kat Marchan <kzm@zkat.tech> (https://github.com/zkat)".into());
        assert_eq!(
            person.parse()?,
            Person {
                name: Some("Kat Marchan".into()),
                email: Some("kzm@zkat.tech".into()),
                url: Some("https://github.com/zkat".into())
            }
        );

        let person = PersonField::Obj {
            name: Some("Kat Marchan".into()),
            email: Some("kzm@zkat.tech".into()),
            url: Some("https://github.com/zkat".into()),
        };
        assert_eq!(
            person.parse()?,
            Person {
                name: Some("Kat Marchan".into()),
                email: Some("kzm@zkat.tech".into()),
                url: Some("https://github.com/zkat".into())
            }
        );
        Ok(())
    }

    #[test]
    fn from_file() -> Result<()> {
        let dir = tempdir()?;
        let file = dir.path().join("package.json");
        fs::write(
            &file,
            r#"
{
    "name": "my-package",
    "version": "1.2.3"
}
        "#,
        )?;
        assert_eq!(
            OroManifest::from_file(&file)?,
            OroManifestBuilder::default()
                .name("my-package")
                .version("1.2.3".parse()?)
                .build()
                .unwrap()
        );
        Ok(())
    }

    #[test]
    fn update_file() -> Result<()> {
        let dir = tempdir()?;
        let file = dir.path().join("package.json");
        fs::write(
            &file,
            r#"
{
    "version": "1.2.3",
    "private": true,
    "dependencies": { "foo": "^1.2.3" },
    "browser": true,
    "name": "my-package"
}
        "#,
        )?;
        let mut deps = HashMap::new();
        deps.insert(String::from("bar"), "> 3.2.1".parse()?);
        let mani = OroManifestBuilder::default()
            .name("new-name")
            .private(false)
            .version("3.2.1".parse()?)
            .dependencies(deps)
            .main("./index.ts")
            .build()
            .unwrap();
        mani.update_file(&file)?;
        // This checks that:
        // * Existing keys are updated
        // * Existing key order is preserved
        // * `None` keys in the toplevel object aren't written out/replaced
        // * New keys are appended to the end of the object
        assert_eq!(
            fs::read_to_string(&file)?,
            r#"{
  "version": "3.2.1",
  "private": false,
  "dependencies": {
    "bar": "> 3.2.1"
  },
  "browser": true,
  "name": "new-name",
  "main": "./index.ts"
}"#
        );
        Ok(())
    }
}
