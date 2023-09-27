use crate::{client_args::ClientArgs, commands::OroCommand};
use async_trait::async_trait;
use clap::Args;
use directories::ProjectDirs;
use kdl::KdlDocument;
use miette::{IntoDiagnostic, Result};
use oro_client::OroClientBuilder;
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

    #[command(flatten)]
    client_args: ClientArgs,
}

#[async_trait]
impl OroCommand for LogoutCmd {
    async fn execute(self) -> Result<()> {
        if let Some(config_path) = &self.config.or_else(|| {
            ProjectDirs::from("", "", "orogene")
                .map(|config| config.config_dir().to_path_buf().join("oro.kdl"))
        }) {
            let builder: OroClientBuilder = self.client_args.try_into()?;
            let client = builder.registry(self.registry.clone()).build();
            std::fs::create_dir_all(config_path.parent().expect("must have parent"))
                .into_diagnostic()?;
            let mut config: KdlDocument = std::fs::read_to_string(config_path)
                .into_diagnostic()?
                .parse()?;

            if let Some(Credentials::Token(token)) =
                config::get_credentials_by_uri(&self.registry, &config)
            {
                client.delete_token(&token).await.into_diagnostic()?;
            }

            config::clear_crendentials_by_uri(&self.registry, &mut config);
            std::fs::write(config_path, config.to_string()).into_diagnostic()?;
        }
        Ok(())
    }
}
