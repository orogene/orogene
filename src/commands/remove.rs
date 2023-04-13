use async_trait::async_trait;
use clap::Args;
use miette::{IntoDiagnostic, Result};
use oro_pretty_json::Formatted;

use crate::apply_args::ApplyArgs;
use crate::commands::OroCommand;

/// Removes one or more dependencies to the target package.
#[derive(Debug, Args)]
#[clap(visible_aliases(["rm"]))]
pub struct RemoveCmd {
    /// Package names of dependencies to remove. These will be removed from
    /// all dependency types.
    #[arg(required = true)]
    names: Vec<String>,

    #[command(flatten)]
    apply: ApplyArgs,
}

#[async_trait]
impl OroCommand for RemoveCmd {
    async fn execute(self) -> Result<()> {
        let mut manifest = oro_pretty_json::from_str(
            &async_std::fs::read_to_string(self.apply.root.join("package.json"))
                .await
                .into_diagnostic()?,
        )
        .into_diagnostic()?;
        let mut count = 0;
        for name in &self.names {
            count += self.remove_from_manifest(&mut manifest, name);
        }

        async_std::fs::write(
            self.apply.root.join("package.json"),
            oro_pretty_json::to_string_pretty(&manifest).into_diagnostic()?,
        )
        .await
        .into_diagnostic()?;

        tracing::info!(
            "{}Removed {count} dependencies from package.json.",
            if self.apply.emoji { "📝 " } else { "" },
        );

        // TODO: Force locked = false here, once --locked is supported.
        // Using `oro remove` with `--locked` doesn't make sense.
        // self.apply.locked = false;

        // Then, we apply the change.
        self.apply.execute().await
    }
}

impl RemoveCmd {
    fn remove_from_manifest(&self, mani: &mut Formatted, name: &str) -> usize {
        let mut count = 0;
        for ty in [
            "dependencies",
            "devDependencies",
            "optionalDependencies",
            "peerDependencies",
        ] {
            if mani.value[ty].is_object() {
                if let Some(obj) = mani.value[ty].as_object_mut() {
                    if obj.contains_key(name) {
                        tracing::debug!(
                            "Removing {name}@{} from {ty}.",
                            obj[name].as_str().unwrap_or("")
                        );
                        obj.remove(name);
                        count += 1;
                    }
                }
            }
        }
        count
    }
}
