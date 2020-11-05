use rogga::PackageResolution;

use crate::incompat::Incompat;
use crate::term::Term;

pub struct Assignment {
    pub term: Term,
    pub resolution: Option<PackageResolution>,
    pub decision_level: usize,
    pub index: usize,
    pub cause: Option<Incompat>,
}

impl Assignment {
    pub fn is_decision(&self) -> bool {
        self.cause.is_none()
    }

    pub fn decision(
        term: Term,
        resolution: PackageResolution,
        decision_level: usize,
        index: usize,
    ) -> Self {
        Self {
            term,
            resolution: Some(resolution),
            decision_level,
            index,
            cause: None,
        }
    }

    pub fn derivation(term: Term, cause: Incompat, decision_level: usize, index: usize) -> Self {
        Self {
            term,
            resolution: None,
            cause: Some(cause),
            decision_level,
            index,
        }
    }
}
