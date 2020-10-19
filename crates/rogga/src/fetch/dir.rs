use std::collections::HashMap;
use std::path::Path;

use async_trait::async_trait;
use futures::io::AsyncRead;
use oro_manifest::OroManifestBuilder;
use package_spec::PackageSpec;
use serde::{Deserialize, Serialize};

use crate::error::{Error, Internal, Result};
use crate::fetch::PackageFetcher;
use crate::package::{Package, PackageRequest};
use crate::packument::{Dist, Packument, VersionMetadata};

use oro_node_semver::Version;

#[derive(Debug)]
pub struct DirFetcher {
    name: Option<String>,
}

impl DirFetcher {
    pub fn new() -> Self {
        Self { name: None }
    }
}

impl DirFetcher {
    async fn pkgjson(&mut self, spec: &PackageSpec) -> Result<PkgJson> {
        let path = match spec {
            PackageSpec::Dir { path, from } => from.join(path),
            _ => panic!("There shouldn't be anything but Dirs here"),
        };
        // TODO: Orogene.toml?
        let json = async_std::fs::read_to_string(&path.join("package.json"))
            .await
            .to_internal()
            .with_context(|| "Failed to read package.json".into())?;
        let pkgjson: PkgJson = serde_json::from_str(&json)
            .to_internal()
            .with_context(|| "Failed to parse package.json".into())?;
        Ok(pkgjson)
    }

    async fn metadata_from_spec(&mut self, spec: &PackageSpec) -> Result<VersionMetadata> {
        let path = match spec {
            PackageSpec::Dir { path, from } => from.join(path),
            _ => panic!("There shouldn't be anything but Dirs here"),
        };
        Ok(self.pkgjson(spec).await?.into_metadata(&path)?)
    }

    async fn packument_from_spec(&mut self, spec: &PackageSpec) -> Result<Packument> {
        let path = match spec {
            PackageSpec::Dir { path, from } => from.join(path),
            _ => panic!("There shouldn't be anything but Dirs here"),
        };
        Ok(self.pkgjson(spec).await?.into_packument(&path)?)
    }
}

#[async_trait]
impl PackageFetcher for DirFetcher {
    async fn name(&mut self, spec: &PackageSpec) -> Result<String> {
        if let Some(ref name) = self.name {
            Ok(name.clone())
        } else if let PackageSpec::Dir { ref path, ref from } = spec {
            self.name = Some(
                self.packument_from_spec(spec)
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
                        let canon = from.join(path).canonicalize();
                        let path = canon.as_ref().map(|p| p.file_name());
                        if let Ok(Some(name)) = path {
                            name.to_string_lossy().into()
                        } else {
                            "".into()
                        }
                    }),
            );
            self.name
                .as_ref()
                .cloned()
                .ok_or_else(|| Error::MiscError("This is impossible".into()))
        } else {
            unreachable!()
        }
    }

    async fn manifest(&mut self, pkg: &Package) -> Result<VersionMetadata> {
        self.metadata_from_spec(&pkg.from).await
    }

    async fn packument(&mut self, pkg: &PackageRequest) -> Result<Packument> {
        self.packument_from_spec(pkg.spec()).await
    }

    async fn tarball(
        &mut self,
        _pkg: &Package,
    ) -> Result<Box<dyn AsyncRead + Unpin + Send + Sync>> {
        // TODO: need to implement pack before this can be implemented :(
        unimplemented!()
    }
}

#[derive(Serialize, Deserialize)]
struct PkgJson {
    name: Option<String>,
    version: Option<Version>,
    description: Option<String>,
}

impl PkgJson {
    pub fn into_metadata(self, path: impl AsRef<Path>) -> Result<VersionMetadata> {
        let PkgJson { name, version, .. } = self;
        let name = name.or_else(|| {
            if let Some(name) = path.as_ref().file_name() {
                Some(name.to_string_lossy().into())
            } else {
                None
            }
        }).ok_or_else(|| Error::MiscError("Failed to find a valid name. Make sure the package.json has a `name` field, or that it exists inside a named directory.".into()))?;
        let version = version.unwrap_or_else(|| Version::parse("0.0.0").expect("Oops, typo"));
        let manifest = OroManifestBuilder::default()
            .name(name)
            .version(version)
            .build()
            .unwrap();
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
            manifest,
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
