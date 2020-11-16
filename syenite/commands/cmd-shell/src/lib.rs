use async_trait::async_trait;
use clap::Clap;
use directories::ProjectDirs;
use oro_command::OroCommand;
use oro_config::OroConfigLayer;
use oro_diagnostics::{AsDiagnostic, DiagnosticResult as Result};
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
                env::current_exe().as_diagnostic("shell::no_oro_bin")?,
            )
            .arg("-r")
            .arg(require_alabaster(self.data_dir)?)
            .args(self.args)
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .stdin(Stdio::inherit())
            .status()
            .as_diagnostic("shell::binerr")?
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
            .expect("Couldn't find home directory.")
            .data_dir()
            .to_path_buf(),
    };
    fs::create_dir_all(&dir).as_diagnostic("shell::data_dir_err")?;
    let data = include_bytes!("../../../../alabaster/dist/alabaster.js").to_vec();
    let hash = Integrity::from(&data).to_hex().1;
    let script = dir.join(format!("oro-{}", hash));
    if !script.exists() {
        fs::write(&script, &data).as_diagnostic("shell::script_write_fail")?;
    }
    Ok(script)
}
