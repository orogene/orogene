use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use async_compression::futures::bufread::GzipEncoder;
use async_std::fs::File;
use async_trait::async_trait;
use futures::io::{AsyncRead, AsyncSeekExt, SeekFrom};
use node_semver::Version;
use once_cell::sync::Lazy;
use oro_common::{
    Bin, CorgiManifest, CorgiPackument, CorgiVersionMetadata, Manifest as OroManifest, Packument,
    VersionMetadata,
};
use oro_package_spec::PackageSpec;
use regex::Regex;
use serde::{Deserialize, Serialize};

use crate::error::{IoContext, NassunError, Result};
use crate::fetch::PackageFetcher;
use crate::package::Package;
use crate::resolver::PackageResolution;

pub(crate) const DEFAULT_WHITE_LIST: [&str; 22] = [
    "!.npmignore",
    "!.gitignore",
    "!**/.git",
    "!**/.svn",
    "!**/.hg",
    "!**/CVS",
    "!**/.git/**",
    "!**/.svn/**",
    "!**/.hg/**",
    "!**/CVS/**",
    "!/.lock-wscript",
    "!/.wafpickle-*",
    "!/build/config.gypi",
    "!npm-debug.log",
    "!**/.npmrc",
    "!.*.swp",
    "!.DS_Store",
    "!**/.DS_Store/**",
    "!._*",
    "!**/._*/**",
    "!*.orig",
    "!/archived-packages/**",
];

