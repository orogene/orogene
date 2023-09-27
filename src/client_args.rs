use std::path::PathBuf;

use clap::Args;
use oro_client::{OroClientBuilder, OroClientError};
use url::Url;

use crate::{apply_args::ApplyArgs, nassun_args::NassunArgs};

#[derive(Debug, Args)]
pub struct ClientArgs {
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
            cache: value.cache,
            proxy: value.proxy,
            proxy_url: value.proxy_url,
            no_proxy_domain: value.no_proxy_domain,
            retries: value.retries,
            auth: value.auth,
        }
    }
}

impl TryFrom<ClientArgs> for OroClientBuilder {
    type Error = OroClientError;
    fn try_from(value: ClientArgs) -> Result<Self, Self::Error> {
        let mut builder = OroClientBuilder::new()
            .retries(value.retries)
            .proxy(value.proxy);
        if let Some(cache) = value.cache {
            builder = builder.cache(cache);
        }
        if let Some(domain) = value.no_proxy_domain {
            builder = builder.no_proxy_domain(domain)
        }
        if let Some(url) = value.proxy_url {
            builder = builder.proxy_url(url)?;
        }
        for (reg, key, val) in &value.auth {
            let url = Url::parse(reg)?;
            if key == "token" {
                builder = builder.token_auth(url, val.into());
            } else if key == "username" {
                let mut password = None;
                for (reg2, key2, val2) in &value.auth {
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
