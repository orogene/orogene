use std::time::Instant;

use async_trait::async_trait;
use clap::Args;
use miette::{IntoDiagnostic, Result, WrapErr};
use oro_client::{self, OroClient};
use oro_command::OroCommand;
use oro_config::OroConfigLayer;
use serde_json::Value;
use url::Url;

#[derive(Debug, Args, OroConfigLayer)]
pub struct PingCmd {
    /// Registry to ping.
    #[arg(from_global)]
    registry: Option<Url>,

    #[clap(from_global)]
    json: bool,

    #[clap(from_global)]
    quiet: bool,
}

#[async_trait]
impl OroCommand for PingCmd {
    async fn execute(self) -> Result<()> {
        let start = Instant::now();
        let registry = self
            .registry
            .unwrap_or_else(|| "https://registry.npmjs.org".parse().unwrap());
        if !self.quiet && !self.json {
            eprintln!("ping: {}", registry);
        }
        let client = OroClient::new(registry.clone());
        let payload = client.ping().await?;
        let time = start.elapsed().as_micros() as f32 / 1000.0;
        if !self.quiet && !self.json {
            eprintln!("pong: {time}ms");
        }
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
            if !self.quiet {
                println!("{output}");
            }
        } else if !self.quiet {
            eprintln!("payload: {payload}");
        }
        Ok(())
    }
}