static PATH_REPLACE_REGEX: Lazy<Regex> = Lazy::new(|| Regex::new("^!+").unwrap());

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

    pub(crate) async fn tarball_from_path(
        &self,
        path: PathBuf,
    ) -> Result<Box<dyn AsyncRead + Unpin + Send + Sync>> {
        let mut cursor = async_std::io::Cursor::new(Vec::new());
        let package_path = std::path::Path::new("./package");
        let manifest = self.manifest(&path).await?;
        let cloned_path = path.clone();

        let files = async_std::task::spawn_blocking(
            move || -> std::result::Result<Vec<PathBuf>, ignore::Error> {
                let mut walk_builder = ignore::WalkBuilder::new(&path);
                let walk_builder = walk_builder.standard_filters(false);
                let npmignore = path.join(".npmignore");

                match manifest {
                    Manifest::FullFat(manifest) => {
                        let mut override_builder = ignore::overrides::OverrideBuilder::new(&path);

                        for file in DEFAULT_WHITE_LIST {
                            override_builder.add(file)?;
                        }

                        match manifest.files.clone() {
                            Some(files) => {
                                for mut file in files {
                                    if file.starts_with('/') {
                                        file = file[1..].to_owned();
                                    } else if file.starts_with("./") {
                                        file = file[2..].to_owned();
                                    } else if file.ends_with("/*") {
                                        file = file[..(file.len() - 2)].to_owned();
                                    }
                                    if path
                                        .join(PATH_REPLACE_REGEX.replace(&file, "").into_owned())
                                        .is_dir()
                                    {
                                        override_builder.add(&file)?;
                                        override_builder.add(&format!("{file}/**"))?;
                                    } else {
                                        override_builder.add(&file)?;
                                    }
                                }
                                Ok::<(), ignore::Error>(())
                            }
                            None if npmignore.exists() => {
                                walk_builder.add_custom_ignore_filename(".npmignore");
                                Ok(())
                            }
                            None => {
                                walk_builder.add_custom_ignore_filename(".gitignore");
                                Ok(())
                            }
                        }?;

                        if let Some(ref browser) = manifest.browser {
                            override_builder.add(browser)?;
                        }
                        if let Some(ref main) = manifest.main {
                            override_builder.add(main)?;
                        }
                        if let Some(ref bin) = manifest.bin {
                            match bin {
                                Bin::Array(paths) => {
                                    for path in paths {
                                        override_builder.add(&format!("{}", path.display()))?;
                                    }
                                }
                                Bin::Hash(paths) => {
                                    for path in paths.values() {
                                        override_builder.add(&format!("{}", path.display()))?;
                                    }
                                }
                                Bin::Str(path) => {
                                    override_builder.add(path)?;
                                }
                            }
                        }
                        override_builder.add("/package.json")?;
                        override_builder.add("!/.git")?;
                        override_builder.add("!/node_modules")?;
                        override_builder.add("!/package-lock.json")?;
                        override_builder.add("!/yarn.lock")?;
                        override_builder.add("!/pnpm-lock.yaml")?;
                        override_builder.add("!/package-lock.kdl")?;

                        walk_builder.overrides(override_builder.build()?);

                        Ok::<(), ignore::Error>(())
                    }
                    Manifest::Corgi(_) => Ok(()),
                }?;

                walk_builder
                    .build()
                    .map(|e| e.map(|e| e.into_path()))
                    .collect::<std::result::Result<Vec<PathBuf>, ignore::Error>>()
            },
        )
        .await?;

        {
            let mut builder = async_tar_wasm::Builder::new(&mut cursor);

            for file in &files {
                if file.is_file() {
                    let dst_file = pathdiff::diff_paths(file, &cloned_path).expect("TODO");
                    let dst_file = package_path.join(dst_file);
                    let mut content = match File::open(file).await {
                        Ok(content) => Ok(content),
                        Err(err) if err.kind() == std::io::ErrorKind::NotFound => continue,
                        Err(err) => Err(err)
                            .io_context(|| format!("Failed to open file at {}", file.display())),
                    }?;

                    builder
                        .append_file(&dst_file, &mut content)
                        .await
                        .io_context(|| "Failed to add file to tarball entries".to_owned())?;
                }
            }
            builder
                .finish()
                .await
                .io_context(|| "Failed to emit the termination sections.".to_owned())?;
        }

        let _ = cursor
            .seek(SeekFrom::Start(0))
            .await
            .io_context(|| "Failed to seek file content".to_owned());

        Ok(Box::new(GzipEncoder::new(cursor)))
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

    async fn tarball(&self, pkg: &Package) -> Result<Box<dyn AsyncRead + Unpin + Send + Sync>> {
        let path = match pkg.resolved() {
            PackageResolution::Dir { path, .. } => path.to_owned(),
            _ => panic!("There shouldn't be anything but Dirs here"),
        };
        self.tarball_from_path(path).await
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
            ..Default::default()
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
    use crate::client::Nassun;
    use async_compression::futures::bufread::GzipDecoder;
    use async_std::io::BufReader;
    use async_std::path::PathBuf as AsyncPathBuf;
    use async_std::stream::StreamExt;
    use miette::IntoDiagnostic;
    use oro_common::Manifest;
    use std::{fs::File, io::Write, path::PathBuf, str::FromStr};

    use tempfile::{tempdir, TempDir};

    use crate::error::IoContext;

    fn setup_dirs() -> Result<(impl PackageFetcher, PackageSpec, TempDir, PathBuf, PathBuf)> {
        let tmp = tempdir().io_context(|| "Failed to make temp dir".into())?;
        let package_path = tmp.path().join("oro-test");
        let cache_path = tmp.path().join("cache");
        std::fs::create_dir_all(&package_path)
            .io_context(|| format!("Failed to create path at {}.", package_path.display()))?;
        std::fs::create_dir_all(&cache_path)
            .io_context(|| format!("Failed to create path at {}.", cache_path.display()))?;
        let pkg_json = package_path.join("package.json");
        let mut package_file = File::create(&pkg_json)
            .io_context(|| format!("Failed to create file at {}.", pkg_json.display()))?;
        package_file
            .write_all(
                r#"{
            "name": "oro-test",
            "version": "1.4.2"
        }"#
                .as_bytes(),
            )
            .io_context(|| {
                format!(
                    "Failed to write mock file contents to {}.",
                    pkg_json.display()
                )
            })?;
        let dir_fetcher = DirFetcher;

        let package_spec = PackageSpec::Dir {
            path: PathBuf::new().join(&package_path),
        };

        Ok((dir_fetcher, package_spec, tmp, package_path, cache_path))
    }

    #[async_std::test]
    async fn read_name() -> Result<()> {
        let (fetcher, package_spec, _tmp, _package_path, cache_path) = setup_dirs()?;
        let name = fetcher.name(&package_spec, &cache_path).await?;
        assert_eq!(name, "oro-test");
        Ok(())
    }

    #[async_std::test]
    async fn read_packument() -> miette::Result<()> {
        let (fetcher, package_spec, _tmp, _package_path, cache_path) = setup_dirs()?;
        let packument = fetcher.packument(&package_spec, &cache_path).await?;
        assert_eq!(packument.versions.len(), 1);
        assert!(packument.versions.contains_key(&"1.4.2".parse()?));
        assert_eq!(
            packument
                .versions
                .get(&"1.4.2".parse()?)
                .unwrap()
                .dist
                .file_count,
            None
        );
        Ok(())
    }

    #[async_std::test]
    async fn read_tarball() -> miette::Result<()> {
        let (fetcher, package_spec, _tmp, package_path, _cache_path) = setup_dirs()?;
        let package = Nassun::new().resolve_spec(package_spec).await?;

        {
            File::create(package_path.join("package.json"))
                .io_context(|| "Failed to create file".to_owned())?
                .write_all(
                    serde_json::to_string(&Manifest {
                        name: Some("oro-test-package".to_owned()),
                        files: Some(vec!["/src/index.js".to_owned()]),
                        ..Default::default()
                    })
                    .into_diagnostic()?
                    .as_bytes(),
                )
                .io_context(|| "Failed to write contents to package.json".to_owned())?;

            std::fs::create_dir_all(package_path.join("src"))
                .io_context(|| "Failed to create directory".to_owned())?;
            File::create(package_path.join("src/index.js"))
                .io_context(|| "Failed to create file".to_owned())?;
            File::create(package_path.join("src/types.d.ts"))
                .io_context(|| "Failed to create file".to_owned())?;
            File::create(package_path.join("webpack.config.js"))
                .io_context(|| "Failed to create file".to_owned())?;
        }
        let gzip_encoded_tarball = fetcher.tarball(&package).await?;
        let tarball = GzipDecoder::new(BufReader::new(gzip_encoded_tarball));
        let tarball = async_tar_wasm::Archive::new(tarball);
        let mut try_tarball_entries = tarball.entries().into_diagnostic()?;
        let mut tarball_entries = Vec::new();
        while let Some(file) = try_tarball_entries.next().await {
            tarball_entries.push(
                file.into_diagnostic()?
                    .path()
                    .into_diagnostic()?
                    .into_owned(),
            );
        }

        assert_eq!(
            tarball_entries,
            vec![
                AsyncPathBuf::from_str("package/package.json").into_diagnostic()?,
                AsyncPathBuf::from_str("package/src/index.js").into_diagnostic()?,
            ]
        );

        Ok(())
    }
}
