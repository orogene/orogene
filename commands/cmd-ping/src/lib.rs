use std::time::Instant;

use async_trait::async_trait;
use clap::Clap;
use miette::{IntoDiagnostic, Result, WrapErr};
use oro_client::{self, Method, OroClient};
use oro_command::OroCommand;
use oro_config::OroConfigLayer;
use serde_json::Value;
use url::Url;

#[derive(Debug, Clap, OroConfigLayer)]
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
        let start = Instant::now();
        if !self.quiet && !self.json {
            eprintln!("ping: {}", self.registry);
        }
        let client = OroClient::new();
        let req = client.opts(
            Method::Get,
            self.registry.join("-/ping?write=true").unwrap(),
        );
        let mut res = client.send(req).await?;
        let time = start.elapsed().as_micros() as f32 / 1000.0;
        if !self.quiet && !self.json {
            eprintln!("pong: {}ms", time);
        }
        if self.json {
            let details: Value =
                serde_json::from_str(&res.body_string().await.unwrap_or_else(|_| "{}".into()))
                    .into_diagnostic()
                    .wrap_err("ping::deserialize")?;
            let output = serde_json::to_string_pretty(&serde_json::json!({
                "registry": self.registry.to_string(),
                "time": time,
                "details": details,
            }))
            .into_diagnostic()
            .wrap_err("ping::serialize")?;
            if !self.quiet {
                println!("{}", output);
            }
        } else if !self.quiet {
            eprintln!(
                "payload: {}",
                res.body_string().await.unwrap_or_else(|_| "".into())
            );
        }
        Ok(())
    }
}
