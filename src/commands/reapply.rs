use async_trait::async_trait;
use clap::Args;
use miette::{IntoDiagnostic, Result};
use oro_common::CorgiManifest;

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

        let nm = self.apply.root.join("node_modules");

        if nm.exists() {
            tracing::info!(
                "{}Clearing node_modules/...",
                if self.apply.emoji { "🚮 " } else { "" },
            );

            std::fs::remove_dir_all(nm).into_diagnostic()?;

            tracing::info!(
                "{}node_modules/ cleared in {}s.",
                if self.apply.emoji { "🚮 " } else { "" },
                total_time.elapsed().as_millis() as f32 / 1000.0,
            );
        } else {
            tracing::info!(
                "{}node_modules/ does not exist. Nothing to clear.",
                if self.apply.emoji { "🚮 " } else { "" },
            )
        }

        let corgi: CorgiManifest = serde_json::from_str(
            &async_std::fs::read_to_string(self.apply.root.join("package.json"))
                .await
                .into_diagnostic()?,
        )
        .into_diagnostic()?;

        // Running `reapply` with `--no-apply` doesn't make sense. We force it
        // here so that people can have `apply false` in their configurations
        // but have `oro apply` still work.
        self.apply.apply = true;
        self.apply.execute(corgi).await?;

        tracing::info!(
            "{}Reapply done in {}s.",
            if self.apply.emoji { "✨ " } else { "" },
            total_time.elapsed().as_millis() as f32 / 1000.0,
        );
        Ok(())
    }
}
