use std::path::PathBuf;

use clap::Args;
use indicatif::ProgressStyle;
use miette::Result;
use node_maintainer::{NodeMaintainer, NodeMaintainerOptions};
use oro_common::CorgiManifest;
use rand::seq::IteratorRandom;
use tracing::{Instrument, Span};
use tracing_indicatif::span_ext::IndicatifSpanExt;
use url::Url;

/// Applies the current project's requested dependencies to `node_modules/`,
/// adding, removing, and updating dependencies as needed. This command is
/// intended to be an idempotent way to make sure your `node_modules` is in
/// the right state to execute, based on your declared dependencies.
#[derive(Debug, Args)]
#[command(next_help_heading = "Apply Options")]
pub struct ApplyArgs {
    /// Prevent all apply operations from executing.
    #[arg(
        long = "no-apply",
        action = clap::ArgAction::SetFalse,
    )]
    pub apply: bool,

    /// When extracting packages, prefer to copy files files instead of
    /// linking them.
    ///
    /// This option has no effect if hard linking fails (for example, if the
    /// cache is on a different drive), or if the project is on a filesystem
    /// that supports Copy-on-Write (zfs, btrfs, APFS (macOS), etc).
    #[arg(long)]
    pub prefer_copy: bool,

    /// Whether to skip restoring packages into `node_modules` and just
    /// resolve the tree and write the lockfile.
    #[arg(long)]
    pub lockfile_only: bool,

    /// Make the resolver error if the newly-resolved tree would defer from
    /// an existing lockfile.
    #[arg(long, visible_alias = "frozen")]
    pub locked: bool,

    /// Skip running install scripts.
    #[arg(long = "no-scripts", alias = "ignore-scripts", action = clap::ArgAction::SetFalse)]
    pub scripts: bool,

    /// Default dist-tag to use when resolving package versions.
    #[arg(long, default_value = "latest")]
    pub default_tag: String,

    /// Controls number of concurrent operations during various apply steps
    /// (resolution fetches, extractions, etc).
    ///
    /// Tuning this might help reduce memory usage (if lowered), or improve
    /// performance (if increased).
    #[arg(long, default_value_t = node_maintainer::DEFAULT_CONCURRENCY)]
    pub concurrency: usize,

    /// Controls number of concurrent script executions while running
    /// `run_script`.
    ///
    /// This option is separate from `concurrency` because executing
    /// concurrent scripts is a much heavier operation.
    #[arg(long, default_value_t = node_maintainer::DEFAULT_SCRIPT_CONCURRENCY)]
    pub script_concurrency: usize,

    /// Disable writing the lockfile after operations complete.
    ///
    /// Note that lockfiles are only written after all operations complete
    /// successfully.
    #[arg(long = "no-lockfile", action = clap::ArgAction::SetFalse)]
    pub lockfile: bool,

    /// Use the hoisted installation mode, where all dependencies and their
    /// transitive dependencies are installed as high up in the `node_modules`
    /// tree as possible.
    ///
    /// This can potentially mean that packages have access to dependencies
    /// they did not specify in their package.json, but it might be useful for
    /// compatibility.
    ///
    /// By default, dependencies are installed in "isolated" mode, using a
    /// symlink/junction structure to simulate a dependency tree.
    #[arg(long)]
    pub hoisted: bool,

    #[arg(from_global)]
    pub registry: Url,

    #[arg(from_global)]
    pub scoped_registries: Vec<(String, Url)>,

    #[arg(from_global)]
    pub json: bool,

    #[arg(from_global)]
    pub root: PathBuf,

    #[arg(from_global)]
    pub cache: Option<PathBuf>,

    #[arg(from_global)]
    pub emoji: bool,

    #[arg(from_global)]
    pub proxy: bool,

    #[arg(from_global)]
    pub proxy_url: Option<String>,

    #[arg(from_global)]
    pub no_proxy: Option<String>,

    #[arg(from_global)]
    pub fetch_retries: u32,
}

