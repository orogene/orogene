use anyhow::Result;
use async_trait::async_trait;

#[async_trait]
pub trait OroCommand {
    async fn execute(self) -> Result<()>;
}
