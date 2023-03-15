use std::collections::HashMap;
use std::path::Path;

use async_std::sync::Arc;
use async_trait::async_trait;
use futures::io::AsyncRead;
use node_semver::Version;
use oro_common::{
    CorgiManifest, CorgiPackument, CorgiVersionMetadata, Manifest as OroManifest, Packument,
    VersionMetadata,
};
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
    pub(crate) async fn corgi_manifest(&self, path: &Path) -> Result<Manifest> {
        let pkg_path = path.join("package.json");
        let json = async_std::fs::read(&pkg_path)
            .await
            .map_err(|err| NassunError::DirReadError(err, pkg_path))?;
        let pkgjson: CorgiManifest =
            serde_json::from_slice(&json[..]).map_err(NassunError::SerdeError)?;
        Ok(Manifest::Corgi(Box::new(pkgjson)))
    }
    pub(crate) async fn manifest(&self, path: &Path) -> Result<Manifest> {
        let pkg_path = path.join("package.json");
        let json = async_std::fs::read(&pkg_path)
            .await
            .map_err(|err| NassunError::DirReadError(err, pkg_path))?;
        let pkgjson: OroManifest =
            serde_json::from_slice(&json[..]).map_err(NassunError::SerdeError)?;
        Ok(Manifest::FullFat(Box::new(pkgjson)))
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

    pub(crate) async fn corgi_metadata_from_path(
        &self,
        path: &Path,
    ) -> Result<CorgiVersionMetadata> {
        self.corgi_manifest(path).await?.into_corgi_metadata(path)
    }

    pub(crate) async fn corgi_packument_from_path(
        &self,
        path: &Path,
    ) -> Result<Arc<CorgiPackument>> {
        Ok(Arc::new(
            self.corgi_manifest(path)
                .await?
                .into_corgi_packument(path)?,
        ))
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

    async fn corgi_metadata(&self, pkg: &Package) -> Result<CorgiVersionMetadata> {
        let path = match pkg.resolved() {
            PackageResolution::Dir { path, .. } => path,
            _ => panic!("There shouldn't be anything but Dirs here"),
        };
        self.corgi_metadata_from_path(path).await
    }

    async fn packument(&self, spec: &PackageSpec, base_dir: &Path) -> Result<Arc<Packument>> {
        let path = match spec {
            PackageSpec::Dir { path } => base_dir.join(path),
            _ => panic!("There shouldn't be anything but Dirs here"),
        };
        self.packument_from_path(&path).await
    }

    async fn corgi_packument(
        &self,
        spec: &PackageSpec,
        base_dir: &Path,
    ) -> Result<Arc<CorgiPackument>> {
        let path = match spec {
            PackageSpec::Dir { path } => base_dir.join(path),
            _ => panic!("There shouldn't be anything but Dirs here"),
        };
        self.corgi_packument_from_path(&path).await
    }

    async fn tarball(&self, _pkg: &Package) -> Result<Box<dyn AsyncRead + Unpin + Send + Sync>> {
        // TODO: need to implement pack before this can be implemented :(
        unimplemented!()
    }
}

#[derive(Serialize, Deserialize)]
pub(crate) enum Manifest {
    FullFat(Box<OroManifest>),
    Corgi(Box<CorgiManifest>),
}

impl Manifest {
    pub(crate) fn into_corgi_metadata(
        self,
        path: impl AsRef<Path>,
    ) -> Result<CorgiVersionMetadata> {
        let Manifest::Corgi(manifest) = &self else {
            unreachable!("This should have been called in such a way as to guarantee corgi.")
        };
        let name = manifest.name.clone().or_else(|| {
            path.as_ref().file_name().map(|name| name.to_string_lossy().into())
        }).ok_or_else(|| NassunError::MiscError("Failed to find a valid name. Make sure the package.json has a `name` field, or that it exists inside a named directory.".into()))?;
        let version = manifest
            .version
            .clone()
            .unwrap_or_else(|| Version::parse("0.0.0").expect("Oops, typo"));
        let mut new_manifest = manifest.clone();
        new_manifest.name = Some(name);
        new_manifest.version = Some(version);
        Ok(CorgiVersionMetadata {
            manifest: *new_manifest,
            ..Default::default()
        })
    }

    pub(crate) fn into_metadata(self, path: impl AsRef<Path>) -> Result<VersionMetadata> {
        let Manifest::FullFat(manifest) = &self else {
            unreachable!("This should have been called in such a way as to guarantee fullfat.")
        };
        let name = manifest.name.clone().or_else(|| {
            path.as_ref().file_name().map(|name| name.to_string_lossy().into())
        }).ok_or_else(|| NassunError::MiscError("Failed to find a valid name. Make sure the package.json has a `name` field, or that it exists inside a named directory.".into()))?;
        let version = manifest
            .version
            .clone()
            .unwrap_or_else(|| Version::parse("0.0.0").expect("Oops, typo"));
        let mut new_manifest = manifest.clone();
        new_manifest.name = Some(name);
        new_manifest.version = Some(version);
        Ok(VersionMetadata {
            manifest: *new_manifest,
            ..Default::default()
        })
    }

    pub(crate) fn into_corgi_packument(self, path: impl AsRef<Path>) -> Result<CorgiPackument> {
        let metadata = self.into_corgi_metadata(path)?;
        let mut packument = CorgiPackument {
            versions: HashMap::new(),
            tags: HashMap::new(),
        };
        let version = metadata
            .manifest
            .version
            .clone()
            .unwrap_or_else(|| Version::parse("0.0.0").expect("Oops, typo"));
        packument.tags.insert("latest".into(), version.clone());
        packument.versions.insert(version, metadata);
        Ok(packument)
    }

    pub(crate) fn into_packument(self, path: impl AsRef<Path>) -> Result<Packument> {
        let metadata = self.into_metadata(path)?;
        let mut packument = Packument {
            versions: HashMap::new(),
            time: HashMap::new(),
            tags: HashMap::new(),
            rest: HashMap::new(),
        };
        let version = metadata
            .manifest
            .version
            .clone()
            .unwrap_or_else(|| Version::parse("0.0.0").expect("Oops, typo"));
        packument.tags.insert("latest".into(), version.clone());
        packument.versions.insert(version, metadata);
        Ok(packument)
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use std::{fs::File, io::Write, path::PathBuf};

    use tempfile::{tempdir, TempDir};

    fn setup_dirs() -> Result<(impl PackageFetcher, PackageSpec, TempDir, PathBuf, PathBuf)>
    {
        let tmp = tempdir()?;
        let package_path = tmp.path().join("oro-test");
        let cache_path = tmp.path().join("cache");
        std::fs::create_dir_all(&package_path)?;
        std::fs::create_dir_all(&cache_path)?;
        let mut package_file = File::create(package_path.join("package.json"))?;
        package_file.write_all(r#"{
            "name": "oro-test",
            "version": "1.4.2"
        }"#.as_bytes())?;
        let dir_fetcher = DirFetcher;

        let package_spec = PackageSpec::Dir { path: PathBuf::new().join(&package_path) };

        Ok((dir_fetcher, package_spec, tmp, package_path, cache_path))
    }

    #[async_std::test]
    async fn read_name() -> Result<()>
    {
        let (fetcher, package_spec, _tmp, _package_path, cache_path) = setup_dirs()?;
        let name = fetcher.name(&package_spec, &cache_path).await?;
        assert_eq!(name, "oro-test");
        Ok(())
    }

    #[async_std::test]
    async fn read_packument() -> miette::Result<()>
    {
        let (fetcher, package_spec, _tmp, _package_path, cache_path) = setup_dirs()?;
        let packument = fetcher.packument(&package_spec, &cache_path).await?;
        assert_eq!(packument.versions.len(), 1);
        assert!(packument.versions.contains_key(&"1.4.2".parse()?));
        assert_eq!(packument.versions.get(&"1.4.2".parse()?).unwrap().dist.file_count, None);
        Ok(())
    }
}