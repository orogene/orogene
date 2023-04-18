use async_trait::async_trait;
use clap::Args;
use miette::{Diagnostic, IntoDiagnostic, Result};
use nassun::PackageSpec;
use thiserror::Error;
use oro_pretty_json::Formatted;

use crate::apply_args::ApplyArgs;
use crate::commands::OroCommand;

#[derive(Debug, Error, Diagnostic)]
enum RemoveCmdError {
    /// Invalid package name. Only package names should be passed to `oro
    /// remove`, but you passed either a package specifier or an invalid
    /// package name.
    #[error("{0} is not a valid package name. Only package names should be passed to `oro remove`, but you passed either a non-NPM package specifier or an invalid package name.")]
    #[diagnostic(
        code(oro::remove::invalid_package_name),
        url(docsrs)
    )]
    InvalidPackageName(String)
}

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
            &async_std::fs::read_to_string(self.apply.root.join("package.json"))
                .await
                .into_diagnostic()?,
        )
        .into_diagnostic()?;
        let mut count = 0;
        for name in &self.names {
            if let Ok(PackageSpec::Npm { name: spec_name, .. }) = name.parse() {
                if &spec_name != name {
                    tracing::warn!("Ignoring version specifier in {name}. Arguments to `oro remove` should only be package names. Proceeding with {spec_name} instead.");
                }
                count += self.remove_from_manifest(&mut manifest, name);
            } else {
                return Err(RemoveCmdError::InvalidPackageName(name.clone()).into());
            }
        }

        async_std::fs::write(
            self.apply.root.join("package.json"),
            oro_pretty_json::to_string_pretty(&manifest).into_diagnostic()?,
        )
        .await
        .into_diagnostic()?;

        tracing::info!(
            "{}Removed {count} dependenc{} from package.json.",
            if count == 1 { "y" } else { "ies" },
            if self.apply.emoji { "📝 " } else { "" },
        );

        if self.apply.locked {
            // NOTE: we force locked to be false here, because it doesn't make
            // sense to run this command in locked mode.
            tracing::info!("Ignoring --locked option. It doesn't make sense to run this command in locked mode.");
            self.apply.locked = false;
        }

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
