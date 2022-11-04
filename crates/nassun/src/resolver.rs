use std::{fmt::Display, path::PathBuf, sync::Arc};

use node_semver::{Range as SemVerRange, Version as SemVerVersion};
use oro_common::Packument;
use oro_package_spec::{GitInfo, PackageSpec, VersionSpec};
use url::Url;

use crate::{fetch::PackageFetcher, package::Package, NassunError};

/// Represents a fully-resolved, specific version of a package as it would be fetched.
#[derive(Clone, Debug)]
pub enum PackageResolution {
    Npm {
        version: SemVerVersion,
        tarball: Url,
    },
    Dir {
        path: PathBuf,
    },
    Git(GitInfo),
}

impl Display for PackageResolution {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use PackageResolution::*;
        match self {
            Npm { tarball, .. } => write!(f, "{}", tarball),
            Dir { path } => write!(f, "{}", path.to_string_lossy()),
            Git(info) => write!(f, "{}", info),
        }
    }
}

#[derive(Clone, Debug)]
pub(crate) struct PackageResolver {
    pub(crate) default_tag: String,
    pub(crate) base_dir: PathBuf,
}

impl PackageResolver {
    pub(crate) async fn resolve(
        &self,
        name: String,
        wanted: PackageSpec,
        fetcher: Arc<dyn PackageFetcher>,
    ) -> Result<Package, NassunError> {
        let packument = fetcher.packument(&wanted, &self.base_dir).await?;
        let resolved = self.get_resolution(&name, &wanted, &packument)?;
        Ok(Package {
            name,
            from: wanted,
            resolved,
            fetcher,
            packument,
        })
    }

    fn get_resolution(
        &self,
        name: &str,
        wanted: &PackageSpec,
        packument: &Arc<Packument>,
    ) -> Result<PackageResolution, NassunError> {
        use PackageSpec::*;
        let spec = match wanted {
            Alias { spec, .. } => spec,
            spec => spec,
        };

        if let Dir { ref path } = spec {
            return Ok(PackageResolution::Dir {
                path: self.base_dir.join(path).canonicalize()?,
            });
        }

        if let Git(info) = spec {
            return Ok(PackageResolution::Git(info.clone()));
        }

        if packument.versions.is_empty() {
            return Err(NassunError::NoVersion {
                name: name.into(),
                spec: spec.clone(),
                versions: Vec::new(),
            });
        }

        let mut target: Option<&SemVerVersion> = match spec {
            Npm {
                requested: Some(VersionSpec::Version(ref version)),
                ..
            } => Some(version),
            Npm {
                requested: Some(VersionSpec::Tag(tag)),
                ..
            } => packument.tags.get(tag.as_str()),
            Npm {
                requested: Some(VersionSpec::Range(_)),
                ..
            }
            | Npm {
                requested: None, ..
            } => None,
            _ => {
                return Err(NassunError::InvalidPackageSpec(spec.clone()))
            }
        };

        let tag_version = packument.tags.get(&self.default_tag);

        if target.is_none()
            && tag_version.is_some()
            && packument
                .versions
                .get(tag_version.as_ref().unwrap())
                .is_some()
            && match spec {
                PackageSpec::Npm {
                    requested: None, ..
                } => true,
                PackageSpec::Npm {
                    requested: Some(VersionSpec::Range(range)),
                    ..
                } => range.satisfies(tag_version.as_ref().unwrap()),
                _ => false,
            }
        {
            target = tag_version;
        }

        if target.is_none() {
            if let Npm {
                requested: Some(VersionSpec::Range(ref range)),
                ..
            } = spec
            {
                target = max_satisfying(packument.versions.keys(), range);
            }
        }

        if target.is_none() {
            if let Npm {
                requested: Some(VersionSpec::Range(range)),
                ..
            } = spec
            {
                if range == &SemVerRange::any() || range == &SemVerRange::parse("*").unwrap() {
                    target = tag_version;
                }
            }
        }

        target
            .and_then(|v| packument.versions.get(v))
            .and_then(|v| {
                Some(PackageResolution::Npm {
                    version: v
                        .manifest
                        .version
                        .clone()
                        .unwrap_or_else(|| "0.0.0".parse().unwrap()),
                    tarball: if let Some(tarball) = &v.dist.tarball {
                        tarball.clone()
                    } else {
                        return None;
                    },
                })
            })
            .ok_or_else(|| NassunError::NoVersion {
                name: name.into(),
                spec: spec.clone(),
                versions: packument.versions.keys().map(|k| k.to_string()).collect(),
            })
    }
}

fn max_satisfying<'a>(
    versions: impl Iterator<Item = &'a SemVerVersion>,
    range: &SemVerRange,
) -> Option<&'a SemVerVersion> {
    versions.filter(|v| range.satisfies(*v)).max()
}
