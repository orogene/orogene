use std::fmt;
use std::path::PathBuf;
use std::str::FromStr;

use nom::combinator::all_consuming;
use nom::error::{convert_error, VerboseError};
use nom::Err;
use oro_diagnostics::DiagnosticCode;
use oro_node_semver::{Version, VersionReq as Range};

pub use crate::error::PackageSpecError;
use crate::parsers::package;

mod error;
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
        package: Box<PackageSpec>,
    },
    Npm {
        scope: Option<String>,
        name: String,
        requested: Option<VersionSpec>,
    },
}

impl PackageSpec {
    pub fn is_registry(&self) -> bool {
        use PackageSpec::*;
        match self {
            Alias { package, .. } => package.is_registry(),
            Dir { .. } => false,
            Npm { .. } => true,
        }
    }

    pub fn target(&self) -> &PackageSpec {
        use PackageSpec::*;
        match self {
            Alias { ref package, .. } => package,
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
            Alias {
                ref name,
                ref package,
            } => {
                write!(f, "{}@", name)?;
                if let Npm { .. } = **package {
                    write!(f, "npm:")?;
                }
                write!(f, "{}", package)
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
    match all_consuming(package::package_spec::<VerboseError<&str>>)(input) {
        Ok((_, arg)) => Ok(arg),
        Err(err) => Err(PackageSpecError::ParseError {
            code: DiagnosticCode::OR1001,
            input: input.into(),
            msg: match err {
                Err::Error(e) => convert_error(input, e),
                Err::Failure(e) => convert_error(input, e),
                Err::Incomplete(_) => "More data was needed".into(),
            },
        }),
    }
}
