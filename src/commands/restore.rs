use std::path::{Path, PathBuf};

use async_trait::async_trait;
use clap::Args;
use indicatif::ProgressStyle;
use miette::{IntoDiagnostic, Result};
use node_maintainer::{NodeMaintainer, NodeMaintainerOptions};
use oro_config::OroConfigLayer;
use tracing::Span;
use tracing_indicatif::span_ext::IndicatifSpanExt;
use url::Url;

use crate::commands::OroCommand;

#[derive(Debug, Args, OroConfigLayer)]
pub struct RestoreCmd {
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

    /// Prefer copying files over hard linking them.
    ///
    /// On filesystems that don't support copy-on-write/reflinks (usually NTFS
    /// or ext4), orogene defaults to hard linking package files from a
    /// centralized cache. As such, this can cause global effects if a file
    /// inside a node_modules is modified, where other projects that have
    /// installed that same file will see those modifications.
    ///
    /// In order to prevent this, you can use this flag to force orogene to
    /// always copy files, at a performance cost.
    #[arg(short, long)]
    prefer_copy: bool,

    /// Validate the integrity of installed files.
    ///
    /// When this is true, orogene will verify all files extracted from the
    /// cache, as well as verify that any files in the existing `node_modules`
    /// are unmodified. If verification fails, the packages will be
    /// reinstalled.
    #[arg(short, long)]
    validate: bool,

    /// Whether to skip restoring packages into `node_modules` and just
    /// resolve the tree and write the lockfile.
    #[arg(long)]
    lockfile_only: bool,
}

#[async_trait]
impl OroCommand for RestoreCmd {
    async fn execute(self) -> Result<()> {
        let total_time = std::time::Instant::now();
        let root = self
            .root
            .as_deref()
            .expect("root should've been set by global defaults");
        let mut nm = NodeMaintainerOptions::new();
        nm = nm
            .root(root)
            .prefer_copy(self.prefer_copy)
            .validate(self.validate)
            .on_resolution_added(move || {
                Span::current().pb_inc_length(1);
            })
            .on_resolve_progress(move |pkg| {
                let span = Span::current();
                span.pb_inc(1);
                span.pb_set_message(&format!("{:?}", pkg.resolved()));
            })
            .on_prune_progress(move |path| {
                let span = Span::current();
                span.pb_inc(1);
                span.pb_set_message(&format!("{}", path.display()));
            })
            .on_extract_progress(move |pkg| {
                let span = Span::current();
                span.pb_inc(1);
                span.pb_set_message(&format!("{:?}", pkg.resolved()));
            });
        if let Some(registry) = self.registry.as_ref() {
            nm = nm.registry(registry.clone());
        }
        if let Some(cache) = self.cache.as_deref() {
            nm = nm.cache(cache);
        }

        let resolved_nm = self.resolve(root, nm).await?;

        if !self.lockfile_only {
            self.prune(&resolved_nm).await?;
            self.extract(&resolved_nm).await?;
        } else if !self.quiet {
            eprintln!("📦 Skipping prune and extract, only writing lockfile");
        }

        resolved_nm
            .write_lockfile(root.join("package-lock.kdl"))
            .await?;

        if !self.quiet {
            eprintln!("📝 Wrote lockfile to package-lock.kdl.");
        }

        if !self.quiet {
            eprintln!("🎉 Done in {}ms.", total_time.elapsed().as_micros() / 1000,);
        }
        Ok(())
    }
}

impl RestoreCmd {
    async fn resolve(&self, root: &Path, builder: NodeMaintainerOptions) -> Result<NodeMaintainer> {
        // Set up progress bar and timing stuff.
        let resolve_time = std::time::Instant::now();
        let resolve_span = tracing::info_span!("resolving");
        resolve_span.pb_set_style(
            &ProgressStyle::default_bar()
                .template("🔍 {bar:40} [{pos}/{len}] {wide_msg:.dim}")
                .unwrap(),
        );
        resolve_span.pb_set_length(0);
        let resolve_span_enter = resolve_span.enter();

        // Actually do a resolve.
        let resolved_nm = builder
            .resolve_spec(root.canonicalize().into_diagnostic()?.to_string_lossy())
            .await?;

        // Wrap up progress bar and print messages.
        std::mem::drop(resolve_span_enter);
        std::mem::drop(resolve_span);
        if !self.quiet {
            eprintln!(
                "🔍 Resolved {} packages in {}ms.",
                resolved_nm.package_count(),
                resolve_time.elapsed().as_micros() / 1000
            );
        }

        Ok(resolved_nm)
    }

    async fn prune(&self, maintainer: &NodeMaintainer) -> Result<()> {
        // Set up progress bar and timing stuff.
        let prune_time = std::time::Instant::now();
        let prune_span = tracing::info_span!("prune");
        prune_span.pb_set_style(
            &ProgressStyle::default_bar()
                .template("🧹 {bar:40} [{pos}] {wide_msg:.dim}")
                .unwrap(),
        );
        prune_span.pb_set_length(maintainer.package_count() as u64);
        let prune_span_enter = prune_span.enter();

        // Actually do the pruning.
        let pruned = maintainer.prune().await?;

        // Wrap up progress bar and message.
        std::mem::drop(prune_span_enter);
        std::mem::drop(prune_span);
        if !self.quiet {
            eprintln!(
                "🧹 Pruned {pruned} packages in {}ms.",
                prune_time.elapsed().as_micros() / 1000
            );
        }

        Ok(())
    }

    async fn extract(&self, maintainer: &NodeMaintainer) -> Result<()> {
        // Set up progress bar and timing stuff.
        let extract_time = std::time::Instant::now();
        let extract_span = tracing::info_span!("extract");
        extract_span.pb_set_style(
            &ProgressStyle::default_bar()
                .template("📦 {bar:40} [{pos}/{len}] {wide_msg:.dim}")
                .unwrap(),
        );
        extract_span.pb_set_length(maintainer.package_count() as u64);
        let extract_span_enter = extract_span.enter();

        // Actually do the extraction.
        let extracted = maintainer.extract().await?;

        // Wrap up progress bar and message.
        std::mem::drop(extract_span_enter);
        std::mem::drop(extract_span);
        if !self.quiet {
            eprintln!(
                "📦 Extracted {extracted} package{} in {}ms.",
                if extracted == 1 { "" } else { "s" },
                extract_time.elapsed().as_micros() / 1000
            );
        }

        Ok(())
    }
}
