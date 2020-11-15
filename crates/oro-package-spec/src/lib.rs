use std::fmt;
use std::path::PathBuf;
use std::str::FromStr;

use nom::combinator::all_consuming;
use nom::Err;
use oro_node_semver::{Version, VersionReq as Range};

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
            Alias { ref spec, .. } => spec,
            _ => self,
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
            Git(info) => write!(f, "{}", info),
            Npm {
                ref scope,
                ref name,
                ref requested,
            } => {
                if let Some(scope) = scope {
                    write!(f, "@{}/", scope)?;
                }
                write!(f, "{}", name)?;
                if let Some(req) = requested {
                    write!(f, "{}", req)?;
                }
                Ok(())
            }
            Alias { ref name, ref spec } => {
                write!(f, "{}@", name)?;
                if let Npm { .. } = **spec {
                    write!(f, "npm:")?;
                }
                write!(f, "{}", spec)
            }
        }
    }
}

impl fmt::Display for VersionSpec {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use VersionSpec::*;
        match self {
            Tag(tag) => write!(f, "{}", tag),
            Version(v) => write!(f, "{}", v),
            Range(range) => write!(f, "{}", range),
        }
    }
}

pub fn parse_package_spec<I>(input: I) -> Result<PackageSpec, PackageSpecError>
where
    I: AsRef<str>,
{
    let input = &input.as_ref()[..];
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
