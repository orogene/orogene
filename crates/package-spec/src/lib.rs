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
