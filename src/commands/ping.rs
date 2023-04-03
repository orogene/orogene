use std::time::Instant;

use async_trait::async_trait;
use clap::Args;
use miette::{IntoDiagnostic, Result, WrapErr};
use oro_client::{self, OroClient};
use serde_json::Value;
use url::Url;

use crate::commands::OroCommand;

#[derive(Debug, Args)]
pub struct PingCmd {
    #[arg(from_global)]
    registry: Url,

    #[arg(from_global)]
    json: bool,
}

#[async_trait]
impl OroCommand for PingCmd {
    async fn execute(self) -> Result<()> {
        let start = Instant::now();
        let registry = self.registry;
        tracing::info!("ping: {registry}");
        let client = OroClient::new(registry.clone());
        let payload = client.ping().await?;
        let time = start.elapsed().as_micros() as f32 / 1000.0;
        tracing::info!("pong: {time}ms");
        if self.json {
            let details: Value = serde_json::from_str(&payload)
                .into_diagnostic()
                .wrap_err("ping::deserialize")?;
            let output = serde_json::to_string_pretty(&serde_json::json!({
                "registry": registry.to_string(),
                "time": time,
                "details": details,
            }))
            .into_diagnostic()
            .wrap_err("ping::serialize")?;
            println!("{output}");
        } else {
            tracing::info!("payload: {payload}");
        }
        Ok(())
    }
}
