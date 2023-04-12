use std::path::PathBuf;

use clap::Args;
use nassun::{Nassun, NassunOpts};
use url::Url;

use crate::apply_args::ApplyArgs;

#[derive(Debug, Args)]
pub struct NassunArgs {
    /// Default dist-tag to use when resolving package versions.
    #[arg(long, default_value = "latest")]
    default_tag: String,

    #[arg(from_global)]
    registry: Url,

    #[arg(from_global)]
    scoped_registries: Vec<(String, Url)>,

    #[arg(from_global)]
    root: PathBuf,

    #[arg(from_global)]
    cache: Option<PathBuf>,
}

impl NassunArgs {
    pub fn from_apply_args(apply_args: &ApplyArgs) -> Self {
        Self {
            default_tag: apply_args.default_tag.clone(),
            registry: apply_args.registry.clone(),
            scoped_registries: apply_args.scoped_registries.clone(),
            root: apply_args.root.clone(),
            cache: apply_args.cache.clone(),
        }
    }

    pub fn to_nassun(&self) -> Nassun {
        let mut nassun_opts = NassunOpts::new()
            .registry(self.registry.clone())
            .base_dir(self.root.clone())
            .default_tag(&self.default_tag);
        for (scope, registry) in &self.scoped_registries {
            nassun_opts = nassun_opts.scope_registry(scope.clone(), registry.clone());
        }
        if let Some(cache) = &self.cache {
            nassun_opts = nassun_opts.cache(cache.clone());
        }
        nassun_opts.build()
    }
}
