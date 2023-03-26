//! General types and utilities for Orogene, including
//! packument/package.json/manifest types.

pub use build_manifest::*;
pub use manifest::Bin;
pub use manifest::*;
pub use packument::*;

mod build_manifest;
mod manifest;
mod packument;
