use std::time::Instant;

use anyhow::{Context, Result};
use async_trait::async_trait;
use clap::Clap;
use oro_client::OroClient;
use oro_command::OroCommand;
use oro_error_code::OroErrCode as Code;
use serde_json::Value;
use url::Url;

#[derive(Debug, Clap, OroCommand)]
pub struct PingCmd {
    #[clap(
        about = "Registry to ping.",
        default_value = "https://registry.npmjs.org",
        long
    )]
    registry: Url,
    #[clap(from_global)]
    loglevel: log::LevelFilter,
    #[clap(from_global)]
    json: bool,
    #[clap(from_global)]
    quiet: bool,
}

#[async_trait]
impl OroCommand for PingCmd {
    async fn execute(self) -> Result<()> {
        let quiet = self.loglevel == log::LevelFilter::Off || self.quiet;
        let start = Instant::now();
        if !quiet && !self.json {
            eprintln!("ping: {}", self.registry);
        }
        let mut res = OroClient::new(self.registry.clone())
            .get("-/ping?write=true")
            .await
            .with_context(|| Code::OR1001(self.registry.to_string()))?;
        let time = start.elapsed().as_micros() as f32 / 1000.0;
        if !quiet && !self.json {
            eprintln!("pong: {}ms", time);
        }
        if self.json {
            let details: Value =
                serde_json::from_str(&res.body_string().await.unwrap_or_else(|_| "{}".into()))
                    .context(Code::OR1004)?;
            let output = serde_json::to_string_pretty(&serde_json::json!({
                "registry": self.registry.to_string(),
                "time": time,
                "details": details,
            }))?;
            if !quiet {
                println!("{}", output);
            }
        } else if !quiet {
            eprintln!(
                "payload: {}",
                res.body_string().await.unwrap_or_else(|_| "".into())
            );
        }
        Ok(())
    }
}
