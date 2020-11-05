use std::collections::HashMap;
use std::iter;
use std::sync::Arc;

use crate::term::Term;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Incompat {
    cause: IncompatCause,
    terms: Vec<Term>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IncompatCause {
    Root,
    NoVersions,
    Dependency,
    Conflict {
        conflict: Arc<Incompat>,
        other: Arc<Incompat>,
    },
}

impl Incompat {
    pub fn new(mut terms: Vec<Term>, cause: IncompatCause) -> Incompat {
        // Remove the root package from generated incompatibilities, since it will
        // always be satisfied. This makes error reporting clearer, and may also
        // make solving more efficient.
        if let IncompatCause::Conflict { .. } = cause {
            if terms.len() != 1 && terms.iter().any(|t| t.positive && t.root) {
                // filter root term out of terms
                terms = terms
                    .into_iter()
                    .filter(|term| !term.positive || !term.root)
                    .collect();
            }
        }

        if terms.len() == 1
            // Short-circuit in the common case of a two-term incompatibility with
            // two different packages (for example, a dependency).
            || (terms.len() == 2 && terms[0].request.name() != terms[1].request.name())
        {
            return Incompat { terms, cause };
        }

        // Coalesce multiple terms about the same package if possible.
        let mut by_name = HashMap::new();
        for term in terms.iter() {
            if !by_name.contains_key(term.request.name()) {
                let hash: HashMap<Arc<rogga::PackageRequest>, Term> = HashMap::new();
                by_name.insert(term.request.name(), hash);
            }
            let by_req = by_name.get_mut(term.request.name()).unwrap(); // Safe unwrap
            if by_req.contains_key(&term.request) {
                by_req.insert(
                    term.request.clone(),
                    by_req
                        .get(&term.request)
                        .unwrap()
                        .intersect(term)
                        // If we have two terms that refer to the same package but have a null
                        // intersection, they're mutually exclusive, making this incompatibility
                        // irrelevant, since we already know that mutually exclusive version
                        // ranges are incompatible. We should never derive an irrelevant
                        // incompatibility.
                        .expect("Can't be None"),
                );
            } else {
                by_req.insert(term.request.clone(), term.clone());
            }
        }

        Incompat {
            cause,
            terms: by_name
                .values()
                .flat_map(|by_ref| {
                    let positive_terms = by_ref
                        .values()
                        .filter(|term| term.positive)
                        .collect::<Vec<&Term>>();
                    if !positive_terms.is_empty() {
                        positive_terms.into_iter().cloned().collect::<Vec<Term>>()
                    } else {
                        by_ref.values().cloned().collect::<Vec<Term>>()
                    }
                })
                .collect::<Vec<Term>>(),
        }
    }

    pub fn is_failure(&self) -> bool {
        self.terms.is_empty() || (self.terms.len() == 1 && self.terms[0].root)
    }

    pub fn external_incompats(&self) -> Box<dyn Iterator<Item = Incompat>> {
        if let IncompatCause::Conflict { conflict, other } = self.cause.clone() {
            Box::new(
                conflict
                    .external_incompats()
                    .chain(other.external_incompats()),
            )
        } else {
            Box::new(iter::once(self.clone()))
        }
    }
}
