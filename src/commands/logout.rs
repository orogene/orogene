use crate::commands::OroCommand;
use async_trait::async_trait;
use clap::Args;
use directories::ProjectDirs;
use kdl::KdlDocument;
use miette::{IntoDiagnostic, Result};
use oro_client::OroClient;
use oro_npm_account::config::{self, Credentials};
use std::path::PathBuf;
use url::Url;

/// Logout from the registry.
#[derive(Debug, Args)]
pub struct LogoutCmd {
    #[arg(from_global)]
    registry: Url,

    #[arg(from_global)]
    config: Option<PathBuf>,
}

#[async_trait]
impl OroCommand for LogoutCmd {
    async fn execute(self) -> Result<()> {
        if let Some(config_dir) = &self
            .config
            .or(ProjectDirs::from("", "", "orogene").map(|config| config.config_dir().into()))
        {
            let client = OroClient::new(self.registry.clone());
            let registry = self.registry.to_string();
            if !config_dir.exists() {
                std::fs::create_dir_all(config_dir).unwrap();
            }
            let config_path = config_dir.join("oro.kdl");
            let mut config: KdlDocument = std::fs::read_to_string(&config_path)
                .unwrap_or_default()
                .parse()?;

            match config::get_credentials_by_uri(&registry, &config) {
                Some(Credentials::AuthToken(token)) => {
                    client.delete_token(&token).await.into_diagnostic()?;
                }
                _ => {
                    tracing::error!("Not logged in to {registry}, so can't log out!");
                }
            }

            config::clear_crendentials_by_uri(&registry, &mut config);
            std::fs::write(&config_path, config.to_string()).into_diagnostic()?;
        }
        Ok(())
    }
}
