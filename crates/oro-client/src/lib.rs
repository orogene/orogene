//! A general-use client for interacting with NPM registry APIs.

mod api;
mod client;
mod credentials;
mod error;
mod notify;

pub use api::login;
pub use api::packument;
pub use client::{OroClient, OroClientBuilder};
pub use error::OroClientError;
