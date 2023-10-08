use std::{fmt::Display, path::PathBuf, sync::Arc};

use node_semver::{Range as SemVerRange, Version as SemVerVersion};
use oro_common::CorgiPackument;
use oro_package_spec::{GitInfo, PackageSpec, VersionSpec};
use ssri::Integrity;
use url::Url;

use crate::error::{IoContext, NassunError};
use crate::fetch::PackageFetcher;
use crate::package::Package;

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

impl PackageResolution {
    pub fn integrity(&self) -> Option<&Integrity> {
        use PackageResolution::*;
        match self {
            Npm { integrity, .. } => integrity.as_ref(),
            Dir { .. } => None,
            Git { .. } => None,
        }
    }
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
            Npm { name, version, .. } => write!(f, "{name}@{version}"),
            Dir { path, name } => write!(f, "{name}@{}", path.to_string_lossy()),
            Git { name, info } => write!(f, "{name}@{info}"),
        }
    }
}

impl PackageResolution {
    pub fn satisfies(&self, spec: &PackageSpec) -> Result<bool, NassunError> {
        use PackageResolution as PR;
        use PackageSpec as PS;
        Ok(match (self, spec.target()) {
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
                pr_path
                    == &ps_path.canonicalize().io_context(|| {
                        format!("Failed to canonicalize path: {}.", ps_path.display())
                    })?
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
        cache: Arc<Option<PathBuf>>,
    ) -> Package {
        Package {
            name,
            from,
            resolved,
            fetcher,
            cache,
            base_dir: self.base_dir.clone(),
        }
    }

    pub(crate) async fn resolve(
        &self,
        name: String,
        wanted: PackageSpec,
        fetcher: Arc<dyn PackageFetcher>,
        cache: Arc<Option<PathBuf>>,
    ) -> Result<Package, NassunError> {
        let packument = fetcher.corgi_packument(&wanted, &self.base_dir).await?;
        let resolved = self.get_resolution(&name, &wanted, &packument)?;
        Ok(Package {
            name,
            from: wanted,
            resolved,
            fetcher,
            base_dir: self.base_dir.clone(),
            cache,
        })
    }

    fn get_resolution(
        &self,
        name: &str,
        wanted: &PackageSpec,
        packument: &Arc<CorgiPackument>,
    ) -> Result<PackageResolution, NassunError> {
        use PackageSpec::*;
        let spec = wanted.target();

        if let Dir { ref path } = spec {
            let p = self.base_dir.join(path);
            return Ok(PackageResolution::Dir {
                name: name.into(),
                path: p
                    .canonicalize()
                    .io_context(|| format!("Failed to canonicalize path at {}.", p.display()))?,
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
                    integrity: v
                        .dist
                        .integrity
                        .as_ref()
                        .map(|i| i.parse())
                        .or_else(|| {
                            v.dist
                                .shasum
                                .as_ref()
                                .map(|s| format!("sha1-{}", s).parse())
                        })
                        .transpose()?,
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

#[cfg(test)]
mod tests {
    use super::*;
    use test_case::test_case;

    #[test_case("1.4.2", "1.4.2", true; "exact version match")]
    #[test_case("1.4.2", "~1.4.0", true; "same minor version")]
    #[test_case("1.4.2", "~1", true; "tilde same major version")]
    #[test_case("1.4.2", "^1.0.0", true; "same major version")]
    #[test_case("1.4.2", ">=1.0.0", true; "minimum version (same major)")]
    #[test_case("2.4.2", ">=1.0.0", true; "minimum version (higher major)")]
    #[test_case("1.4.2", "1.0.0 - 1.9.0", true; "in range")]
    #[test_case("1.0.0-rc.10", ">=1.0.0-rc.5 <1.0.0", true; "pre-release in range")]
    #[test_case("1.4.2", "1", true; "partial major version")]
    #[test_case("1.4.2", "1.x", true; "x minor version")]
    #[test_case("1.4.2", "1.4.x", true; "x patch version")]
    #[test_case("1.4.2", "1.*", true; "star minor version")]
    #[test_case("1.4.2", "1.4.*", true; "star patch version")]
    // negative cases
    #[test_case("1.4.3", "1.4.2", false; "mismatch exact version match")]
    #[test_case("1.5.2", "~1.4.0", false; "mismatch same minor version")]
    #[test_case("2.4.2", "~1", false; "mismatch tilde same major version")]
    #[test_case("2.4.2", "^1.0.0", false; "mismatch same major version")]
    #[test_case("1.4.2", ">=2.0.0", false; "mismatch minimum version (same major)")]
    #[test_case("2.4.2", "1.0.0 - 1.9.0", false; "out of range")]
    #[test_case("2.0.0-rc.10", ">=1.0.0-rc.5 <1.0.0", false; "pre-release out of range")]
    #[test_case("2.4.2", "1", false; "mismatch partial major version")]
    #[test_case("2.4.2", "1.x", false; "mismatch x minor version")]
    #[test_case("1.5.2", "1.4.x", false; "mismatch x patch version")]
    #[test_case("2.4.2", "1.*", false; "mismatch star minor version")]
    #[test_case("1.5.2", "1.4.*", false; "mismatch star patch version")]
    fn satisfies_npm_specs(package_version: &str, expected_version: &str, satifies: bool) {
        let resolution = PackageResolution::Npm {
            name: "oro-test-package".to_owned(),
            version: SemVerVersion::parse(package_version).unwrap(),
            tarball: Url::parse("https://example.com/npm/oro-test-package.tar.gz").unwrap(),
            integrity: None,
        };
        let package_spec = PackageSpec::Npm {
            scope: None,
            name: "oro-test-package".to_owned(),
            requested: Some(VersionSpec::Range(
                SemVerRange::parse(expected_version).unwrap(),
            )),
        };
        assert_eq!(resolution.satisfies(&package_spec).unwrap(), satifies);
    }
}
