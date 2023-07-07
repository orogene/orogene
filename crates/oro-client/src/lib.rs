//! A general-use client for interacting with NPM registry APIs.

mod api;
mod client;
mod error;

pub use api::packument;
pub use client::{OroClient, OroClientBuilder, OroClientProxyConfig};
pub use error::OroClientError;
