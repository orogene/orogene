use std::io::{self, Write};
use std::time::Instant;

use anyhow::{anyhow, Context, Result};
use async_trait::async_trait;
use clap::Clap;
use oro_client::OroClient;
use oro_command::{ArgMatches, OroCommand, OroConfig};
use oro_error_code::OroErrCode as Code;
use serde::Deserialize;
use serde_json::Value;
use url::Url;

#[derive(Debug, Clap)]
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
    fn layer_config(&mut self, args: ArgMatches, config: OroConfig) -> Result<()> {
        if args.occurrences_of("registry") == 0 {
            if let Ok(reg) = config.get_str("registry") {
                self.registry = Url::parse(&reg).with_context(|| Code::OR1005(reg))?;
            }
        }
        Ok(())
    }

    async fn execute(self) -> Result<()> {
        self.ping(io::stdout(), io::stderr()).await
    }
}

impl PingCmd {
    async fn ping<O, E>(self, mut stdout: O, mut stderr: E) -> Result<()>
    where
        O: Write,
        E: Write,
    {
        if !self.json {
            writeln!(stderr, "PING: {}", self.registry)?;
        }
        let start = Instant::now();
        // This silliness is due to silliness in Surf that should be addressed
        // soon. Once it's fixed, this line will just be a nice .await? See:
        // https://github.com/dtolnay/anyhow/issues/35#issuecomment-547986739
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

#[cfg(test)]
mod tests {
    use super::*;

    use anyhow::Result;
    use async_std;
    use mockito::mock;
    use serde_json::json;

    #[async_std::test]
    async fn basic() -> Result<()> {
        let m = mock("GET", "/-/ping?write=true")
            .with_status(200)
            .with_body("hello, world!")
            .create();
        let registry = &mockito::server_url();
        let mut stdout: Vec<u8> = Vec::new();
        let mut stderr: Vec<u8> = Vec::new();
        let cmd = PingCmd {
            registry: Url::parse(registry)?,
            json: false,
        };
        cmd.ping(&mut stdout, &mut stderr).await?;
        m.assert();
        assert_eq!(String::from_utf8(stdout)?, "");
        let stderr = String::from_utf8(stderr)?;
        assert!(stderr.contains(&format!("PING: {}", registry)));
        assert!(stderr.contains("PONG:"));
        assert!(stderr.contains("hello, world!"));
        Ok(())
    }

    #[async_std::test]
    async fn json() -> Result<()> {
        let m = mock("GET", "/-/ping?write=true")
            .with_status(200)
            .with_body(r#"{"message": "hello, world!"}"#)
            .create();
        let registry = &mockito::server_url();
        let mut stdout: Vec<u8> = Vec::new();
        let mut stderr: Vec<u8> = Vec::new();
        let cmd = PingCmd {
            registry: Url::parse(registry)?,
            json: true,
        };

        cmd.ping(&mut stdout, &mut stderr).await?;
        m.assert();

        let stdout = String::from_utf8(stdout)?;
        assert!(stdout.contains(r#""message": "hello, world!""#));
        let mut parsed = serde_json::from_str::<Value>(&stdout)?;
        assert!(parsed["time"].take().is_number());
        assert_eq!(
            parsed,
            json!({
                "registry": Url::parse(registry)?.to_string(),
                "details": {
                    "message": "hello, world!"
                },
                "time": null,
            })
        );

        let stderr = String::from_utf8(stderr).unwrap();
        assert!(stderr.contains(&format!("PING: {}", registry)));
        assert!(stderr.contains("PONG:"));

        Ok(())
    }
}
