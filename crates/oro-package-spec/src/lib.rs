use std::fmt;
use std::path::Path;

mod parser;
mod types;

pub use types::{PackageArgError, PackageSpec, VersionSpec};

impl PackageSpec {
    pub fn from_string(
        s: impl AsRef<str>,
        dir: impl AsRef<Path>,
    ) -> Result<PackageSpec, PackageArgError> {
        parser::parse_package_spec(&s.as_ref(), dir.as_ref())
    }

    pub fn resolve<N, S, D>(name: N, spec: S, dir: D) -> Result<PackageSpec, PackageArgError>
    where
        N: AsRef<str>,
        S: AsRef<str>,
        D: AsRef<Path>,
    {
        let mut arg = String::new();
        arg.push_str(name.as_ref());
        arg.push_str("@");
        arg.push_str(spec.as_ref());
        parser::parse_package_spec(&arg, dir.as_ref())
    }

    pub fn validate_name<S: AsRef<str>>(name: S) -> Result<String, PackageArgError> {
        let name = name.as_ref();
        Ok(name.into())
    }

    pub fn is_registry(&self) -> bool {
        match self {
            PackageSpec::Alias { package, .. } => package.is_registry(),
            PackageSpec::Dir { .. } => false,
            PackageSpec::Npm { .. } => true,
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
