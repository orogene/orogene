use std::collections::HashMap;
use std::fs::File;
use std::future::Future;
use std::path::PathBuf;
use std::pin::Pin;
use std::time::Instant;

use anyhow::{Context, Result};
use async_std::io::BufReader;
use async_trait::async_trait;
use clap::Clap;
use oro_command::OroCommand;
use oro_error_code::OroErrCode as Code;
use rogga::{PackageArg, PackageRequest, PackageResolution, Resolver, ResolverError, Rogga};
use semver::Version;
use serde::{Deserialize, Serialize};
use ssri::Integrity;
use url::Url;

#[derive(Debug, Clap, OroCommand)]
pub struct RestoreCmd {
    #[clap(
        about = "Registry to ping.",
        default_value = "https://registry.npmjs.org",
        long
    )]
    registry: Url,
    #[clap(about = "cache to fill up")]
    cache: PathBuf,
    #[clap(from_global)]
    loglevel: log::LevelFilter,
    #[clap(from_global)]
    json: bool,
    #[clap(from_global)]
    quiet: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PkgLock {
    name: Option<String>,
    version: Option<Version>,
    #[serde(rename = "lockfileVersion")]
    lockfile_version: f32,
    #[serde(default)]
    requires: bool,
    #[serde(default)]
    dependencies: HashMap<String, PkgLockDep>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PkgLockDep {
    version: String,
    integrity: Option<String>,
    resolved: Option<Url>,
    #[serde(default)]
    bundled: bool,
    #[serde(default)]
    dev: bool,
    #[serde(default)]
    optional: bool,
    #[serde(default)]
    requires: HashMap<String, String>,
    #[serde(default)]
    dependencies: HashMap<String, PkgLockDep>,
}

pub struct PkgLockResolver<'a> {
    dep: &'a PkgLockDep,
}
#[async_trait]
impl<'a> Resolver for PkgLockResolver<'a> {
    async fn resolve(
        &self,
        wanted: &PackageRequest,
    ) -> std::result::Result<PackageResolution, ResolverError> {
        Ok(match wanted.spec() {
            PackageArg::Npm { .. } => {
                PackageResolution::Npm {
                    version: self
                        .dep
                        .version
                        .parse()
                        .map_err(|e| ResolverError::OtherError(Box::new(e)))?,
                    // TODO - need to do a metadata request if the tarball is empty.
                    tarball: self.dep.resolved.clone().unwrap(),
                }
            }
            PackageArg::Dir { .. } => PackageResolution::Dir {
                path: self
                    .dep
                    .version
                    .parse()
                    .map_err(|e| ResolverError::OtherError(Box::new(e)))?,
            },
            _ => panic!("Should not be getting any other type right now"),
        })
    }
}

impl RestoreCmd {
    fn extract<'a>(
        &'a self,
        rogga: &'a Rogga,
        name: &'a str,
        dep: &'a PkgLockDep,
    ) -> Pin<Box<dyn Future<Output = Result<()>> + Send + 'a>> {
        Box::pin(async move {
            let mut futs = Vec::new();
            let req = rogga.dep_request(name, &dep.version)?;
            let resolver = PkgLockResolver { dep };
            for (name, dep) in dep.dependencies.iter() {
                if !dep.bundled {
                    futs.push(self.extract(rogga, name, dep));
                }
            }
            futures::future::try_join_all(futs).await?;
            let resolved = req.resolve_with(resolver).await?;
            let tarball = resolved.tarball().await?;
            rogga::cache::from_tarball(&self.cache, tarball).await?;
            // println!("{:#?}", resolved.resolved);
            Ok(())
        })
    }
}

#[async_trait]
impl OroCommand for RestoreCmd {
    async fn execute(self) -> Result<()> {
        let pkglock: PkgLock =
            serde_json::from_reader(std::io::BufReader::new(File::open("./package-lock.json")?))?;
        let rogga = Rogga::new(&self.registry);
        let mut futs = Vec::new();
        for (name, dep) in pkglock.dependencies.iter() {
            futs.push(self.extract(&rogga, name, dep));
        }
        futures::future::try_join_all(futs).await?;
        Ok(())
    }
}
