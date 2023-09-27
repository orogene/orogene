use std::path::PathBuf;

use clap::Args;
use miette::Result;
use nassun::{Nassun, NassunOpts};
use oro_client::OroClientBuilder;
use url::Url;

use crate::{apply_args::ApplyArgs, client_args::ClientArgs};

#[derive(Clone, Debug, Args)]
pub struct NassunArgs {
    /// Default dist-tag to use when resolving package versions.
    #[arg(long, default_value = "latest")]
    pub default_tag: String,

    #[arg(from_global)]
    pub registry: Url,

    #[arg(from_global)]
    pub scoped_registries: Vec<(String, Url)>,

    #[arg(from_global)]
    pub root: PathBuf,

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

impl NassunArgs {
    pub fn from_apply_args(apply_args: &ApplyArgs) -> Self {
        Self {
            default_tag: apply_args.default_tag.clone(),
            registry: apply_args.registry.clone(),
            scoped_registries: apply_args.scoped_registries.clone(),
            root: apply_args.root.clone(),
            cache: apply_args.cache.clone(),
            proxy: apply_args.proxy,
            proxy_url: apply_args.proxy_url.clone(),
            no_proxy_domain: apply_args.no_proxy_domain.clone(),
            retries: apply_args.retries,
            auth: apply_args.auth.clone(),
        }
    }

    pub fn to_nassun(&self) -> Result<Nassun> {
        let client_args: ClientArgs = ((*self).clone()).into();
        let client_builder: OroClientBuilder = client_args.try_into()?;
        let mut nassun_opts = NassunOpts::new()
            .registry(self.registry.clone())
            .base_dir(self.root.clone())
            .default_tag(&self.default_tag)
            .client(client_builder.build());
        for (scope, registry) in &self.scoped_registries {
            nassun_opts = nassun_opts.scope_registry(scope.clone(), registry.clone());
        }
        if let Some(cache) = &self.cache {
            nassun_opts = nassun_opts.cache(cache.clone());
        }
        Ok(nassun_opts.build())
    }
}
