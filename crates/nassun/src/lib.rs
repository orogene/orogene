//! An NPM registry-compatible package interface. You can use it for
//! resolving, fetching metadata for, and downloading individual packages.

use futures::AsyncRead;
pub use oro_package_spec::{GitHost, GitInfo, PackageSpec, VersionSpec};

mod client;
mod entries;
mod error;
mod fetch;
mod package;
mod resolver;
mod tarball;
#[cfg(target_arch = "wasm32")]
mod wasm;

pub use client::*;
pub use entries::*;
pub use error::NassunError;
pub use package::*;
pub use resolver::*;
pub use tarball::*;
#[cfg(target_arch = "wasm32")]
pub use wasm::*;

#[cfg(not(target_arch = "wasm32"))]
pub(crate) type TarballStream = Box<dyn AsyncRead + Unpin + Send + Sync>;
#[cfg(target_arch = "wasm32")]
pub(crate) type TarballStream = Box<dyn AsyncRead + Unpin>;
