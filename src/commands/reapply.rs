use async_trait::async_trait;
use clap::Args;
use miette::{IntoDiagnostic, Result};

use crate::apply_args::ApplyArgs;
use crate::commands::OroCommand;

/// Removes the existing `node_modules`, if any, and reapplies it from
/// scratch. You can use this to make sure you have a pristine `node_modules`.
#[derive(Debug, Args)]
pub struct ReapplyCmd {
    #[command(flatten)]
    apply: ApplyArgs,
}

#[async_trait]
impl OroCommand for ReapplyCmd {
    async fn execute(mut self) -> Result<()> {
        let total_time = std::time::Instant::now();

        tracing::info!(
            "{}Clearing node_modules/...",
            if self.apply.emoji { "🚮 " } else { "" },
        );

        std::fs::remove_dir_all(self.apply.root.join("node_modules")).into_diagnostic()?;

        tracing::info!(
            "{}node_modules/ cleared in {}s.",
            if self.apply.emoji { "🚮 " } else { "" },
            total_time.elapsed().as_millis() as f32 / 1000.0,
        );

        // Running `reapply` with `--no-apply` doesn't make sense. We force it
        // here so that people can have `apply false` in their configurations
        // but have `oro apply` still work.
        self.apply.apply = true;
        self.apply.execute().await?;

        tracing::info!(
            "{}Reapply done in {}s.",
            if self.apply.emoji { "✨ " } else { "" },
            total_time.elapsed().as_millis() as f32 / 1000.0,
        );
        Ok(())
    }
}
