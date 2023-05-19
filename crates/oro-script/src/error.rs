use miette::Diagnostic;
use thiserror::Error;

#[derive(Debug, Error, Diagnostic)]
pub enum OroScriptError {
    /// Generic IO-related error. Refer to the error message for more details.
    #[error("{0}")]
    #[diagnostic(code(oro_script::io_error), url(docsrs))]
    IoError(String, #[source] std::io::Error),

    /// Generic serde-related error. Refer to the error message for more
    /// details.
    #[error(transparent)]
    #[diagnostic(code(oro_script::serde_error), url(docsrs))]
    SerdeError(#[from] serde_json::Error),

    /// Failed to spawn child process when executing a script. Refer to the
    /// error message for more details.
    #[error("Failed to spawn child process.")]
    #[diagnostic(code(oro_script::spawn_error), url(docsrs))]
    SpawnError(#[source] std::io::Error),

    /// Failed to find an event in a package's `package.json`. This means, for
    /// example, that a `"postinstall"` script was requested, but not actually
    /// present.
    #[error("Failed to find event `{0}` in package.")]
    #[diagnostic(code(oro_script::missing_event), url(docsrs))]
    MissingEvent(String),

    /// Failed to join new contents of PATH variable while trying to add a
    /// `node_modules/.bin` entry to the PATH.
    ///
    /// When executing a script, the current package and their ancestors get
    /// their `node_modules/.bin` directories added to the PATH. This error
    /// means something went wrong while putting the variable back together.
    /// For more details on what may have happened, refer to the error
    /// message.
    #[error("Failed to join new contents of PATH variable while trying to add a `node_modules/.bin` entry to the PATH.")]
    #[diagnostic(code(oro_script::join_path_error), url(docsrs))]
    JoinPathError(#[from] std::env::JoinPathsError),

    /// Something went wrong while performing an operation on a script's
    /// process. For more details, refer to the error message.
    #[error("Error performing process operation on script.")]
    #[diagnostic(code(oro_script::script_process_error), url(docsrs))]
    ScriptProcessError(#[source] std::io::Error),

    /// The script terminated with a non-zero exit code, meaning some error
    /// happened with the script itself. Details may have been logged in the
    /// log file/stdout/stderr.
    #[error("Script exited with code {}", .0.code().unwrap_or(-1))]
    #[diagnostic(code(oro_script::script_error), url(docsrs))]
    ScriptError(std::process::ExitStatus, Option<Vec<u8>>, Option<Vec<u8>>),
}

pub(crate) type Result<T> = std::result::Result<T, OroScriptError>;

pub trait IoContext {
    type T;

    fn io_context(self, context: impl FnOnce() -> String) -> Result<Self::T>;
}

impl<T> IoContext for std::result::Result<T, std::io::Error> {
    type T = T;

    fn io_context(self, context: impl FnOnce() -> String) -> Result<Self::T> {
        self.map_err(|e| OroScriptError::IoError(context(), e))
    }
}
