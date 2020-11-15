use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use oro_node_semver::{Version, VersionReq};
use rogga::{PackageRequest, PackageSpec, VersionSpec};

use crate::error::{Internal, NodeMaintainerError, Result};
use crate::set_relation::SetRelation;

// TODO: Implement Debug, Eq, PArtialEq
#[derive(Clone, PartialEq, Eq)]
pub struct Term {
    pub positive: bool,
    pub root: bool,
    pub request: Arc<PackageRequest>,
}

impl Term {
    pub fn new(request: Arc<PackageRequest>, positive: bool, root: bool) -> Self {
        Self {
            root,
            positive,
            request,
        }
    }

    pub fn invert(&self) -> Self {
        Self::new(self.request.clone(), !self.positive, self.root)
    }

    pub async fn relation(&self, other: &Term) -> Result<SetRelation> {
        if self.request.name() != other.request.name() {
            return Err(NodeMaintainerError::NameMismatch(
                self.request.name().clone(),
                other.request.name().clone(),
            ));
        }
        spec_relation((&self, &self.request), (&other, &other.request)).await
    }

    pub fn intersect(&self, other: &Term) -> Option<Term> {
        assert_eq!(
            self.request.name(),
            other.request.name(),
            "Terms must refer to packages with the same name."
        );
        todo!()
    }

    pub fn difference(&self, other: &Term) -> Option<Term> {
        self.intersect(&other.invert())
    }
}

fn spec_relation<'a>(
    left: (&'a Term, &'a PackageRequest),
    right: (&'a Term, &'a PackageRequest),
) -> Pin<Box<dyn Future<Output = Result<SetRelation>> + 'a>> {
    // NOTE: Shenanigans because async fn need to be boxed futures instead.
    Box::pin(async move {
        use PackageSpec::*;
        Ok(match (left.1.spec(), right.1.spec()) {
            (Dir { .. }, Dir { .. }) => dir_relation(left, right)?,
            (Npm { .. }, Npm { .. }) => npm_relation(left, right).await?,
            (Alias { ref spec, .. }, _) => match **spec {
                Dir { .. } => dir_relation(left, right)?,
                Npm { .. } => npm_relation(left, right).await?,
                _ => unreachable!(),
            },
            _ => {
                if left.0.positive && right.0.positive {
                    SetRelation::Disjoint
                } else if !left.0.positive && right.0.positive {
                    SetRelation::Overlapping
                } else if left.0.positive && !right.0.positive {
                    SetRelation::Subset
                } else {
                    SetRelation::Overlapping
                }
            }
        })
    })
}

fn dir_relation(
    left: (&Term, &PackageRequest),
    right: (&Term, &PackageRequest),
) -> Result<SetRelation> {
    Ok(match (left.1.spec(), right.1.spec()) {
        (
            PackageSpec::Dir {
                path: ref left_path,
            },
            PackageSpec::Dir {
                path: ref right_path,
            },
        ) => {
            // TODO: would be nice to cache these some day, huh
            let left_path = left
                .1
                .base_dir()
                .join(left_path)
                .canonicalize()
                .to_internal()?;
            let right_path = right
                .1
                .base_dir()
                .join(right_path)
                .canonicalize()
                .to_internal()?;
            if left.0.positive == right.0.positive {
                if left_path == right_path {
                    SetRelation::Overlapping
                } else {
                    SetRelation::Disjoint
                }
            } else if left_path == right_path {
                SetRelation::Disjoint
            } else {
                SetRelation::Overlapping
            }
        }
        _ => unreachable!(),
    })
}

async fn npm_relation(
    left: (&Term, &PackageRequest),
    right: (&Term, &PackageRequest),
) -> Result<SetRelation> {
    let left_req = npm_version_req(&left).await?;
    let right_req = npm_version_req(&right).await?;
    Ok(if right.0.positive {
        if left.0.positive {
            if right_req.allows_all(&left_req) {
                SetRelation::Subset
            } else if !left_req.allows_any(&right_req) {
                SetRelation::Disjoint
            } else {
                SetRelation::Overlapping
            }
        } else if left_req.allows_all(&right_req) {
            SetRelation::Disjoint
        } else {
            SetRelation::Overlapping
        }
    } else if left.0.positive {
        if !right_req.allows_any(&left_req) {
            SetRelation::Subset
        } else if right_req.allows_all(&left_req) {
            SetRelation::Disjoint
        } else {
            SetRelation::Overlapping
        }
    } else if left_req.allows_all(&right_req) {
        SetRelation::Subset
    } else {
        SetRelation::Overlapping
    })
}

async fn npm_version_req(req: &(&Term, &PackageRequest)) -> Result<VersionReq> {
    if let PackageSpec::Npm { ref requested, .. } = req.1.spec() {
        let vspec = if let Some(vspec) = requested {
            vspec.clone()
        } else {
            // TODO: this should be configurable? oh god it's gonna suck
            VersionSpec::Tag("latest".into())
        };
        use VersionSpec::*;
        Ok(match vspec {
            Tag(ref tag) => {
                // TODO: Oh no this triggers a clone. Should .packument() just return &Packument?
                let packument = req.0.request.packument().await.to_internal()?;
                let version = if let Some(version) = packument.tags.get(tag) {
                    version
                } else {
                    return Err(NodeMaintainerError::TagNotFound(tag.clone()));
                };
                version_to_exact(version)
            }
            Range(vreq) => vreq,
            Version(ref version) => version_to_exact(version),
        })
    } else {
        unreachable!()
    }
}

fn version_to_exact(v: &Version) -> VersionReq {
    format!("={}", v).parse().unwrap()
}
