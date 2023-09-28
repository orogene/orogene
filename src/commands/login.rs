use crate::client_args::ClientArgs;
use crate::commands::OroCommand;
use async_trait::async_trait;
use clap::{clap_derive::ValueEnum, Args};
use directories::ProjectDirs;
use kdl::KdlDocument;
use miette::{IntoDiagnostic, Result};
use oro_client::login::{AuthType, LoginOptions};
use oro_client::OroClientBuilder;
use oro_npm_account::config::{self, Credentials};
use oro_npm_account::login::login;
use std::path::PathBuf;
use url::Url;

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
enum LoginType {
    Web,
    Legacy,
}

impl From<LoginType> for AuthType {
    fn from(login_type: LoginType) -> Self {
        match login_type {
            LoginType::Web => AuthType::Web,
            LoginType::Legacy => AuthType::Legacy,
        }
    }
}

/// Log in to the registry.
#[derive(Debug, Args)]
pub struct LoginCmd {
    #[arg(from_global)]
    registry: Url,

    #[arg(from_global)]
    config: Option<PathBuf>,

    /// What authentication strategy to use with login.
    #[arg(long, value_enum, default_value_t = LoginType::Web)]
    auth_type: LoginType,

    /// Set an authorization token directly (the equivalent of NPM's `:token` or `:_authToken`).
    #[arg(long)]
    token: Option<String>,

    /// Set a username directly instead of logging in to a registry.
    #[arg(long)]
    username: Option<String>,

    /// If a `username` is provided, this (optional) password will be set
    /// along with it.
    #[arg(long)]
    password: Option<String>,

    /// Set a legacy authorization token (the equivalent of NPM's `:_auth`).
    #[arg(long)]
    legacy_token: Option<String>,

    /// Associate an operation with a scope for a scoped registry.
    #[arg(long)]
    scope: Option<String>,

    #[command(flatten)]
    client_args: ClientArgs,
}

#[async_trait]
impl OroCommand for LoginCmd {
    async fn execute(self) -> Result<()> {
        if let Some(config_path) = &self.config.or_else(|| {
            ProjectDirs::from("", "", "orogene")
                .map(|config| config.config_dir().to_path_buf().join("oro.kdl"))
        }) {
            std::fs::create_dir_all(config_path.parent().expect("must have parent")).unwrap();
            let mut config: KdlDocument = std::fs::read_to_string(config_path)
                .into_diagnostic()?
                .parse()?;

            tracing::info!("Logging in to {}", self.registry);

            let builder: OroClientBuilder = self.client_args.try_into()?;

            let credentials = if let Some(token) = &self.token {
                Credentials::Token(token.clone())
            } else if let Some(username) = &self.username {
                Credentials::BasicAuth {
                    username: username.clone(),
                    password: self.password.clone(),
                }
            } else if let Some(legacy_token) = &self.legacy_token {
                Credentials::LegacyAuth(legacy_token.clone())
            } else {
                let token = login(
                    &self.auth_type.into(),
                    &self.registry,
                    &LoginOptions {
                        scope: self.scope.clone(),
                        client: Some(builder.registry(self.registry.clone()).build()),
                    },
                )
                .await
                .into_diagnostic()?;

                Credentials::Token(token.token)
            };

            config::set_credentials_by_uri(&self.registry, &credentials, &mut config);

            if let Some(scope) = self.scope {
                config::set_scoped_registry(&scope, &self.registry, &mut config);
            }

            std::fs::write(config_path, config.to_string()).into_diagnostic()?;
        }
        Ok(())
    }
}
