use async_trait::async_trait;
use clap::Args;
use miette::{IntoDiagnostic, Result};
use nassun::PackageSpec;
use oro_common::CorgiManifest;
use oro_pretty_json::Formatted;

use crate::apply_args::ApplyArgs;
use crate::commands::OroCommand;
use crate::OroError;

/// Removes one or more dependencies from the target package.
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
    async fn execute(mut self) -> Result<()> {
        let mut manifest = oro_pretty_json::from_str(
            async_std::fs::read_to_string(self.apply.root.join("package.json"))
                .await
                .into_diagnostic()?,
        )
        .into_diagnostic()?;
        let mut count = 0;
        for name in &self.names {
            if let Ok(PackageSpec::Npm {
                name: spec_name, ..
            }) = name.parse()
            {
                if &spec_name != name {
                    tracing::warn!("Ignoring version specifier in `{name}`. Arguments to `oro remove` should only be package names. Proceeding with `{spec_name}` instead.");
                }
                count += self.remove_from_manifest(&mut manifest, &spec_name);
            } else {
                return Err(OroError::InvalidPackageName(name.clone()).into());
            }
        }

        if self.apply.locked {
            // NOTE: we force locked to be false here, because it doesn't make
            // sense to run this command in locked mode.
            tracing::info!("Ignoring --locked option. It doesn't make sense to run this command in locked mode.");
            self.apply.locked = false;
        }

        let corgi: CorgiManifest =
            serde_json::from_str(&oro_pretty_json::to_string_pretty(&manifest).into_diagnostic()?)
                .into_diagnostic()?;

        // Then, we apply the change.
        self.apply.execute(corgi).await?;

        async_std::fs::write(
            self.apply.root.join("package.json"),
            oro_pretty_json::to_string_pretty(&manifest).into_diagnostic()?,
        )
        .await
        .into_diagnostic()?;

        tracing::info!(
            "{}Removed {count} dependenc{} from package.json.",
            if self.apply.emoji { "📝 " } else { "" },
            if count == 1 { "y" } else { "ies" },
        );

        Ok(())
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
