use serde::Deserialize;
use ssri::Integrity;

#[derive(Clone, Debug, Deserialize)]
pub struct Manifest {
    pub name: Option<String>,
    pub version: Option<String>,
    pub integrity: Option<Integrity>,
    pub resolved: String,
}

#[derive(Clone, Debug, Deserialize)]
pub struct Packument {}
