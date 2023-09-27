//! An NPM dependency resolver for building `node_modules/` trees and
//! extracting them to their final resting place.

pub use nassun::Nassun;
#[cfg(not(target_arch = "wasm32"))]
pub use nassun::{NassunError, NassunOpts};

pub use error::*;
pub use into_kdl::IntoKdl;
pub use lockfile::*;
#[cfg(not(target_arch = "wasm32"))]
pub use maintainer::*;
#[cfg(target_arch = "wasm32")]
mod wasm;

mod error;
mod graph;
mod into_kdl;
mod linkers;
mod lockfile;
mod maintainer;
mod resolver;
#[cfg(target_arch = "wasm32")]
pub use wasm::*;
