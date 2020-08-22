use async_trait::async_trait;
use oro_node_semver::{Version as SemVerVersion, VersionReq as SemVerRange};
use package_arg::{PackageArg, VersionReq};
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
    #[error("Only Version, Tag, Range, and Alias package args are supported.")]
    InvalidPackageArg,
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
        use PackageArg::*;
        let spec = match wanted.spec() {
            Alias { package, .. } => &*package,
            spec => spec,
        };

        if let Dir { ref path } = spec {
            return Ok(PackageResolution::Dir { path: path.clone() });
        }

        // TODO, move a lot of this out into a generic "PackumentResolver"
        // that takes a package_arg::VersionReq and an existing packument,
        // since it's going to apply to a set of resolvers, but not to all of
        // them.
        let packument = wanted
            .packument()
            .await
            .map_err(|e| ResolverError::OtherError(Box::new(e)))?;
        if packument.versions.is_empty() {
            return Err(ResolverError::NoVersion);
        }

        let mut target: Option<&SemVerVersion> = match spec {
            Npm {
                requested: Some(VersionReq::Version(version)),
                ..
            } => Some(version),
            Npm {
                requested: Some(VersionReq::Tag(tag)),
                ..
            } => packument.tags.get(tag.as_str()),
            Npm {
                requested: Some(VersionReq::Range(_)),
                ..
            }
            | Npm {
                requested: None, ..
            } => None,
            _ => {
                return Err(ResolverError::OtherError(Box::new(
                    ClassicResolverError::InvalidPackageArg,
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
                PackageArg::Npm {
                    requested: None, ..
                } => true,
                PackageArg::Npm {
                    requested: Some(VersionReq::Range(range)),
                    ..
                } => range.satisfies(tag_version.as_ref().unwrap()),
                _ => false,
            }
        {
            target = tag_version;
        }

        if target.is_none() {
            if let Npm {
                requested: Some(VersionReq::Range(range)),
                ..
            } = spec
            {
                target = max_satisfying(packument.versions.keys(), range)
            }
        }

        if target.is_none() {
            if let Npm {
                requested: Some(VersionReq::Range(range)),
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
            .map(|v| PackageResolution::Npm {
                version: v.version.clone(),
                tarball: v.dist.tarball.clone(),
            })
            .ok_or_else(|| ResolverError::NoVersion)
    }
}

fn max_satisfying<'a>(
    versions: impl Iterator<Item = &'a SemVerVersion>,
    range: &SemVerRange,
) -> Option<&'a SemVerVersion> {
    versions.filter(|v| range.satisfies(*v)).max()
}
