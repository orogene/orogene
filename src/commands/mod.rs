use async_trait::async_trait;
use miette::Result;

pub mod add;
pub mod apply;
pub mod ping;
pub mod reapply;
pub mod remove;
pub mod view;

#[async_trait]
pub trait OroCommand {
    async fn execute(self) -> Result<()>;
}
