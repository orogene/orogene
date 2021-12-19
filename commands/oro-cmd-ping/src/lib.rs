use std::time::{Duration, Instant};

use nuget_api::v3::NuGetClient;
use turron_command::{
    async_trait::async_trait,
    clap::{self, Clap},
    indicatif::ProgressBar,
    turron_config::TurronConfigLayer,
    TurronCommand,
};
use turron_common::{
    miette::{Context, IntoDiagnostic, Result},
    serde_json::{self, json},
    smol::{self, Timer},
};

#[derive(Debug, Clap, TurronConfigLayer)]
#[config_layer = "ping"]
pub struct PingCmd {
    #[clap(
        about = "Source to ping",
        default_value = "https://api.nuget.org/v3/index.json",
        long
    )]
    source: String,
    #[clap(from_global)]
    quiet: bool,
    #[clap(from_global)]
    json: bool,
}

#[async_trait]
impl TurronCommand for PingCmd {
    async fn execute(self) -> Result<()> {
        let start = Instant::now();
        let spinner = if self.quiet || self.json {
            ProgressBar::hidden()
        } else {
            ProgressBar::new_spinner()
        };
        spinner.println(format!("ping: {}", self.source));
        let spin_clone = spinner.clone();
        let fut = smol::spawn(async move {
            while !spin_clone.is_finished() {
                spin_clone.tick();
                Timer::after(Duration::from_millis(20)).await;
            }
        });
        let client = NuGetClient::from_source(self.source.clone()).await?;
        let time = start.elapsed().as_micros() as f32 / 1000.0;
        if !self.quiet && self.json {
            let output = serde_json::to_string_pretty(&json!({
                "source": self.source.to_string(),
                "time": time,
                "endpoints": client.endpoints,
            }))
            .into_diagnostic()
            .context("Failed to serialize JSON ping output.")?;
            println!("{}", output);
        }
        spinner.println(format!("pong: {}ms", time));
        spinner.finish();
        fut.await;
        Ok(())
    }
}
