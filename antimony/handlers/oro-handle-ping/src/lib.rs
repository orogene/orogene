use std::time::Instant;

use anyhow::Result;
use async_trait::async_trait;
use erased_serde::Serialize;
use serde::Deserialize;
use serde_json::Value;

use oro_client::{Method, OroClient};
use oro_config::OroConfig;
use oro_gui_handler::OroHandler;

#[derive(Deserialize)]
pub struct PingHandler;

#[typetag::deserialize(name = "ping")]
#[async_trait]
impl OroHandler for PingHandler {
    async fn execute(self: Box<Self>, config: &OroConfig) -> Result<Box<dyn Serialize>> {
        // TODO: do this better.
        let registry = config
            .get_str("registry")
            .ok()
            .unwrap_or_else(|| "https://registry.npmjs.org".into());
        let start = Instant::now();
        let client = OroClient::new(registry.clone());
        let req = client.opts(Method::Get, "-/ping?write=true");
        let mut res = client.send(req).await?;
        let time = start.elapsed().as_micros() as f32 / 1000.0;
        let details: Value =
            serde_json::from_str(&res.body_string().await.unwrap_or_else(|_| "{}".into()))?;
        let output = serde_json::json!({
            "registry": registry.to_string(),
            "time": time,
            "details": details,
        });
        Ok(Box::new(output))
    }
}
