use std::{fs, path::PathBuf};

use async_trait::async_trait;
use clap::Args;
use indicatif::ProgressStyle;
use miette::{Context, IntoDiagnostic, Result};
use node_maintainer::NodeMaintainerOptions;
use oro_config::OroConfigLayer;
use tracing::Span;
use tracing_indicatif::span_ext::IndicatifSpanExt;
use url::Url;

use crate::commands::OroCommand;

#[derive(Debug, Args, OroConfigLayer)]
pub struct ResolveCmd {
    #[clap(from_global)]
    registry: Option<Url>,

    #[clap(from_global)]
    json: bool,

    #[clap(from_global)]
    quiet: bool,

    #[clap(from_global)]
    root: Option<PathBuf>,

    #[clap(from_global)]
    cache: Option<PathBuf>,
}

#[async_trait]
impl OroCommand for ResolveCmd {
    async fn execute(self) -> Result<()> {
        let start = std::time::Instant::now();
        let root = self
            .root
            .expect("root should've been set by global defaults");
        let mut nm = NodeMaintainerOptions::new()
            .on_resolution_added(move || {
                Span::current().pb_inc_length(1);
            })
            .on_resolve_progress(move |pkg| {
                let span = Span::current();
                span.pb_inc(1);
                span.pb_set_message(&format!("{:?}", pkg.resolved()));
            });
        if let Some(registry) = self.registry {
            nm = nm.registry(registry);
        }
        if let Some(cache) = self.cache {
            nm = nm.cache(cache);
        }

        let lock_path = root.join("package-lock.kdl");
        if lock_path.exists() {
            let kdl = fs::read_to_string(&lock_path)
                .into_diagnostic()
                .wrap_err_with(|| {
                    format!("Failed to read lockfile at {}", lock_path.to_string_lossy())
                })?;
            nm = nm.kdl_lock(kdl).wrap_err_with(|| {
                format!(
                    "Failed to parse lockfile at {}",
                    lock_path.to_string_lossy()
                )
            })?;
        }

        let lock_path = root.join("package-lock.json");
        if lock_path.exists() {
            let json = fs::read_to_string(&lock_path)
                .into_diagnostic()
                .wrap_err_with(|| {
                    format!("Failed to read lockfile at {}", lock_path.to_string_lossy())
                })?;
            nm = nm.npm_lock(json).wrap_err_with(|| {
                format!(
                    "Failed to parse NPM package lockfile at {}",
                    lock_path.to_string_lossy()
                )
            })?;
        }

        let resolve_span = tracing::info_span!("resolving");
        resolve_span.pb_set_style(
            &ProgressStyle::default_bar()
                .template("🔍 {bar:40} [{pos}/{len}] {wide_msg:.dim}")
                .unwrap(),
        );
        resolve_span.pb_set_length(0);
        let resolve_span_enter = resolve_span.enter();
        let resolved_nm = nm
            .resolve_spec(root.canonicalize().into_diagnostic()?.to_string_lossy())
            .await?;

        std::mem::drop(resolve_span_enter);
        std::mem::drop(resolve_span);
        if !self.quiet {
            eprintln!("🔍 Resolved {} packages.", resolved_nm.package_count(),);
        }
        resolved_nm
            .write_lockfile(root.join("package-lock.kdl"))
            .await?;
        if !self.quiet {
            eprintln!("📦 Wrote lockfile to package-lock.kdl");
        }
        if !self.quiet {
            eprintln!("🎉 Done in {}ms.", start.elapsed().as_micros() / 1000,);
        }
        Ok(())
    }
}
