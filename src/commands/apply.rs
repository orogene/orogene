use async_trait::async_trait;
use clap::Args;
use miette::Result;

use crate::apply_args::ApplyArgs;
use crate::commands::OroCommand;

/// Applies the current project's requested dependencies to `node_modules/`,
/// adding, removing, and updating dependencies as needed. This command is
/// intended to be an idempotent way to make sure your `node_modules` is in
/// the right state to execute, based on your declared dependencies.
///
/// This command is automatically executed by a number of Orogene subcommands.
/// To force a full reapplication of `node_modules`, consider using the `oro
/// reapply` command.
#[derive(Debug, Args)]
#[clap(visible_aliases(["a", "ap", "app"]))]
pub struct ApplyCmd {
    #[command(flatten)]
    apply: ApplyArgs,
}

#[async_trait]
impl OroCommand for ApplyCmd {
    async fn execute(mut self) -> Result<()> {
        // Running `apply` with `--no-apply` doesn't make sense. We force it
        // here so that people can have `apply false` in their configurations
        // but have `oro apply` still work.
        self.apply.apply = true;
        self.apply.execute().await
    }
}
