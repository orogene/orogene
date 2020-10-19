use std::future::Future;
use std::path::PathBuf;
use std::pin::Pin;

use anyhow::Result;
use async_trait::async_trait;
use clap::Clap;
use oro_command::OroCommand;
use oro_config::OroConfigLayer;
use oro_tree::{self, Package, PkgLock};
use rogga::{
    PackageRequest, PackageResolution, PackageResolver, PackageSpec, ResolverError, Rogga,
};
use url::Url;

#[derive(Debug, Clap, OroConfigLayer)]
pub struct RestoreCmd {
    #[clap(
        about = "Registry to ping.",
        default_value = "https://registry.npmjs.org",
        long
    )]
    registry: Url,
    #[clap(about = "cache to fill up", long, short = 'C')]
    cache: PathBuf,
    #[clap(from_global)]
    loglevel: log::LevelFilter,
    #[clap(from_global)]
    json: bool,
    #[clap(from_global)]
    quiet: bool,
}

pub struct PkgLockResolver<'a> {
    dep: &'a Package,
}

#[async_trait]
impl<'a> PackageResolver for PkgLockResolver<'a> {
    async fn resolve(
        &self,
        wanted: &PackageRequest,
    ) -> std::result::Result<PackageResolution, ResolverError> {
        Ok(match wanted.spec() {
            PackageSpec::Npm { .. } => {
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
            PackageSpec::Dir { .. } => PackageResolution::Dir {
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
        dep: &'a Package,
        dir: PathBuf,
    ) -> Pin<Box<dyn Future<Output = Result<()>> + Send + 'a>> {
        Box::pin(async move {
            let mut futs = Vec::new();
            let req = rogga.dep_request(name, &dep.version, &dir)?;
            let resolver = PkgLockResolver { dep };
            for (name, dep) in dep.dependencies.iter() {
                if !dep.bundled {
                    futs.push(self.extract(rogga, name, dep, dir.join("node_modules").join(name)));
                }
            }
            futs.push(Box::pin(async move {
                let resolved = req.resolve_with(&resolver).await?;
                let tarball = resolved.tarball().await?;
                rogga::cache::from_tarball(&self.cache, tarball).await?;
                // rogga::cache::tarball_itself(&self.cache, tarball).await?;
                // rogga::cache::tarball_to_mem(&self.cache, tarball).await?;
                // rogga::cache::to_node_modules(
                //     &self.cache.join(format!(
                //         "{}-{}",
                //         resolved.name,
                //         match resolved.resolved {
                //             PackageResolution::Npm { version, .. } => version.to_string(),
                //             PackageResolution::Dir { .. } => "path".into(),
                //         }
                //     )),
                //     tarball,
                // )
                // .await?;
                Ok(())
            }));
            futures::future::try_join_all(futs).await?;
            Ok(())
        })
    }
}

#[async_trait]
impl OroCommand for RestoreCmd {
    async fn execute(self) -> Result<()> {
        let pkglock: PkgLock = oro_tree::read("./package-lock.json")?;
        let rogga = Rogga::new(&self.registry);
        let mut futs = Vec::new();
        for (name, dep) in pkglock.dependencies.iter() {
            futs.push(self.extract(&rogga, name, dep, std::env::current_dir()?));
        }
        futures::future::try_join_all(futs).await?;
        Ok(())
    }
}
