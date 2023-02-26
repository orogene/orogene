//! General types and utilities for Orogene, including
//! packument/package.json/manifest types.

pub use manifest::Bin;
pub use manifest::*;
pub use packument::*;

mod manifest;
mod packument;
