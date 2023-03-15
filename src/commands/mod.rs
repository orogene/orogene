use async_trait::async_trait;
use miette::Result;

pub mod ping;
pub mod restore;
pub mod view;

#[async_trait]
pub trait OroCommand {
    async fn execute(self) -> Result<()>;
}
