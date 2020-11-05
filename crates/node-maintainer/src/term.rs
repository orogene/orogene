use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use oro_node_semver::{Version, VersionReq};
use rogga::{PackageRequest, PackageSpec, VersionSpec};

use crate::error::{Error, Internal, Result};
use crate::set_relation::SetRelation;

// TODO: Implement Debug, Eq, PArtialEq
#[derive(Clone, Debug, PartialEq, Eq)]
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
            return Err(Error::NameMismatch(
                self.request.name().clone(),
                other.request.name().clone(),
            ));
        }
        spec_relation((&self, self.request.spec()), (&other, other.request.spec())).await
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
    left: (&'a Term, &'a PackageSpec),
    right: (&'a Term, &'a PackageSpec),
) -> Pin<Box<dyn Future<Output = Result<SetRelation>> + 'a>> {
    // NOTE: Shenanigans because async fn need to be boxed futures instead.
    Box::pin(async move {
        use PackageSpec::*;
        Ok(match (left.1, right.1) {
            (Dir { .. }, Dir { .. }) => dir_relation(left, right)?,
            (Npm { .. }, Npm { .. }) => npm_relation(left, right).await?,
            (Alias { ref package, .. }, _) => spec_relation((left.0, package), right).await?,
            (_, Alias { ref package, .. }) => spec_relation(left, (right.0, package)).await?,
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

fn dir_relation(left: (&Term, &PackageSpec), right: (&Term, &PackageSpec)) -> Result<SetRelation> {
    Ok(match (left.1, right.1) {
        (
            PackageSpec::Dir {
                path: ref left_path,
                from: ref left_from,
            },
            PackageSpec::Dir {
                path: ref right_path,
                from: ref right_from,
            },
        ) => {
            // TODO: would be nice to cache these some day, huh
            let left_path = left_from.join(left_path).canonicalize().to_internal()?;
            let right_path = right_from.join(right_path).canonicalize().to_internal()?;
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
    left: (&Term, &PackageSpec),
    right: (&Term, &PackageSpec),
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

async fn npm_version_req(req: &(&Term, &PackageSpec)) -> Result<VersionReq> {
    if let PackageSpec::Npm { ref requested, .. } = req.1 {
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
                    return Err(Error::TagNotFound(tag.clone()));
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
