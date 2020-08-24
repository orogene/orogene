use anyhow::Result;
use async_trait::async_trait;
use erased_serde::Serialize;

use oro_config::OroConfig;

#[async_trait]
#[typetag::deserialize(tag = "cmd", content = "args")]
pub trait OroHandler: Send + Sync {
    async fn execute(self: Box<Self>, config: &OroConfig) -> Result<Box<dyn Serialize>>;
}