impl ApplyArgs {
    pub async fn execute(&self, manifest: CorgiManifest) -> Result<()> {
        let total_time = std::time::Instant::now();

        if !self.apply {
            tracing::info!("{}Skipping applying node_modules/.", self.emoji_tada(),);
            return Ok(());
        }

        let root = &self.root;
        let maintainer = self.resolve(manifest, self.configured_maintainer()).await?;

        if !self.lockfile_only {
            self.prune(&maintainer).await?;
            self.extract(&maintainer).await?;
            self.rebuild(&maintainer).await?;
        } else {
            tracing::info!(
                "{}Skipping installing node_modules/, only writing lockfile.",
                self.emoji_package()
            );
        }

        if self.lockfile {
            maintainer
                .write_lockfile(root.join("package-lock.kdl"))
                .await?;
            tracing::info!(
                "{}Wrote lockfile to package-lock.kdl.",
                self.emoji_writing()
            );
        }

        tracing::info!(
            "{}Applied node_modules/ in {}s. {}",
            self.emoji_tada(),
            total_time.elapsed().as_millis() as f32 / 1000.0,
            hackerish_encouragement()
        );
        Ok(())
    }

    fn configured_maintainer(&self) -> NodeMaintainerOptions {
        let root = &self.root;
        let mut nm = NodeMaintainerOptions::new();
        nm = nm
            .registry(self.registry.clone())
            .locked(self.locked)
            .default_tag(&self.default_tag)
            .concurrency(self.concurrency)
            .script_concurrency(self.script_concurrency)
            .root(root)
            .prefer_copy(self.prefer_copy)
            .hoisted(self.hoisted)
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
                span.pb_set_message(&format!("{:?}", pkg.resolved()))
            })
            .on_script_start(|pkg, event| {
                let span = Span::current();
                span.pb_set_style(
                    &ProgressStyle::default_bar()
                        .template(&format!(
                            "{{span_child_prefix}}{{spinner}} {}::{event} ({{elapsed}}): {{wide_msg:.dim}}",
                            pkg.name(),
                        ))
                        .unwrap(),
                );
            })
            .on_script_line(|line| {
                let span = Span::current();
                span.pb_inc(1);
                span.pb_set_message(line);
            });

        for (scope, registry) in &self.scoped_registries {
            nm = nm.scope_registry(scope, registry.clone());
        }

        if let Some(cache) = self.cache.as_deref() {
            nm = nm.cache(cache);
        }

        nm
    }

    async fn resolve(
        &self,
        root_manifest: CorgiManifest,
        builder: NodeMaintainerOptions,
    ) -> Result<NodeMaintainer> {
        // Set up progress bar and timing stuff.
        let resolve_time = std::time::Instant::now();
        let resolve_span = tracing::debug_span!("resolving");
        resolve_span.pb_set_style(
            &ProgressStyle::default_bar()
                .template(&format!(
                    "{}Resolving {}",
                    self.emoji_magnifying_glass(),
                    "{bar:40} [{pos}/{len}] {wide_msg:.dim}"
                ))
                .unwrap(),
        );
        resolve_span.pb_set_length(0);
        let resolve_span_enter = resolve_span.enter();

        // Actually do a resolve.
        let resolved_nm = builder.resolve_manifest(root_manifest).await?;

        // Wrap up progress bar and print messages.
        std::mem::drop(resolve_span_enter);
        std::mem::drop(resolve_span);
        tracing::info!(
            "{}Resolved {} packages in {}s.",
            self.emoji_magnifying_glass(),
            resolved_nm.package_count(),
            resolve_time.elapsed().as_millis() as f32 / 1000.0
        );

        Ok(resolved_nm)
    }

    async fn prune(&self, maintainer: &NodeMaintainer) -> Result<usize> {
        // Set up progress bar and timing stuff.
        let prune_time = std::time::Instant::now();
        let prune_span = tracing::debug_span!("prune");
        prune_span.pb_set_style(
            &ProgressStyle::default_bar()
                .template(&format!(
                    "{}Pruning extraneous {}",
                    self.emoji_broom(),
                    "{bar:40} [{pos}] {wide_msg:.dim}"
                ))
                .unwrap(),
        );
        prune_span.pb_set_length(maintainer.package_count() as u64);
        let prune_span_enter = prune_span.enter();

        // Actually do the pruning.
        let pruned = maintainer.prune().await?;

        // Wrap up progress bar and message.
        std::mem::drop(prune_span_enter);
        std::mem::drop(prune_span);
        tracing::info!(
            "{}Pruned {pruned} packages in {}s.",
            self.emoji_broom(),
            prune_time.elapsed().as_millis() as f32 / 1000.0
        );

        Ok(pruned)
    }

    async fn extract(&self, maintainer: &NodeMaintainer) -> Result<usize> {
        // Set up progress bar and timing stuff.
        let extract_time = std::time::Instant::now();
        let extract_span = tracing::debug_span!("extract");
        extract_span.pb_set_style(
            &ProgressStyle::default_bar()
                .template(&format!(
                    "{}Extracting {}",
                    self.emoji_package(),
                    "{bar:40} [{pos}/{len}] {wide_msg:.dim}"
                ))
                .unwrap(),
        );
        extract_span.pb_set_length(maintainer.package_count() as u64);
        let extract_span_enter = extract_span.enter();

        // Actually do the extraction.
        let extracted = maintainer.extract().await?;

        // Wrap up progress bar and message.
        std::mem::drop(extract_span_enter);
        std::mem::drop(extract_span);
        tracing::info!(
            "{}Extracted {extracted} package{} in {}s.",
            self.emoji_package(),
            if extracted == 1 { "" } else { "s" },
            extract_time.elapsed().as_millis() as f32 / 1000.0
        );

        Ok(extracted)
    }

    async fn rebuild(&self, maintainer: &NodeMaintainer) -> Result<()> {
        let script_time = std::time::Instant::now();
        let script_span = if self.scripts {
            tracing::info_span!("Building")
        } else {
            tracing::debug_span!("Building")
        };
        if self.scripts {
            script_span.pb_set_style(
                &ProgressStyle::default_bar()
                    .template(&format!(
                        "{{spinner}} {}Running scripts {{wide_msg:.dim}}",
                        self.emoji_run(),
                    ))
                    .unwrap(),
            );
        }
        maintainer
            .rebuild(!self.scripts)
            .instrument(script_span)
            .await?;
        if self.scripts {
            tracing::info!(
                "{}Ran lifecycle scripts in {}s.",
                self.emoji_run(),
                script_time.elapsed().as_millis() as f32 / 1000.0
            );
        } else {
            tracing::info!(
                "{}Linked script bins in {}s.",
                self.emoji_link(),
                script_time.elapsed().as_millis() as f32 / 1000.0
            );
        }
        Ok(())
    }

    fn emoji_run(&self) -> &'static str {
        self.maybe_emoji("🏃 ")
    }

    fn emoji_package(&self) -> &'static str {
        self.maybe_emoji("📦 ")
    }

    fn emoji_magnifying_glass(&self) -> &'static str {
        self.maybe_emoji("🔍 ")
    }

    fn emoji_broom(&self) -> &'static str {
        self.maybe_emoji("🧹 ")
    }

    fn emoji_writing(&self) -> &'static str {
        self.maybe_emoji("📝 ")
    }

    fn emoji_tada(&self) -> &'static str {
        self.maybe_emoji("🎉 ")
    }

    fn emoji_link(&self) -> &'static str {
        self.maybe_emoji("🔗 ")
    }

    fn maybe_emoji(&self, emoji: &'static str) -> &'static str {
        if self.emoji {
            emoji
        } else {
            ""
        }
    }
}

// Inspired and brazenly taken from SLIME:
// https://github.com/slime/slime/blob/e193bc5f3431a2f71f1d7a0e3f28e6dc4dd5de2d/slime.el#L1360-L1375
fn hackerish_encouragement() -> &'static str {
    let encouragements = [
        "Let the hacking commence!",
        "Hacks and glory await!",
        "Hack and be merry!",
        "Your hacking starts... NOW!",
        "May the source be with you.",
        "Fasterthanlime-fame is but a hack away!",
        "Hack the planet!",
        "Hackuna Matata~",
        "Are we JavaScript yet?",
        "[Scientifically-proven optimal words of hackerish encouragement here]",
    ];

    let mut rng = rand::thread_rng();
    encouragements
        .iter()
        .choose(&mut rng)
        .expect("Iterator should not be empty.")
}
