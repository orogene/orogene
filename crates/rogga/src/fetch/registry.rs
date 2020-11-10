use std::path::Path;

use async_std::sync::{Arc, Mutex};
use async_trait::async_trait;
use dashmap::DashMap;
use futures::io::AsyncRead;
use http_types::Method;
use oro_client::{self, OroClient};
use oro_diagnostics::DiagnosticCode;
use oro_package_spec::PackageSpec;
use url::Url;

use crate::error::{Error, Internal, Result};
use crate::fetch::PackageFetcher;
use crate::package::Package;
use crate::packument::{Packument, VersionMetadata};
use crate::resolver::PackageResolution;

#[derive(Debug)]
pub struct RegistryFetcher {
    client: Arc<Mutex<OroClient>>,
    /// Corgis are a compressed kind of packument that omits some
    /// "unnecessary" fields (for some common operations during package
    /// management). This can significantly speed up installs, and is done
    /// through a special Accept header on request.
    use_corgi: bool,
    packuments: DashMap<Url, Packument>,
}

impl RegistryFetcher {
    pub fn new(client: Arc<Mutex<OroClient>>, use_corgi: bool) -> Self {
        Self {
            client,
            use_corgi,
            packuments: DashMap::new(),
        }
    }
}

impl RegistryFetcher {
    async fn packument_from_name(&self, scope: &Option<String>, name: &str) -> Result<Packument> {
        let client = self.client.lock().await.clone();
        let full_name = format!(
            "{}{}",
            scope
                .clone()
                .map(|s| format!("@{}/", s))
                .unwrap_or_else(|| String::from("")),
            name
        );
        let opts = client.opts(Method::Get, &full_name);
        let packument_data = client
            .send(opts.header(
                "Accept",
                if self.use_corgi {
                    "application/vnd.npm.install-v1+json; q=1.0, application/json; q=0.8, */*"
                } else {
                    "application/json"
                },
            ))
            .await
            .with_context(|| format!("Failed to get packument for {}.", full_name))?
            .body_string()
            .await
            .map_err(|e| Error::MiscError(e.to_string()))?;
        // let val: serde_json::Value = serde_json::from_str(&packument_data).to_internal()?;
        // println!("{:#?}", val);
        Ok(
            serde_json::from_str(&packument_data).map_err(|err| Error::SerdeError {
                code: DiagnosticCode::OR1006,
                name: full_name,
                data: packument_data,
                serde_error: err,
            })?,
        )
    }
}

#[async_trait]
impl PackageFetcher for RegistryFetcher {
    async fn name(&self, spec: &PackageSpec, _base_dir: &Path) -> Result<String> {
        match spec {
            // TODO: scopes
            PackageSpec::Npm { ref name, .. } | PackageSpec::Alias { ref name, .. } => {
                Ok(name.clone())
            }
            _ => unreachable!(),
        }
    }

    async fn metadata(&self, pkg: &Package) -> Result<VersionMetadata> {
        let wanted = match pkg.resolved {
            PackageResolution::Npm { ref version, .. } => version,
            _ => panic!("How did a non-Npm resolution get here?"),
        };
        let packument = self.packument(&pkg.from, &Path::new("")).await?;
        // TODO: unwrap
        Ok(packument.versions.get(&wanted).unwrap().clone())
    }

    async fn packument(&self, spec: &PackageSpec, _base_dir: &Path) -> Result<Packument> {
        // When fetching the packument itself, we need the _package_ name, not
        // its alias! Hence these shenanigans.
        let pkg = match spec {
            PackageSpec::Alias { ref package, .. } => package,
            pkg @ PackageSpec::Npm { .. } => pkg,
            _ => unreachable!(),
        };
        if let PackageSpec::Npm {
            ref scope,
            ref name,
            ..
        } = pkg
        {
            Ok(self.packument_from_name(scope, name).await?)
        } else {
            unreachable!()
        }
    }

    async fn tarball(&self, pkg: &Package) -> Result<Box<dyn AsyncRead + Unpin + Send + Sync>> {
        // NOTE: This .clone() is so we can free up the client lock, which
        // would otherwise, you know, make it so we can only make one request
        // at a time :(
        let client = self.client.lock().await.clone();
        let url = match pkg.resolved {
            PackageResolution::Npm { ref tarball, .. } => tarball,
            _ => panic!("How did a non-Npm resolution get here?"),
        };
        Ok(Box::new(
            client
                .send(client.opts(Method::Get, url))
                .await
                .with_context(|| format!("Failed to get tarball for {:#?}.", pkg.resolved))?,
        ))
    }
}
