use std::time::{Duration, Instant};

use oro_api_client::{ApiClient, AsyncReadResponseExt, Request};
use oro_command::{
    clap::{self, Clap},
    indicatif::ProgressBar,
    oro_config::OroConfigLayer,
    OroCommand,
};
use oro_common::{
    async_trait::async_trait,
    miette::{Context, IntoDiagnostic, Result},
    serde_json::{self, Value},
    smol::{self, Timer},
    url::Url,
};

#[derive(Debug, Clap, OroConfigLayer)]
#[config_layer = "ping"]
pub struct PingCmd {
    #[clap(
        about = "Registry to ping.",
        default_value = "https://registry.npmjs.org"
    )]
    registry: Url,
    #[clap(from_global)]
    json: bool,
    #[clap(from_global)]
    quiet: bool,
}

#[async_trait]
impl OroCommand for PingCmd {
    async fn execute(self) -> Result<()> {
        let start = Instant::now();
        let spinner = if self.quiet || self.json {
            ProgressBar::hidden()
        } else {
            ProgressBar::new_spinner()
        };
        spinner.println(format!("ping: {}", self.registry));
        let spin_clone = spinner.clone();
        let fut = smol::spawn(async move {
            while !spin_clone.is_finished() {
                spin_clone.tick();
                Timer::after(Duration::from_millis(20)).await;
            }
        });
        let client = ApiClient::new();
        let req = Request::get(self.registry.join("-/ping?write=true").unwrap().to_string())
            .body(())
            .expect("Failed to create request");
        let mut res = client
            .send(req)
            .await
            .into_diagnostic()
            .context("Ping failed")?;
        let time = start.elapsed().as_micros() as f32 / 1000.0;
        if !self.quiet && !self.json {
            spinner.println(format!("pong: {}ms", time));
        }
        spinner.finish();
        if self.json {
            let details: Value =
                serde_json::from_slice(&res.bytes().await.unwrap_or_else(|_| "{}".into()))
                    .into_diagnostic()
                    .context("Failed to deserialize JSON from registry")?;
            let output = serde_json::to_string_pretty(&serde_json::json!({
                "registry": self.registry.to_string(),
                "time": time,
                "details": details,
            }))
            .into_diagnostic()
            .context("Failed to serialize JSON ping output.")?;
            println!("{}", output);
        } else if !self.quiet {
        }
        fut.await;
        Ok(())
    }
}
