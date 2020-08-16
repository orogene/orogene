use std::collections::HashMap;
use std::str::FromStr;

use derive_builder::Builder;
use error::Result;
use oro_semver::{Version, VersionReq};
use serde::{Deserialize, Serialize};

pub use error::Error;

mod error;

#[derive(Builder, Clone, Debug, PartialEq, Serialize, Deserialize)]
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
    pub browser: bool,

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
    pub directories: Directories,

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
    pub config: Option<serde_json::Value>,

    #[serde(default)]
    #[builder(default)]
    pub os: Vec<String>,

    #[serde(default)]
    #[builder(default)]
    pub cpu: Vec<String>,

    #[serde(default)]
    #[builder(default)]
    pub private: bool,

    #[serde(default)]
    #[builder(default)]
    pub publish_config: HashMap<String, String>,

    // Deps
    #[serde(default)]
    #[builder(default)]
    pub dependencies: HashMap<String, VersionReq>,

    #[serde(default)]
    #[builder(default)]
    pub dev_dependencies: HashMap<String, VersionReq>,

    #[serde(default)]
    #[builder(default)]
    pub optional_dependencies: HashMap<String, VersionReq>,

    #[serde(default)]
    #[builder(default)]
    pub peer_dependencies: HashMap<String, VersionReq>,

    #[serde(default, alias = "bundleDependencies", alias = "bundledDependencies")]
    #[builder(default)]
    pub bundled_dependencies: Vec<String>,

    #[serde(default)]
    #[builder(default)]
    pub workspaces: Vec<String>,

    #[serde(flatten, default)]
    #[builder(default)]
    pub _rest: HashMap<String, serde_json::Value>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Bugs {
    Str(String),
    Obj {
        url: Option<String>,
        email: Option<String>,
    },
}

/// Represents a human!
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
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

#[derive(Clone, Debug, PartialEq)]
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

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Directories {
    pub bin: Option<String>,
    pub man: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Bin {
    Str(String),
    Hash(HashMap<String, String>),
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Man {
    Str(String),
    Vec(Vec<String>),
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Exports {
    Str(String),
    Vec(Vec<String>),
    Obj(HashMap<String, Exports>),
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Imports {
    Str(String),
    Vec(Vec<String>),
    Obj(HashMap<String, Imports>),
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
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
    use nom::sequence::{delimited, tuple};
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
                    take_till(|c| c == '('),
                    opt(delimited(char('('), take_till1(|c| c == ')'), char(')'))),
                )),
                |(name, email, _, url): (Option<&str>, Option<&str>, _, Option<&str>)| Person {
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

    use anyhow::Result;

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
        deps.insert(String::from("foo"), VersionReq::parse("^3.2.1")?);
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
}
