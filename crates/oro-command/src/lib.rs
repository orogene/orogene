use async_trait::async_trait;
use miette::Result;

#[async_trait]
pub trait OroCommand {
    async fn execute(self) -> Result<()>;
}
