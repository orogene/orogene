use crate::client_args::ClientArgs;
use crate::commands::pack::PackCmd as Packer;
use crate::{commands::OroCommand, nassun_args::NassunArgs};
use async_trait::async_trait;
use clap::{clap_derive::ValueEnum, Args};
use directories::ProjectDirs;
use kdl::KdlDocument;
use miette::{IntoDiagnostic, Result};
use oro_common::{Access, Manifest};
use oro_npm_account::config;
use oro_npm_publish::publish::{publish, PublishOptions};
use oro_package_spec::PackageSpec;
use oro_script::{OroScript, OroScriptError};
use std::path::PathBuf;
use std::process::Stdio;

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum IAccess {
    Public,
    Restricted,
}

impl From<IAccess> for Access {
    fn from(access: IAccess) -> Self {
        match access {
            IAccess::Public => Access::Public,
            IAccess::Restricted => Access::Restricted,
        }
    }
}

#[derive(Debug, Args)]
pub struct PublishCmd {
    #[arg()]
    spec: Option<String>,

    #[arg(from_global)]
    config: Option<PathBuf>,

    /// Indicates that you don't want npm to make any changes and
    /// that it should only report what it would have done.
    #[arg(long, default_value_t = false)]
    dry_run: bool,

    /// Specifies the tag of the package to be published.
    #[arg(long, default_value = "latest")]
    tag: String,

    #[arg(long, value_enum, default_value_t = IAccess::Public)]
    access: IAccess,

    #[command(flatten)]
    nassun_args: NassunArgs,
}

#[async_trait]
impl OroCommand for PublishCmd {
    async fn execute(self) -> Result<()> {
        if let Some(config_path) = &self.config.or_else(|| {
            ProjectDirs::from("", "", "orogene")
                .map(|config| config.config_dir().to_path_buf().join("oro.kdl"))
        }) {
            let root = self.nassun_args.root.clone();
            let registry = self.nassun_args.registry.clone();
            let spec = self
                .spec
                .map_or(Ok(PackageSpec::Dir { path: root.clone() }), |v| v.parse())?;
            let package = self
                .nassun_args
                .to_nassun()?
                .resolve_spec(spec.clone())
                .await?;
            let config: KdlDocument = std::fs::read_to_string(config_path)
                .into_diagnostic()?
                .parse()?;
            let credentials = config::get_credentials_by_uri(&registry, &config);
            let client: ClientArgs = self.nassun_args.into();
            let client = client.into_client_builder(credentials.as_ref())?.build();
            let mut manifest = package.metadata().await?.manifest;
            oro_package_json::normalize(&mut manifest, root.clone(), true).await?;
            Self::run_prepublishonly_script(spec.clone(), &manifest).await?;
            Packer::run_prepack_script(spec.clone(), &manifest).await?;
            let tarball = package.tarball().await?.into_inner();
            Packer::run_postpack_script(spec.clone(), &manifest).await?;
            tracing::info!(
                "Publishing to {} with tag {} and {:?} access {}",
                registry,
                self.tag,
                self.access,
                if self.dry_run { "(dry-run)" } else { "" }
            );
            if credentials.is_none() && !self.dry_run {
                return Err(miette::miette!(
                    "This command requires you to be logged in to {registry}"
                ));
            }
            if !self.dry_run {
                publish(
                    &manifest,
                    tarball,
                    PublishOptions {
                        default_tag: self.tag,
                        access: self.access.into(),
                        client,
                        ..Default::default()
                    },
                )
                .await?;
            }
            Self::run_publish_script(spec.clone(), &manifest).await?;
            Self::run_postpublish_script(spec.clone(), &manifest).await?;
        }
        Ok(())
    }
}

impl PublishCmd {
    pub async fn run_prepublishonly_script(
        spec: PackageSpec,
        manifest: &Manifest,
    ) -> Result<(), OroScriptError> {
        if manifest.scripts.get("prepublishOnly").is_some() {
            if let PackageSpec::Dir { path } = spec {
                async_std::task::spawn_blocking(move || {
                    OroScript::new(path, "prepublishOnly")?
                        .stdout(Stdio::inherit())
                        .spawn()?
                        .wait()
                })
                .await?;
            }
        }
        Ok(())
    }

    pub async fn run_publish_script(
        spec: PackageSpec,
        manifest: &Manifest,
    ) -> Result<(), OroScriptError> {
        if manifest.scripts.get("publish").is_some() {
            if let PackageSpec::Dir { path } = spec {
                async_std::task::spawn_blocking(move || {
                    OroScript::new(path, "publish")?
                        .stdout(Stdio::inherit())
                        .spawn()?
                        .wait()
                })
                .await?;
            }
        }
        Ok(())
    }

    pub async fn run_postpublish_script(
        spec: PackageSpec,
        manifest: &Manifest,
    ) -> Result<(), OroScriptError> {
        if manifest.scripts.get("postpublish").is_some() {
            if let PackageSpec::Dir { path } = spec {
                async_std::task::spawn_blocking(move || {
                    OroScript::new(path, "postpublish")?
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
