use std::str::FromStr;

use futures::io::AsyncRead;
use package_arg::{PackageArg, PackageArgError};

use crate::fetch::{
    DirFetcher, Manifest, PackageFetcher, PackageFetcherError, Packument, RegistryFetcher,
};
pub struct Package {
    spec: PackageArg,
    fetcher: Box<dyn PackageFetcher>,
}

impl Package {
    /// Creates a Package from a plain string spec, i.e. `foo@1.2.3`.
    pub fn from_arg<T: AsRef<str>>(arg: T) -> Result<Self, PackageArgError> {
        let spec = PackageArg::from_string(arg.as_ref())?;
        let fetcher = pick_fetcher(&spec);
        Ok(Self { spec, fetcher })
    }

    /// Creates a Package from a two-part dependency declaration, such as
    /// `dependencies` entries in a `package.json`.
    pub fn from_dep<T: AsRef<str>, U: AsRef<str>>(
        name: T,
        spec: U,
    ) -> Result<Self, PackageArgError> {
        let spec = PackageArg::resolve(name.as_ref(), spec.as_ref())?;
        let fetcher = pick_fetcher(&spec);
        Ok(Self { spec, fetcher })
    }

    pub async fn name(&self) -> Result<String, PackageFetcherError> {
        use PackageArg::*;
        match self.spec {
            Dir { .. } => Ok(self.manifest().await?.name),
            Alias { ref name, .. } | Npm { ref name, .. } => Ok(name.clone()),
        }
    }

    pub async fn manifest(&self) -> Result<Manifest, PackageFetcherError> {
        self.fetcher.manifest().await
    }

    pub async fn packument(&self) -> Result<Packument, PackageFetcherError> {
        self.fetcher.packument().await
    }

    pub async fn tarball(&self) -> Result<Box<dyn AsyncRead + Send + Sync>, PackageFetcherError> {
        self.fetcher.tarball().await
    }
}

fn pick_fetcher(arg: &PackageArg) -> Box<dyn PackageFetcher> {
    use PackageArg::*;
    match *arg {
        Dir { .. } => Box::new(DirFetcher::new()),
        Alias { ref package, .. } => pick_fetcher(package),
        Npm { .. } => Box::new(RegistryFetcher::new()),
    }
}

impl FromStr for Package {
    type Err = PackageArgError;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        Package::from_arg(s)
    }
}
