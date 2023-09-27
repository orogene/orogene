use crate::commands::OroCommand;
use async_trait::async_trait;
use clap::Args;
use directories::ProjectDirs;
use kdl::KdlDocument;
use miette::{IntoDiagnostic, Result};
use oro_client::login::{AuthType, LoginOptions};
use oro_npm_account::config::{self, Credentials};
use oro_npm_account::login::login;
use std::path::PathBuf;
use url::Url;

/// Log in to the registry.
#[derive(Debug, Args)]
pub struct LoginCmd {
    #[arg(from_global)]
    registry: Url,

    #[arg(from_global)]
    config: Option<PathBuf>,

    /// What authentication strategy to use with login.
    #[arg(long, value_enum, default_value_t = AuthType::Web)]
    auth_type: AuthType,

    /// Associate an operation with a scope for a scoped registry.
    #[arg(long)]
    scope: Option<String>,
}

#[async_trait]
impl OroCommand for LoginCmd {
    async fn execute(self) -> Result<()> {
        if let Some(config_dir) = &self
            .config
            .map(|config_path| {
                config_path
                    .parent()
                    .expect("must have a parent")
                    .to_path_buf()
            })
            .or(ProjectDirs::from("", "", "orogene")
                .map(|config| config.config_dir().to_path_buf()))
        {
            let registry = self.registry.to_string();
            if !config_dir.exists() {
                std::fs::create_dir_all(config_dir).unwrap();
            }
            let config_path = config_dir.join("oro.kdl");
            let mut config: KdlDocument = std::fs::read_to_string(&config_path)
                .into_diagnostic()?
                .parse()?;

            tracing::info!("Login in on {}", &registry);

            let token = login(
                &self.auth_type,
                &self.registry,
                &LoginOptions {
                    scope: self.scope.clone(),
                },
            )
            .await
            .into_diagnostic()?;

            tracing::info!("Logged in on {}", &registry);

            config::set_credentials_by_uri(
                &registry,
                &Credentials::AuthToken(token.token),
                &mut config,
            );

            if let Some(scope) = self.scope {
                config::set_scoped_registry(&scope, &registry, &mut config);
            }

            std::fs::write(&config_path, config.to_string()).into_diagnostic()?;
        }
        Ok(())
    }
}
