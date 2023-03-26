use miette::Diagnostic;
use thiserror::Error;

#[derive(Debug, Error, Diagnostic)]
pub enum OroScriptError {
    #[error(transparent)]
    #[diagnostic(code(oro_script::io_error))]
    IoError(#[from] std::io::Error),

    #[error(transparent)]
    #[diagnostic(code(oro_script::serde_error))]
    SerdeError(#[from] serde_json::Error),

    #[error("Failed to spawn child process.")]
    #[diagnostic(code(oro_script::spawn_error))]
    SpawnError(#[source] std::io::Error),

    #[error("Failed to find event `{0}` in package.")]
    #[diagnostic(code(oro_script::missing_event))]
    MissingEvent(String),

    #[error(transparent)]
    #[diagnostic(code(oro_script::join_path_error))]
    JoinPathError(#[from] std::env::JoinPathsError),

    #[error("Error parsing script: `{0}`")]
    #[diagnostic(code(oro_script::parse_error))]
    ScriptParseError(String),

    #[error("Error performing process operation on script.")]
    #[diagnostic(code(oro_script::script_process_error))]
    ScriptProcessError(#[source] std::io::Error),

    #[error("Script exited with code {}", .0.code().unwrap_or(-1))]
    #[diagnostic(code(oro_script::script_error))]
    ScriptError(std::process::ExitStatus, Option<Vec<u8>>, Option<Vec<u8>>),
}

pub(crate) type Result<T> = std::result::Result<T, OroScriptError>;
