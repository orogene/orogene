use std::{fmt::Display, path::PathBuf, sync::Arc};

use colored::*;
use node_semver::{Range as SemVerRange, Version as SemVerVersion};
use oro_common::CorgiPackument;
use oro_package_spec::{GitInfo, PackageSpec, VersionSpec};
use ssri::Integrity;
use url::Url;

use crate::{fetch::PackageFetcher, package::Package, NassunError};

/// Represents a fully-resolved, specific version of a package as it would be fetched.
#[derive(Clone, PartialEq, Eq)]
pub enum PackageResolution {
    Npm {
        name: String,
        version: SemVerVersion,
        tarball: Url,
        integrity: Option<Integrity>,
    },
    Dir {
        name: String,
        path: PathBuf,
    },
    Git {
        name: String,
        info: GitInfo,
    },
}

impl Display for PackageResolution {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use PackageResolution::*;
        match self {
            Npm { tarball, .. } => write!(f, "{tarball}"),
            Dir { path, .. } => write!(f, "file:{}", path.to_string_lossy()),
            Git { info, .. } => write!(f, "{info}"),
        }
    }
}

impl std::fmt::Debug for PackageResolution {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use PackageResolution::*;
        match self {
            Npm {
                tarball,
                name,
                version,
                ..
            } => write!(f, "{name}@{version} ({tarball})"),
            Dir { path, name } => write!(f, "{name}@{}", path.to_string_lossy()),
            Git { name, info } => write!(f, "{name}@{info}"),
        }
    }
}

impl PackageResolution {
    pub fn satisfies(&self, spec: &PackageSpec) -> Result<bool, NassunError> {
        use PackageResolution as PR;
        use PackageSpec as PS;
        Ok(match (self, spec) {
            (PR::Npm { version, .. }, PS::Npm { requested, .. }) => {
                match requested {
                    Some(VersionSpec::Version(v)) => version == v,
                    Some(VersionSpec::Range(r)) => r.satisfies(version),
                    // It's expected that `spec` has previously been resolved at least down to a range.
                    Some(VersionSpec::Tag(_)) => false,
                    None => false,
                }
            }
            (PR::Dir { path: pr_path, .. }, PS::Dir { path: ps_path }) => {
                pr_path == &ps_path.canonicalize()?
            }
            // TODO: Implement this.
            (PR::Git { .. }, PS::Git(..)) => false,
            _ => false,
        })
    }

    pub fn npm_version(&self) -> Option<SemVerVersion> {
        match self {
            Self::Npm { version, .. } => Some(version.clone()),
            _ => None,
        }
    }
}

#[derive(Clone, Debug)]
pub(crate) struct PackageResolver {
    pub(crate) default_tag: String,
    pub(crate) base_dir: PathBuf,
}

impl PackageResolver {
    pub(crate) fn resolve_from(
        &self,
        name: String,
        from: PackageSpec,
        resolved: PackageResolution,
        fetcher: Arc<dyn PackageFetcher>,
    ) -> Package {
        Package {
            name,
            from,
            resolved,
            fetcher,
            base_dir: self.base_dir.clone(),
        }
    }

    pub(crate) async fn resolve(
        &self,
        name: String,
        wanted: PackageSpec,
        fetcher: Arc<dyn PackageFetcher>,
    ) -> Result<Package, NassunError> {
        let packument = fetcher.corgi_packument(&wanted, &self.base_dir).await?;
        let resolved = self.get_resolution(&name, &wanted, &packument)?;
        Ok(Package {
            name,
            from: wanted,
            resolved,
            fetcher,
            base_dir: self.base_dir.clone(),
        })
    }

    fn get_resolution(
        &self,
        name: &str,
        wanted: &PackageSpec,
        packument: &Arc<CorgiPackument>,
    ) -> Result<PackageResolution, NassunError> {
        use PackageSpec::*;
        let spec = match wanted {
            Alias { spec, .. } => spec,
            spec => spec,
        };

        if let Dir { ref path } = spec {
            return Ok(PackageResolution::Dir {
                name: name.into(),
                path: self.base_dir.join(path).canonicalize()?,
            });
        }

        if let Git(info) = spec {
            return Ok(PackageResolution::Git {
                name: name.into(),
                info: info.clone(),
            });
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
            _ => return Err(NassunError::InvalidPackageSpec(spec.clone())),
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
            .ok_or_else(|| NassunError::NoVersion {
                name: name.into(),
                spec: spec.clone(),
                versions: packument.versions.keys().map(|k| k.to_string()).collect(),
            })
            .and_then(|v| {
                if let Some(deprecated) = &v.deprecated {
                    tracing::warn!(
                        "{} {}@{}: {}",
                        "deprecated".magenta(),
                        name,
                        v.manifest
                            .version
                            .as_ref()
                            .map(|v| v.to_string())
                            .unwrap_or_else(|| "unknown".into()),
                        deprecated
                    );
                }

                Ok(PackageResolution::Npm {
                    name: name.into(),
                    version: v
                        .manifest
                        .version
                        .clone()
                        .unwrap_or_else(|| "0.0.0".parse().unwrap()),
                    tarball: if let Some(tarball) = &v.dist.tarball {
                        tarball.clone()
                    } else {
                        return Err(NassunError::NoTarball(
                            name.into(),
                            wanted.clone(),
                            Box::new(v.clone()),
                        ));
                    },
                    integrity: v.dist.integrity.as_ref().map(|i| i.parse()).transpose()?,
                })
            })
    }
}

fn max_satisfying<'a>(
    versions: impl Iterator<Item = &'a SemVerVersion>,
    range: &SemVerRange,
) -> Option<&'a SemVerVersion> {
    versions.filter(|v| range.satisfies(v)).max()
}
