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

            config::set_credentials_by_uri(
                &self.registry,
                &Credentials::Token(token.token),
                &mut config,
            );

            if let Some(scope) = self.scope {
                config::set_scoped_registry(&scope, &self.registry, &mut config);
            }

            std::fs::write(config_path, config.to_string()).into_diagnostic()?;
        }
        Ok(())
    }
}
