use oro_common::miette::Result;

// Re-exports for common command deps:
pub use async_trait;
pub use clap;
pub use dialoguer;
pub use directories;
pub use indicatif;
pub use owo_colors;
pub use oro_config;

#[async_trait::async_trait]
pub trait OroCommand {
    async fn execute(self) -> Result<()>;
}
