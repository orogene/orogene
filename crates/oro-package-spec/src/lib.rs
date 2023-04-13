//! NPM package specifier parser. This is the stuff that takes something like
//! `foo@^1.2.3` and turns it into something meaningful.

use std::fmt;
use std::path::PathBuf;
use std::str::FromStr;

use node_semver::{Range, Version};
use nom::combinator::all_consuming;
use nom::Err;

pub use crate::error::{PackageSpecError, SpecErrorKind};
pub use crate::gitinfo::{GitHost, GitInfo};
use crate::parsers::package;

mod error;
mod gitinfo;
mod parsers;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum VersionSpec {
    Tag(String),
    Version(Version),
    Range(Range),
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum PackageSpec {
    Dir {
        path: PathBuf,
    },
    Alias {
        name: String,
        spec: Box<PackageSpec>,
    },
    Npm {
        scope: Option<String>,
        name: String,
        requested: Option<VersionSpec>,
    },
    Git(GitInfo),
}

impl PackageSpec {
    pub fn is_alias(&self) -> bool {
        matches!(self, PackageSpec::Alias { .. })
    }

    pub fn is_npm(&self) -> bool {
        use PackageSpec::*;
        match self {
            Alias { spec, .. } => spec.is_npm(),
            Dir { .. } | Git(..) => false,
            Npm { .. } => true,
        }
    }

    pub fn target(&self) -> &PackageSpec {
        use PackageSpec::*;
        match self {
            Alias { ref spec, .. } => {
                if spec.is_alias() {
                    spec.target()
                } else {
                    spec
                }
            }
            _ => self,
        }
    }

    pub fn target_mut(&mut self) -> &mut PackageSpec {
        use PackageSpec::*;
        match self {
            Alias { spec, .. } => {
                if spec.is_alias() {
                    spec.target_mut()
                } else {
                    spec
                }
            }
            _ => self,
        }
    }

    pub fn requested(&self) -> String {
        use PackageSpec::*;
        match self {
            Dir { path } => format!("{}", path.display()),
            Git(info) => format!("{info}"),
            Npm { ref requested, .. } => requested
                .as_ref()
                .map(|r| r.to_string())
                .unwrap_or_else(|| "*".to_string()),
            Alias { ref spec, .. } => {
                format!(
                    "{}{}",
                    if let Npm { .. } = **spec { "npm:" } else { "" },
                    spec
                )
            }
        }
    }
}

impl FromStr for PackageSpec {
    type Err = PackageSpecError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        parse_package_spec(s)
    }
}

impl fmt::Display for PackageSpec {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use PackageSpec::*;
        match self {
            Dir { path } => write!(f, "{}", path.display()),
            Git(info) => write!(f, "{info}"),
            Npm {
                ref name,
                ref requested,
                ..
            } => {
                write!(f, "{name}")?;
                if let Some(req) = requested {
                    write!(f, "@{req}")?;
                }
                Ok(())
            }
            Alias { ref name, ref spec } => {
                write!(f, "{name}@")?;
                if let Npm { .. } = **spec {
                    write!(f, "npm:")?;
                }
                write!(f, "{spec}")
            }
        }
    }
}

impl fmt::Display for VersionSpec {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use VersionSpec::*;
        match self {
            Tag(tag) => write!(f, "{tag}"),
            Version(v) => write!(f, "{v}"),
            Range(range) => write!(f, "{range}"),
        }
    }
}

fn parse_package_spec<I>(input: I) -> Result<PackageSpec, PackageSpecError>
where
    I: AsRef<str>,
{
    let input = input.as_ref();
    match all_consuming(package::package_spec)(input) {
        Ok((_, arg)) => Ok(arg),
        Err(err) => Err(match err {
            Err::Error(e) | Err::Failure(e) => PackageSpecError {
                input: input.into(),
                offset: e.input.as_ptr() as usize - input.as_ptr() as usize,
                kind: if let Some(kind) = e.kind {
                    kind
                } else if let Some(ctx) = e.context {
                    SpecErrorKind::Context(ctx)
                } else {
                    SpecErrorKind::Other
                },
            },
            Err::Incomplete(_) => PackageSpecError {
                input: input.into(),
                offset: input.len() - 1,
                kind: SpecErrorKind::IncompleteInput,
            },
        }),
    }
}
