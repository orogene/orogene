use anyhow::{anyhow, Context, Result};
use async_trait::async_trait;
use clap::Clap;
use directories::ProjectDirs;
use oro_command::OroCommand;
use oro_config::OroConfigLayer;
use oro_diagnostics::DiagnosticCode;
use ssri::Integrity;
use std::path::PathBuf;
use std::process::{self, Command, Stdio};
use std::{env, fs};

#[derive(Debug, Clap, OroConfigLayer)]
pub struct ShellCmd {
    #[clap(long, default_value = "node")]
    node: String,

    #[clap(from_global)]
    data_dir: Option<PathBuf>,

    #[clap(from_global)]
    loglevel: log::LevelFilter,

    #[clap(from_global)]
    json: bool,

    #[clap(from_global)]
    quiet: bool,

    #[clap(multiple = true)]
    #[oro_config(ignore)]
    args: Vec<String>,
}

#[async_trait]
impl OroCommand for ShellCmd {
    async fn execute(self) -> Result<()> {
        let node = self.node;
        let code = Command::new(&node)
            .env(
                "ORO_BIN",
                env::current_exe().with_context(|| {
                    format!(
                        "{:#?}: Failed to get path for current executable.",
                        DiagnosticCode::OR1018
                    )
                })?,
            )
            .arg("-r")
            .arg(require_alabaster(self.data_dir)?)
            .args(self.args)
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .stdin(Stdio::inherit())
            .status()
            .with_context(|| {
                format!(
                    "{:#?}: Failed to execute node binary at `{}`.",
                    DiagnosticCode::OR1017,
                    node
                )
            })?
            .code()
            .unwrap_or(1);
        if code > 0 {
            process::exit(code);
        }
        Ok(())
    }
}

fn require_alabaster(dir_override: Option<PathBuf>) -> Result<PathBuf> {
    let dir = match dir_override {
        Some(dir) => dir,
        None => ProjectDirs::from("", "", "orogene") // TODO I'd rather get this from oro-config?
            .ok_or_else(|| {
                anyhow!(
                    "{:#?}: Couldn't find home directory.",
                    DiagnosticCode::OR1019
                )
            })?
            .data_dir()
            .to_path_buf(),
    };
    fs::create_dir_all(&dir).with_context(|| {
        format!(
            "{:#?}: Failed to create data directory at `{:?}`",
            DiagnosticCode::OR1020,
            dir
        )
    })?;
    let data = include_bytes!("../../../../alabaster/dist/alabaster.js").to_vec();
    let hash = Integrity::from(&data).to_hex().1;
    let script = dir.join(format!("oro-{}", hash));
    if !script.exists() {
        fs::write(&script, &data).with_context(|| {
            format!(
                "{:#?}: Failed to write alabaster patches to orogene data dir at {:?}.",
                DiagnosticCode::OR1021,
                script
            )
        })?;
    }
    Ok(script)
}
