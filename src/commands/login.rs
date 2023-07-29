use crate::commands::OroCommand;
use async_trait::async_trait;
use clap::Args;
use directories::ProjectDirs;
use kdl::KdlDocument;
use miette::{IntoDiagnostic, Result};
use oro_client::login::AuthType;
use oro_npm_account::config::{self, Credentials};
use oro_npm_account::login::login;
use url::Url;

/// Log in to the registry.
#[derive(Debug, Args)]
pub struct LoginCmd {
    #[arg(from_global)]
    registry: Url,

    /// What authentication strategy to use with login.
    #[arg(long, value_enum, default_value_t = AuthType::Web)]
    auth_type: AuthType,
}

#[async_trait]
impl OroCommand for LoginCmd {
    async fn execute(self) -> Result<()> {
        if let Some(dirs) = ProjectDirs::from("", "", "orogene") {
            let registry = self.registry.to_string();
            let config_dir = dirs.config_dir();
            if !config_dir.exists() {
                std::fs::create_dir_all(config_dir).unwrap();
            }
            let config_path = config_dir.join("oro.kdl");
            let mut config: KdlDocument = std::fs::read_to_string(&config_path)
                .unwrap_or_default()
                .parse()?;

            tracing::info!("Login in on {}", &registry);

            let token = login(&self.auth_type, &self.registry)
                .await
                .into_diagnostic()?;

            tracing::info!("Logged in on {}", &registry);

            config::set_credentials_by_uri(
                &registry,
                &Credentials::AuthToken(token.token),
                &mut config,
            );

            std::fs::write(&config_path, config.to_string()).into_diagnostic()?;
        }
        Ok(())
    }
}
