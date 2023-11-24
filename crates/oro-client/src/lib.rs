//! A general-use client for interacting with NPM registry APIs.

mod api;
mod auth_middleware;
mod authentication_helper;
mod client;
mod credentials;
mod error;
mod traits;

pub use api::login;
pub use api::packument;
pub use api::publish;
pub use auth_middleware::nerf_dart;
pub use authentication_helper::OTPResponse;
pub use client::{OroClient, OroClientBuilder};
pub use error::OroClientError;
