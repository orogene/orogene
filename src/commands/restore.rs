use std::path::{Path, PathBuf};

use async_trait::async_trait;
use clap::Args;
use indicatif::ProgressStyle;
use miette::{IntoDiagnostic, Result};
use node_maintainer::{NodeMaintainer, NodeMaintainerOptions};
use oro_config::OroConfigLayer;
use rand::seq::IteratorRandom;
use tracing::{Instrument, Span};
use tracing_indicatif::span_ext::IndicatifSpanExt;
use url::Url;

use crate::commands::OroCommand;

#[derive(Debug, Args, OroConfigLayer)]
pub struct RestoreCmd {
    /// When extracting packages, prefer to copy files files instead of
    /// linking them.
    ///
    /// This option has no effect if hard linking fails (for example, if the
    /// cache is on a different drive), or if the project is on a filesystem
    /// that supports Copy-on-Write (zfs, btrfs, APFS (macOS), etc).
    #[arg(long)]
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

    /// Skip running install scripts.
    #[arg(long)]
    ignore_scripts: bool,

    /// Default dist-tag to use when resolving package versions.
    #[arg(long)]
    default_tag: Option<String>,

    /// Controls number of concurrent operations during various restore steps
    /// (resolution fetches, extractions, etc). Tuning this might help reduce
    /// memory usage.
    #[arg(long)]
    concurrency: Option<usize>,

    #[arg(from_global)]
    registry: Option<Url>,

    #[arg(from_global)]
    json: bool,

    #[arg(from_global)]
    root: Option<PathBuf>,

    #[arg(from_global)]
    cache: Option<PathBuf>,

    #[arg(from_global)]
    no_emoji: bool,
}

#[async_trait]
impl OroCommand for RestoreCmd {
    async fn execute(self) -> Result<()> {
        let total_time = std::time::Instant::now();
        let emoji = Emoji::new(!self.no_emoji);
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
        if let Some(registry) = self.registry.as_ref() {
            nm = nm.registry(registry.clone());
        }
        if let Some(cache) = self.cache.as_deref() {
            nm = nm.cache(cache);
        }
        if let Some(tag) = self.default_tag.as_deref() {
            nm = nm.default_tag(tag);
        }
        if let Some(concurrency) = self.concurrency {
            nm = nm.concurrency(concurrency);
        }

        let resolved_nm = self.resolve(&emoji, root, nm).await?;

        if !self.lockfile_only {
            self.prune(&emoji, &resolved_nm).await?;
            self.extract(&emoji, &resolved_nm).await?;
            self.rebuild(&emoji, &resolved_nm).await?;
        } else {
            tracing::info!(
                "{}Skipping installing node_modules/, only writing lockfile.",
                emoji.package()
            );
        }

        resolved_nm
            .write_lockfile(root.join("package-lock.kdl"))
            .await?;

        tracing::info!("{}Wrote lockfile to package-lock.kdl.", emoji.writing());

        tracing::info!(
            "{}Done in {}s. {}",
            emoji.tada(),
            total_time.elapsed().as_millis() as f32 / 1000.0,
            hackerish_encouragement()
        );
        Ok(())
    }
}

impl RestoreCmd {
    async fn resolve(
        &self,
        emoji: &Emoji,
        root: &Path,
        builder: NodeMaintainerOptions,
    ) -> Result<NodeMaintainer> {
        // Set up progress bar and timing stuff.
        let resolve_time = std::time::Instant::now();
        let resolve_span = tracing::debug_span!("resolving");
        resolve_span.pb_set_style(
            &ProgressStyle::default_bar()
                .template(&format!(
                    "{}Resolving {}",
                    emoji.magnifying_glass(),
                    "{bar:40} [{pos}/{len}] {wide_msg:.dim}"
                ))
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
        tracing::info!(
            "{}Resolved {} packages in {}s.",
            emoji.magnifying_glass(),
            resolved_nm.package_count(),
            resolve_time.elapsed().as_millis() as f32 / 1000.0
        );

        Ok(resolved_nm)
    }

    async fn prune(&self, emoji: &Emoji, maintainer: &NodeMaintainer) -> Result<usize> {
        // Set up progress bar and timing stuff.
        let prune_time = std::time::Instant::now();
        let prune_span = tracing::debug_span!("prune");
        prune_span.pb_set_style(
            &ProgressStyle::default_bar()
                .template(&format!(
                    "{}Pruning extraneous {}",
                    emoji.broom(),
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
            emoji.broom(),
            prune_time.elapsed().as_millis() as f32 / 1000.0
        );

        Ok(pruned)
    }

    async fn extract(&self, emoji: &Emoji, maintainer: &NodeMaintainer) -> Result<usize> {
        // Set up progress bar and timing stuff.
        let extract_time = std::time::Instant::now();
        let extract_span = tracing::debug_span!("extract");
        extract_span.pb_set_style(
            &ProgressStyle::default_bar()
                .template(&format!(
                    "{}Extracting {}",
                    emoji.package(),
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
            emoji.package(),
            if extracted == 1 { "" } else { "s" },
            extract_time.elapsed().as_millis() as f32 / 1000.0
        );

        Ok(extracted)
    }

    async fn rebuild(&self, emoji: &Emoji, maintainer: &NodeMaintainer) -> Result<()> {
        let script_time = std::time::Instant::now();
        let script_span = tracing::info_span!("Building");
        script_span.pb_set_style(
            &ProgressStyle::default_bar()
                .template(&format!(
                    "{{spinner}} {}Running scripts {{wide_msg:.dim}}",
                    emoji.run(),
                ))
                .unwrap(),
        );
        maintainer
            .rebuild(self.ignore_scripts)
            .instrument(script_span)
            .await?;
        tracing::info!(
            "{}Ran lifecycle scripts in {}s.",
            emoji.run(),
            script_time.elapsed().as_millis() as f32 / 1000.0
        );
        Ok(())
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

struct Emoji(bool);

impl Emoji {
    fn new(use_emoji: bool) -> Self {
        Self(use_emoji)
    }

    const RUN: &'static str = "🏃 ";
    const PACKAGE: &'static str = "📦 ";
    const MAGNIFYING_GLASS: &'static str = "🔍 ";
    const BROOM: &'static str = "🧹 ";
    const WRITING: &'static str = "📝 ";
    const TADA: &'static str = "🎉 ";

    fn run(&self) -> &'static str {
        if self.0 {
            Self::RUN
        } else {
            ""
        }
    }

    fn package(&self) -> &'static str {
        if self.0 {
            Self::PACKAGE
        } else {
            ""
        }
    }

    fn magnifying_glass(&self) -> &'static str {
        if self.0 {
            Self::MAGNIFYING_GLASS
        } else {
            ""
        }
    }

    fn broom(&self) -> &'static str {
        if self.0 {
            Self::BROOM
        } else {
            ""
        }
    }

    fn writing(&self) -> &'static str {
        if self.0 {
            Self::WRITING
        } else {
            ""
        }
    }

    fn tada(&self) -> &'static str {
        if self.0 {
            Self::TADA
        } else {
            ""
        }
    }
}
