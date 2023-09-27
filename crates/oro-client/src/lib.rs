//! A general-use client for interacting with NPM registry APIs.

mod api;
mod auth_middleware;
mod client;
mod credentials;
mod error;
mod notify;

pub use api::login;
pub use api::packument;
pub use auth_middleware::nerf_dart;
pub use client::{OroClient, OroClientBuilder};
pub use error::OroClientError;
