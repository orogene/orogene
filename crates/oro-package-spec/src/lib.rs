use std::fmt;
use std::path::{Path, PathBuf};

use nom::combinator::all_consuming;
use nom::error::{convert_error, VerboseError};
use nom::Err;
use oro_error_code::OroErrCode as ErrCode;
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
        from: PathBuf,
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

// TODO:
// 1. Stop taking a dir arg and move that to PackageRequest?
// 2. Implement FromStr
// 3. Remove PackageSpec::resolve()
impl PackageSpec {
    pub fn from_string(
        s: impl AsRef<str>,
        dir: impl AsRef<Path>,
    ) -> Result<PackageSpec, PackageSpecError> {
        parse_package_spec(&s.as_ref(), dir.as_ref())
    }

    pub fn resolve<N, S, D>(name: N, spec: S, dir: D) -> Result<PackageSpec, PackageSpecError>
    where
        N: AsRef<str>,
        S: AsRef<str>,
        D: AsRef<Path>,
    {
        parse_package_spec(
            &(format!("{}@{}", name.as_ref(), spec.as_ref())),
            dir.as_ref(),
        )
    }

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

impl fmt::Display for PackageSpec {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use PackageSpec::*;
        match self {
            Dir { from, path } => write!(f, "{}", from.join(path).display()),
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

pub fn parse_package_spec<I, D>(input: I, dir: D) -> Result<PackageSpec, PackageSpecError>
where
    I: AsRef<str>,
    D: AsRef<Path>,
{
    let input = &input.as_ref()[..];
    match all_consuming(package::package_spec::<VerboseError<&str>>(dir.as_ref()))(input) {
        Ok((_, arg)) => Ok(arg),
        Err(err) => Err(PackageSpecError::ParseError(ErrCode::OR1000 {
            input: input.into(),
            msg: match err {
                Err::Error(e) => convert_error(input, e),
                Err::Failure(e) => convert_error(input, e),
                Err::Incomplete(_) => "More data was needed".into(),
            },
        })),
    }
}
