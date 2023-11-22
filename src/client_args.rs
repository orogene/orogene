use std::path::PathBuf;

use clap::Args;
use oro_client::{OroClientBuilder, OroClientError};
use oro_npm_account::config::Credentials;
use url::Url;

use crate::{apply_args::ApplyArgs, nassun_args::NassunArgs};

#[derive(Debug, Args)]
pub struct ClientArgs {
    #[arg(from_global)]
    pub registry: Url,

    #[arg(from_global)]
    pub cache: Option<PathBuf>,

    #[arg(from_global)]
    pub proxy: bool,

    #[arg(from_global)]
    pub proxy_url: Option<String>,

    #[arg(from_global)]
    pub no_proxy_domain: Option<String>,

    #[arg(from_global)]
    pub retries: u32,

    #[arg(from_global)]
    pub auth: Vec<(String, String, String)>,
}

impl From<ApplyArgs> for ClientArgs {
    fn from(value: ApplyArgs) -> Self {
        Self {
            registry: value.registry,
            cache: value.cache,
            proxy: value.proxy,
            proxy_url: value.proxy_url,
            no_proxy_domain: value.no_proxy_domain,
            retries: value.retries,
            auth: value.auth,
        }
    }
}

impl From<NassunArgs> for ClientArgs {
    fn from(value: NassunArgs) -> Self {
        Self {
            registry: value.registry,
            cache: value.cache,
            proxy: value.proxy,
            proxy_url: value.proxy_url,
            no_proxy_domain: value.no_proxy_domain,
            retries: value.retries,
            auth: value.auth,
        }
    }
}

impl ClientArgs {
    pub(crate) fn into_client_builder(
        self,
        config: Option<&Credentials>,
    ) -> Result<OroClientBuilder, OroClientError> {
        let mut builder = OroClientBuilder::new()
            .registry(self.registry.clone())
            .retries(self.retries)
            .proxy(self.proxy);
        if let Some(cache) = self.cache {
            builder = builder.cache(cache);
        }
        if let Some(domain) = self.no_proxy_domain {
            builder = builder.no_proxy_domain(domain)
        }
        if let Some(url) = self.proxy_url {
            builder = builder.proxy_url(url)?;
        }
        if let Some(config) = config {
            builder = match config {
                Credentials::Token(token) => {
                    builder.token_auth(self.registry.clone(), token.to_owned())
                }
                Credentials::LegacyAuth(legacy_auth) => {
                    builder.legacy_auth(self.registry.clone(), legacy_auth.to_owned())
                }
                Credentials::BasicAuth { username, password } => builder.basic_auth(
                    self.registry.clone(),
                    username.to_owned(),
                    password.to_owned(),
                ),
            }
        }
        for (reg, key, val) in &self.auth {
            let url = Url::parse(reg)?;
            if key == "token" {
                builder = builder.token_auth(url, val.into());
            } else if key == "username" {
                let mut password = None;
                for (reg2, key2, val2) in &self.auth {
                    if reg2 == reg && key2 == "password" {
                        password = Some(val2.to_owned());
                        break;
                    }
                }
                builder = builder.basic_auth(url, val.into(), password);
            } else if key == "legacy-auth" {
                builder = builder.legacy_auth(url, val.into());
            } else if key == "password" {
            } else {
                tracing::warn!("Invalid authentication configuration for {reg}: {key} {val}");
            }
        }
        Ok(builder)
    }
}

impl TryFrom<ClientArgs> for OroClientBuilder {
    type Error = OroClientError;
    fn try_from(value: ClientArgs) -> Result<Self, Self::Error> {
        value.into_client_builder(None)
    }
}
