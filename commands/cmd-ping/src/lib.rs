use std::time::Instant;

use anyhow::{anyhow, Context, Result};
use async_trait::async_trait;
use clap::Clap;
use oro_client::OroClient;
use oro_command::OroCommand;
use oro_error_code::OroErrCode as Code;
use serde::Deserialize;
use serde_json::Value;
use url::Url;

#[derive(Debug, Clap, OroCommand)]
pub struct PingCmd {
    #[clap(
        about = "Registry to ping.",
        default_value = "https://registry.npmjs.org"
    )]
    registry: Url,
    #[clap(from_global)]
    loglevel: log::LevelFilter,
    #[clap(from_global)]
    json: bool,
    #[clap(from_global)]
    quiet: bool,
}

#[derive(Debug, Deserialize)]
struct NpmError {
    message: String,
}

#[async_trait]
impl OroCommand for PingCmd {
    async fn execute(self) -> Result<()> {
        let quiet = self.loglevel == log::LevelFilter::Off || self.quiet;
        if !quiet && !self.json {
            eprintln!("ping: {}", self.registry);
        }
        let start = Instant::now();
        let mut res = OroClient::new(self.registry.clone())
            .get("-/ping?write=true")
            .await
            .with_context(|| Code::OR1001(self.registry.to_string()))?;
        if res.status().is_client_error() || res.status().is_server_error() {
            let msg = match res.body_json::<NpmError>().await {
                Ok(err) => err.message,
                parse_err @ Err(_) => match res.body_string().await {
                    Ok(msg) => msg,
                    body_err @ Err(_) => {
                        return Err(anyhow!("{}", Code::OR1002))
                            .with_context(|| format!("{:?}", parse_err))
                            .with_context(|| format!("{:?}", body_err))
                    }
                },
            };
            return Err(anyhow!(
                "{}",
                Code::OR1003 {
                    registry: self.registry.to_string(),
                    message: msg,
                }
            ));
        }

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
            eprintln!("payload: {}", res.body_string().await.unwrap_or_else(|_| "".into()));
        }
        Ok(())
    }
}
