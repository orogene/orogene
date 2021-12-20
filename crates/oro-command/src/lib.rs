use oro_common::{async_trait::async_trait, miette::Result};

// Re-exports for common command deps:
pub use clap;
pub use dialoguer;
pub use directories;
pub use indicatif;
pub use oro_config;
pub use owo_colors;

#[async_trait]
pub trait OroCommand {
    async fn execute(self) -> Result<()>;
}
