use async_trait::async_trait;
use clap::Args;
use miette::{IntoDiagnostic, Result};
use nassun::PackageResolution;
use oro_common::CorgiManifest;
use oro_package_spec::{PackageSpec, VersionSpec};
use oro_pretty_json::Formatted;

use crate::apply_args::ApplyArgs;
use crate::commands::OroCommand;
use crate::nassun_args::NassunArgs;

/// Adds one or more dependencies to the target package.
#[derive(Debug, Args)]
pub struct AddCmd {
    /// Specifiers for packages to add.
    #[arg(required = true)]
    specs: Vec<String>,

    /// Prefix to prepend to package versions for resolved NPM dependencies.
    ///
    /// For example, if you do `oro add foo@1.2.3 --prefix ~`, this will write `"foo": "~1.2.3"` to your `package.json`.
    #[arg(long, default_value = "^")]
    prefix: String,

    /// Add packages as devDependencies.
    #[arg(long, short = 'D')]
    dev: bool,

    /// Add packages as optionalDependencies.
    #[arg(long, short = 'O', visible_alias = "optional")]
    opt: bool,

    #[command(flatten)]
    apply: ApplyArgs,
}

#[async_trait]
impl OroCommand for AddCmd {
    async fn execute(mut self) -> Result<()> {
        let mut manifest = oro_pretty_json::from_str(
            &async_std::fs::read_to_string(self.apply.root.join("package.json"))
                .await
                .into_diagnostic()?,
        )
        .into_diagnostic()?;
        let nassun = NassunArgs::from_apply_args(&self.apply).to_nassun()?;
        use PackageResolution as Pr;
        use PackageSpec as Ps;
        let mut count = 0;
        for spec in &self.specs {
            let pkg = nassun.resolve(spec).await?;
            let name = pkg.name();
            let requested: PackageSpec = spec.parse()?;
            let resolved_spec = match requested.target() {
                Ps::Alias { .. } => {
                    unreachable!(".target() ensures this alias is fully resolved");
                }
                Ps::Git(info) => {
                    format!("{info}")
                }
                Ps::Dir { path } => {
                    {
                        // TODO: make relative to root?
                        path.to_string_lossy().to_string()
                    }
                }
                Ps::Npm { .. } => {
                    let mut from = pkg.from().clone();
                    let resolved = pkg.resolved();
                    let version = if let Pr::Npm { version, .. } = resolved {
                        version
                    } else {
                        unreachable!("No other type of spec should be here.");
                    };
                    match from.target_mut() {
                        Ps::Npm { requested, .. } => {
                            // We use Tag in a hacky way here to have some level of "preserved" formatting.
                            *requested =
                                Some(VersionSpec::Tag(format!("{}{version}", self.prefix)));
                        }
                        _ => {
                            unreachable!("No other type of spec should be here.");
                        }
                    }
                    from.requested()
                }
            };
            tracing::info!(
                "{}Resolved {spec} to {name}@{resolved_spec}.",
                if self.apply.emoji { "🔍 " } else { "" }
            );
            self.remove_from_manifest(&mut manifest, name);
            self.add_to_manifest(&mut manifest, name, &resolved_spec);
            count += 1;
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
            "{}Updated package.json with {count} new {}.",
            if self.apply.emoji { "📝 " } else { "" },
            if count == 1 {
                self.dep_kind_str_singular()
            } else {
                self.dep_kind_str()
            }
        );

        Ok(())
    }
}

impl AddCmd {
    fn add_to_manifest(&self, mani: &mut Formatted, name: &str, spec: &str) {
        let deps = self.dep_kind_str();
        tracing::debug!("Adding {name}@{spec} to {deps}.");
        mani.value[deps][name] =
            serde_json::to_value(spec).expect("Value is always a valid string");
    }

    fn remove_from_manifest(&self, mani: &mut Formatted, name: &str) {
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
                    }
                }
            }
        }
    }

    fn dep_kind_str(&self) -> &'static str {
        if self.dev {
            "devDependencies"
        } else if self.opt {
            "optionalDependencies"
        } else {
            "dependencies"
        }
    }

    fn dep_kind_str_singular(&self) -> &'static str {
        if self.dev {
            "devDependency"
        } else if self.opt {
            "optionalDependency"
        } else {
            "dependency"
        }
    }
}
