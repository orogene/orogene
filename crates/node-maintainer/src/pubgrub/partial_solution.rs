/*
use std::collections::HashMap;
use std::sync::Arc;

use rogga::{PackageRequest, PackageResolution};

use crate::assignment::Assignment;

pub struct PartialSolution {
    // assignments: Vec<Assignment>,
    decisions: HashMap<String, PackageResolution>,
    positive: HashMap<String, Assignment>,
    // negative: HashMap<String, HashMap<PackageRequest, Assignment>>,
    // attempted_solutions: usize,
    // backtracking: bool,
}

impl PartialSolution {
    pub fn decisions(&self) -> impl Iterator<Item = &PackageResolution> {
        self.decisions.values()
    }

    pub fn decision_level(&self) -> usize {
        self.decisions.len()
    }

    pub fn unsatisfied(&self) -> impl Iterator<Item = Arc<PackageRequest>> + '_ {
        self.positive
            .values()
            .filter(|assignment| !self.decisions.contains_key(assignment.term.request.name()))
            .map(|assignment| assignment.term.request.clone())
            .collect::<Vec<Arc<PackageRequest>>>()
            .into_iter()
    }
}
*/
