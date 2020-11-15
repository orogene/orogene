use async_trait::async_trait;
use oro_diagnostics::{Diagnostic, DiagnosticCategory};
use oro_node_semver::{Version as SemVerVersion, VersionReq as SemVerRange};
use oro_package_spec::{PackageSpec, VersionSpec};
use rogga::{PackageRequest, PackageResolution, PackageResolver, ResolverError};
use thiserror::Error;

pub struct ClassicResolver {
    pub default_tag: String,
}

impl Default for ClassicResolver {
    fn default() -> Self {
        ClassicResolver {
            default_tag: "latest".into(),
        }
    }
}

#[derive(Debug, Error)]
pub enum ClassicResolverError {
    #[error("Only Version, Tag, Range, and Alias package args are supported, but got `{0}`.")]
    InvalidPackageSpec(PackageSpec),
    #[error(transparent)]
    IoError(#[from] std::io::Error),
}

impl Diagnostic for ClassicResolverError {
    fn category(&self) -> DiagnosticCategory {
        todo!()
    }

    fn subpath(&self) -> String {
        todo!()
    }

    fn advice(&self) -> Option<String> {
        todo!()
    }
}

impl ClassicResolver {
    pub fn new() -> Self {
        ClassicResolver::default()
    }

    pub fn default_tag(mut self, tag: String) -> Self {
        self.default_tag = tag;
        self
    }
}

#[async_trait]
impl PackageResolver for ClassicResolver {
    async fn resolve(&self, wanted: &PackageRequest) -> Result<PackageResolution, ResolverError> {
        use PackageSpec::*;
        let spec = match wanted.spec() {
            Alias { spec, .. } => &*spec,
            spec => spec,
        };

        if let Dir { ref path } = spec {
            return Ok(PackageResolution::Dir {
                path: wanted.base_dir().join(path).canonicalize().map_err(|e| {
                    ResolverError::OtherError(Box::new(ClassicResolverError::IoError(e)))
                })?,
            });
        }

        // TODO, move a lot of this out into a generic "PackumentResolver"
        // that takes an oro_package_spec::VersionReq and an existing packument,
        // since it's going to apply to a set of resolvers, but not to all of
        // them.
        let packument = wanted
            .packument()
            .await
            .map_err(|e| ResolverError::OtherError(Box::new(e)))?;
        if packument.versions.is_empty() {
            return Err(ResolverError::NoVersion {
                name: wanted.name().clone(),
                spec: wanted.spec().clone(),
                versions: Vec::new(),
            });
        }

        let mut target: Option<&SemVerVersion> = match spec {
            Npm {
                requested: Some(VersionSpec::Version(version)),
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
                return Err(ResolverError::OtherError(Box::new(
                    ClassicResolverError::InvalidPackageSpec(spec.clone()),
                )))
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
                requested: Some(VersionSpec::Range(range)),
                ..
            } = spec
            {
                target = max_satisfying(packument.versions.keys(), range);
                if target.is_none() {
                    eprintln!("Failed to find version for {}", wanted.name());
                }
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
            .and_then(|v| packument.versions.get(&v))
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
            .ok_or_else(|| ResolverError::NoVersion {
                name: wanted.name().clone(),
                spec: wanted.spec().clone(),
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
