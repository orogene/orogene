use std::io::{self, Write};
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
    #[clap(long, about = "Format output as JSON.")]
    json: bool,
}

#[derive(Debug, Deserialize)]
struct NpmError {
    message: String,
}

#[async_trait]
impl OroCommand for PingCmd {
    async fn execute(self) -> Result<()> {
        let (mut stdout, mut stderr) = (io::stdout(), io::stderr());
        if !self.json {
            writeln!(stderr, "PING: {}", self.registry)?;
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
                    message: msg.clone()
                }
            ));
        }

        let time = start.elapsed().as_micros() as f32 / 1000.0;
        if !self.json {
            writeln!(stderr, "PONG: {}ms", time)?;
        }
        if self.json {
            let details: Value =
                serde_json::from_str(&res.body_string().await.unwrap_or("{}".into()))
                    .context(Code::OR1004)?;
            writeln!(
                stdout,
                "{}",
                serde_json::to_string_pretty(&serde_json::json!({
                    "registry": self.registry.to_string(),
                    "time": time,
                    "details": details,
                }))?
            )?;
        } else {
            writeln!(
                stderr,
                "PONG: {}",
                res.body_string().await.unwrap_or("".into())
            )?;
        }
        Ok(())
    }
}
