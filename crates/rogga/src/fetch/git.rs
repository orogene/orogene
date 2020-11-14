use std::collections::HashMap;
use std::path::Path;

use async_std::sync::{Arc, Mutex};
use async_trait::async_trait;
use futures::AsyncRead;
use oro_client::{self, OroClient};
use oro_manifest::OroManifest;
use oro_node_semver::Version;
use oro_package_spec::PackageSpec;
use serde::{Deserialize, Serialize};

use crate::error::{RoggaError, Result};
use crate::fetch::PackageFetcher;
use crate::package::Package;
use crate::packument::{Dist, Packument, VersionMetadata};
use crate::resolver::PackageResolution;

#[derive(Debug)]
pub struct GitFetcher {
    client: Arc<Mutex<OroClient>>,
}

impl GitFetcher {
    pub fn new(client: Arc<Mutex<OroClient>>) -> Self {
        Self { client }
    }
}

impl GitFetcher {
    async fn manifest(&self, path: &Path) -> Result<Manifest> {
        let pkg_path = path.join("package.json");
        let json = async_std::fs::read(&pkg_path)
            .await
            .map_err(|err| RoggaError::IoError(err, pkg_path))?;
        let pkgjson: OroManifest =
            serde_json::from_slice(&json[..]).map_err(RoggaError::SerdeError)?;
        Ok(Manifest(pkgjson))
    }

    async fn metadata_from_resolved(&self, res: &PackageResolution) -> Result<VersionMetadata> {
        let path = match res {
            PackageResolution::Git { path, .. } => path,
            _ => unreachable!(),
        };
        Ok(self.manifest(path).await?.into_metadata(&path)?)
    }

    async fn packument_from_spec(
        &self,
        spec: &PackageSpec,
        base_dir: &Path,
    ) -> Result<Arc<Packument>> {
        let path = match spec {
            PackageSpec::Git { path, .. } => base_dir.join(path),
            _ => unreachable!(),
        };
        Ok(Arc::new(self.manifest(&path).await?.into_packument(&path)?))
    }
}

#[async_trait]
impl PackageFetcher for GitFetcher {
    async fn name(&self, spec: &PackageSpec, base_dir: &Path) -> Result<String> {
        if let PackageSpec::Git(_) = spec {
            Ok(self
                .packument_from_spec(spec, base_dir)
                .await?
                .versions
                .iter()
                .next()
                .unwrap()
                .1
                .manifest
                .clone()
                .name
                .unwrap_or_else(|| "".into()))
        } else {
            unreachable!()
        }
    }

    async fn metadata(&self, pkg: &Package) -> Result<VersionMetadata> {
        self.metadata_from_resolved(pkg.resolved()).await
    }

    async fn packument(&self, spec: &PackageSpec, base_dir: &Path) -> Result<Arc<Packument>> {
        self.packument_from_spec(spec, base_dir).await
    }

    async fn tarball(
        &self,
        _pkg: &crate::Package,
    ) -> Result<Box<dyn AsyncRead + Unpin + Send + Sync>> {
        todo!()
    }
}

#[derive(Serialize, Deserialize)]
struct Manifest(OroManifest);

impl Manifest {
    pub fn into_metadata(self, path: impl AsRef<Path>) -> Result<VersionMetadata> {
        let Manifest(OroManifest {
            ref name,
            ref version,
            ..
        }) = self;
        let name = name.clone().or_else(|| {
            if let Some(name) = path.as_ref().file_name() {
                Some(name.to_string_lossy().into())
            } else {
                None
            }
        }).ok_or_else(|| RoggaError::MiscError("Failed to find a valid name. Make sure the package.json has a `name` field, or that it exists inside a named directory.".into()))?;
        let version = version
            .clone()
            .unwrap_or_else(|| Version::parse("0.0.0").expect("Oops, typo"));
        let mut new_manifest = self.0.clone();
        new_manifest.name = Some(name);
        new_manifest.version = Some(version);
        Ok(VersionMetadata {
            dist: Dist {
                shasum: None,
                tarball: None,

                integrity: None,
                file_count: None,
                unpacked_size: None,
                npm_signature: None,
                rest: HashMap::new(),
            },
            npm_user: None,
            has_shrinkwrap: None,
            maintainers: Vec::new(),
            deprecated: None,
            manifest: self.0.clone(),
        })
    }

    pub fn into_packument(self, path: impl AsRef<Path>) -> Result<Packument> {
        let metadata = self.into_metadata(path)?;
        let mut packument = Packument {
            versions: HashMap::new(),
            time: HashMap::new(),
            tags: HashMap::new(),
            rest: HashMap::new(),
        };
        packument
            .tags
            .insert("latest".into(), metadata.manifest.version.clone().unwrap());
        packument
            .versions
            .insert(metadata.manifest.version.clone().unwrap(), metadata);
        Ok(packument)
    }
}
