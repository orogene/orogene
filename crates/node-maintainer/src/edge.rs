use oro_package_spec::PackageSpec;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum DepType {
    Prod,
    Dev,
    Peer,
    Opt,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Edge {
    pub(crate) requested: PackageSpec,
    pub(crate) dep_type: DepType,
}

impl Edge {
    pub(crate) fn new(requested: PackageSpec, dep_type: DepType) -> Self {
        Self {
            requested,
            dep_type,
        }
    }
}
