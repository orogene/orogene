use std::str::FromStr;

use anyhow::{self, Result};

mod parser;
mod types;

pub use types::{PackageArg, PackageArgError, VersionReq};

impl PackageArg {
    pub fn from_string<S: AsRef<str>>(s: S) -> Result<PackageArg> {
        parser::parse_package_arg(&s.as_ref())
    }

    pub fn resolve(name: String, spec: String) -> Result<PackageArg> {
        let mut arg = String::new();
        arg.push_str(&name);
        arg.push_str("@");
        arg.push_str(&spec);
        parser::parse_package_arg(&arg)
    }

    pub fn validate_name<S: AsRef<str>>(name: S) -> Result<String> {
        let name = name.as_ref();
        Ok(name.into())
    }

    pub fn is_registry(&self) -> bool {
        match self {
            PackageArg::Alias { package, .. } => package.is_registry(),
            PackageArg::Dir { .. } => false,
            PackageArg::Npm { .. } => true,
        }
    }
}

impl FromStr for PackageArg {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        PackageArg::from_string(s)
    }
}
