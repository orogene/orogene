//! An NPM registry-compatible package interface. You can use it for
//! resolving, fetching metadata for, and downloading individual packages.

use futures::AsyncRead;
pub use oro_package_spec::{GitHost, GitInfo, PackageSpec, VersionSpec};

pub mod client;
pub mod entries;
mod error;
pub mod fetch;
pub mod package;
pub mod resolver;
pub mod tarball;
#[cfg(target_arch = "wasm32")]
mod wasm;

#[cfg(not(target_arch = "wasm32"))]
pub use client::*;
#[cfg(not(target_arch = "wasm32"))]
pub use entries::*;
#[cfg(not(target_arch = "wasm32"))]
pub use error::NassunError;
#[cfg(not(target_arch = "wasm32"))]
pub use package::*;
pub use resolver::*;
#[cfg(not(target_arch = "wasm32"))]
pub use tarball::*;
#[cfg(target_arch = "wasm32")]
pub use wasm::*;

#[cfg(not(target_arch = "wasm32"))]
pub(crate) type TarballStream = Box<dyn AsyncRead + Unpin + Send + Sync>;
#[cfg(target_arch = "wasm32")]
pub(crate) type TarballStream = Box<dyn AsyncRead + Unpin>;
