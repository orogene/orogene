use std::collections::HashMap;
use std::path::Path;

use async_std::sync::Arc;
use async_trait::async_trait;
use futures::io::AsyncRead;
use node_semver::Version;
use oro_common::{Dist, Manifest as OroManifest, Packument, VersionMetadata};
use oro_package_spec::PackageSpec;
use serde::{Deserialize, Serialize};

use crate::error::{NassunError, Result};
use crate::fetch::PackageFetcher;
use crate::package::Package;
use crate::resolver::PackageResolution;

#[derive(Debug)]
pub(crate) struct DirFetcher;

impl DirFetcher {
    pub(crate) fn new() -> Self {
        Self
    }
}

impl DirFetcher {
    pub(crate) async fn manifest(&self, path: &Path) -> Result<Manifest> {
        let pkg_path = path.join("package.json");
        let json = async_std::fs::read(&pkg_path)
            .await
            .map_err(|err| NassunError::DirReadError(err, pkg_path))?;
        let pkgjson: OroManifest =
            serde_json::from_slice(&json[..]).map_err(NassunError::SerdeError)?;
        Ok(Manifest(pkgjson))
    }

    pub(crate) async fn name_from_path(&self, path: &Path) -> Result<String> {
        Ok(self
            .packument_from_path(path)
            .await?
            .versions
            .iter()
            .next()
            .unwrap()
            .1
            .manifest
            .clone()
            .name
            .unwrap_or_else(|| {
                let canon = path.canonicalize();
                let path = canon.as_ref().map(|p| p.file_name());
                if let Ok(Some(name)) = path {
                    name.to_string_lossy().into()
                } else {
                    "".into()
                }
            }))
    }

    pub(crate) async fn metadata_from_path(&self, path: &Path) -> Result<VersionMetadata> {
        self.manifest(path).await?.into_metadata(path)
    }

    pub(crate) async fn packument_from_path(&self, path: &Path) -> Result<Arc<Packument>> {
        Ok(Arc::new(self.manifest(path).await?.into_packument(path)?))
    }
}

#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
impl PackageFetcher for DirFetcher {
    async fn name(&self, spec: &PackageSpec, base_dir: &Path) -> Result<String> {
        let path = match spec {
            PackageSpec::Alias { name, .. } => return Ok(name.clone()),
            PackageSpec::Dir { path } => path,
            _ => panic!("There shouldn't be anything but Dirs here"),
        };
        self.name_from_path(&base_dir.join(path)).await
    }

    async fn metadata(&self, pkg: &Package) -> Result<VersionMetadata> {
        let path = match pkg.resolved() {
            PackageResolution::Dir { path, .. } => path,
            _ => panic!("There shouldn't be anything but Dirs here"),
        };
        self.metadata_from_path(path).await
    }

    async fn packument(&self, spec: &PackageSpec, base_dir: &Path) -> Result<Arc<Packument>> {
        let path = match spec {
            PackageSpec::Dir { path } => base_dir.join(path),
            _ => panic!("There shouldn't be anything but Dirs here"),
        };
        self.packument_from_path(&path).await
    }

    async fn tarball(&self, _pkg: &Package) -> Result<Box<dyn AsyncRead + Unpin + Send + Sync>> {
        // TODO: need to implement pack before this can be implemented :(
        unimplemented!()
    }
}

#[derive(Serialize, Deserialize)]
pub(crate) struct Manifest(OroManifest);

impl Manifest {
    pub(crate) fn into_metadata(self, path: impl AsRef<Path>) -> Result<VersionMetadata> {
        let Manifest(OroManifest {
            ref name,
            ref version,
            ..
        }) = self;
        let name = name.clone().or_else(|| {
            path.as_ref().file_name().map(|name| name.to_string_lossy().into())
        }).ok_or_else(|| NassunError::MiscError("Failed to find a valid name. Make sure the package.json has a `name` field, or that it exists inside a named directory.".into()))?;
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

    pub(crate) fn into_packument(self, path: impl AsRef<Path>) -> Result<Packument> {
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
