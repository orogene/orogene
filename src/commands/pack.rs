use crate::{commands::OroCommand, nassun_args::NassunArgs};
use async_trait::async_trait;
use clap::Args;
use miette::{IntoDiagnostic, Result};
use oro_common::Manifest;
use oro_package_spec::PackageSpec;
use oro_script::{OroScript, OroScriptError};
use std::process::Stdio;

/// Create a tarball from package.
#[derive(Debug, Args)]
pub struct PackCmd {
    #[arg(action = clap::ArgAction::Append, num_args(0..))]
    specs: Option<Vec<String>>,

    /// Indicates that you don't want npm to make any changes and
    /// that it should only report what it would have done.
    #[arg(long, default_value_t = false)]
    dry_run: bool,

    #[command(flatten)]
    nassun_args: NassunArgs,
}

#[async_trait]
impl OroCommand for PackCmd {
    async fn execute(self) -> Result<()> {
        let root = self.nassun_args.root.clone();
        let nassun = self.nassun_args.to_nassun()?;
        let specs = if let Some(specs) = self.specs {
            specs
                .into_iter()
                .map(|spec| spec.parse())
                .collect::<Result<Vec<PackageSpec>, _>>()
                .into_diagnostic()?
        } else {
            vec![PackageSpec::Dir {
                path: self.nassun_args.root.clone(),
            }]
        };
        for spec in specs {
            let package = nassun.resolve_spec(spec.clone()).await?;
            let manifest = package.metadata().await?.manifest;
            Self::run_prepack_script(spec.clone(), &manifest).await?;
            let mut tarball = package.tarball().await?.into_inner();
            Self::run_postpack_script(spec.clone(), &manifest).await?;
            if !self.dry_run {
                let manifest_name = manifest.name.map_or("".to_owned(), |value| {
                    value.replace('/', "-").replace('@', "")
                });
                let manifest_version = manifest
                    .version
                    .map_or("".to_owned(), |value| value.to_string());
                let tarball_name = format!("{manifest_name}-{manifest_version}.tgz");
                let mut file = async_std::fs::File::create(root.join(&tarball_name))
                    .await
                    .into_diagnostic()?;
                async_std::io::copy(&mut tarball, &mut file)
                    .await
                    .into_diagnostic()?;
            }
        }
        Ok(())
    }
}

impl PackCmd {
    pub async fn run_prepack_script(
        spec: PackageSpec,
        manifest: &Manifest,
    ) -> Result<(), OroScriptError> {
        if manifest.scripts.get("prepack").is_some() {
            if let PackageSpec::Dir { path } = spec {
                async_std::task::spawn_blocking(move || {
                    OroScript::new(&*path, "prepack")?
                        .stdout(Stdio::inherit())
                        .spawn()?
                        .wait()
                })
                .await?;
            }
        }
        Ok(())
    }

    pub async fn run_postpack_script(
        spec: PackageSpec,
        manifest: &Manifest,
    ) -> Result<(), OroScriptError> {
        if manifest.scripts.get("postpack").is_some() {
            if let PackageSpec::Dir { path } = spec {
                async_std::task::spawn_blocking(move || {
                    OroScript::new(&*path, "postpack")?
                        .stdout(Stdio::inherit())
                        .spawn()?
                        .wait()
                })
                .await?;
            }
        }
        Ok(())
    }
}
