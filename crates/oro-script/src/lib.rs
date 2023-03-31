//! Execute package run-scripts and lifecycle scripts.

use std::ffi::{OsStr, OsString};
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStderr, ChildStdin, ChildStdout, Command, Output, Stdio};

pub use error::OroScriptError;
use error::Result;
use oro_common::BuildManifest;
use regex::Regex;

mod error;

#[derive(Debug)]
pub struct OroScript<'a> {
    manifest: Option<&'a BuildManifest>,
    event: String,
    package_path: PathBuf,
    paths: Vec<PathBuf>,
    cmd: Command,
    workspace_path: Option<PathBuf>,
}

impl<'a> OroScript<'a> {
    pub fn new(package_path: impl AsRef<Path>, event: impl AsRef<str>) -> Result<Self> {
        let package_path = dunce::canonicalize(package_path.as_ref())?;
        let shell = if cfg!(target_os = "windows") {
            if let Some(com_spec) = std::env::var_os("ComSpec") {
                com_spec
            } else {
                OsString::from("cmd")
            }
        } else {
            OsString::from("sh")
        };
        let shell_str = shell.to_string_lossy();
        let shell_is_cmd = Regex::new(r"(?:^|\\)cmd(?:\.exe)?$")
            .unwrap()
            .is_match(&shell_str);
        let mut cmd = Command::new(&shell);
        if shell_is_cmd {
            cmd.arg("/d");
            cmd.arg("/s");
            cmd.arg("/c");
        } else {
            cmd.arg("-c");
        }
        cmd.current_dir(&package_path);
        cmd.stdin(Stdio::null());
        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::piped());
        Ok(Self {
            event: event.as_ref().into(),
            manifest: None,
            package_path,
            paths: Self::get_existing_paths(),
            workspace_path: None,
            cmd,
        })
    }

    /// If specified, `node_modules/.bin` directories above this path will not
    /// be added to the $PATH variable when running the script.
    pub fn workspace_path(mut self, path: impl AsRef<Path>) -> Self {
        self.workspace_path = Some(path.as_ref().to_path_buf());
        self
    }

    /// Set an environment variable.
    pub fn env(mut self, key: impl AsRef<OsStr>, value: impl AsRef<OsStr>) -> Self {
        self.cmd.env(key.as_ref(), value.as_ref());
        self
    }

    /// Set the [`Stdio`] that the script will use as its
    /// standard output stream.
    pub fn stdout(mut self, stdout: impl Into<Stdio>) -> Self {
        self.cmd.stdout(stdout.into());
        self
    }

    /// Set the [`Stdio`] that the script will use as its
    /// standard error stream.
    pub fn stderr(mut self, stderr: impl Into<Stdio>) -> Self {
        self.cmd.stderr(stderr.into());
        self
    }

    /// Set the [`Stdio`] that the script will use as its
    /// standard input stream.
    ///
    /// NOTE: This defaults to [`Stdio::null`], which is
    /// appropriate when running lifecycle scripts, but regular run-scripts
    /// and such cases can use [`Stdio::inherit`].
    pub fn stdin(mut self, stdin: impl Into<Stdio>) -> Self {
        self.cmd.stdin(stdin.into());
        self
    }

    /// Execute script, collecting all its output.
    pub fn output(self) -> Result<Output> {
        self.set_all_paths()?
            .set_script()?
            .cmd
            .output()
            .map_err(OroScriptError::ScriptProcessError)
            .and_then(|out| {
                if out.status.success() {
                    Ok(out)
                } else {
                    Err(OroScriptError::ScriptError(
                        out.status,
                        Some(out.stdout),
                        Some(out.stderr),
                    ))
                }
            })
    }

    /// Spawn script as a child process.
    pub fn spawn(self) -> Result<ScriptChild> {
        self.set_all_paths()?
            .set_script()?
            .cmd
            .spawn()
            .map(ScriptChild::new)
            .map_err(OroScriptError::SpawnError)
    }

    fn set_script(mut self) -> Result<Self> {
        let event = &self.event;
        if let Some(pkg) = self.manifest {
            let script = pkg
                .scripts
                .get(event)
                .ok_or_else(|| OroScriptError::MissingEvent(event.to_string()))?;
            tracing::trace!(
                "Executing script for event '{event}' for package at {}: {script}",
                self.package_path.display()
            );
            #[cfg(windows)]
            {
                use std::os::windows::process::CommandExt;
                self.cmd.raw_arg(script);
            }
            #[cfg(not(windows))]
            self.cmd.arg(script);
        } else {
            let package_path = &self.package_path;
            let pkg = BuildManifest::from_path(package_path.join("package.json"))?;
            let script = pkg
                .scripts
                .get(event)
                .ok_or_else(|| OroScriptError::MissingEvent(event.to_string()))?;
            tracing::trace!(
                "Executing script for event '{event}' for package at {}: {script}",
                self.package_path.display()
            );
            #[cfg(windows)]
            {
                use std::os::windows::process::CommandExt;
                self.cmd.raw_arg(script);
            }
            #[cfg(not(windows))]
            self.cmd.arg(script);
        }
        Ok(self)
    }

    fn set_all_paths(mut self) -> Result<Self> {
        for dir in self.package_path.ancestors() {
            self.paths
                .push(dir.join("node_modules").join(".bin").to_path_buf());
            if let Some(workspace_path) = &self.workspace_path {
                if dir == workspace_path {
                    break;
                }
            }
        }
        let paths = format!("{}", std::env::join_paths(&self.paths)?.to_string_lossy());
        for (var, _) in Self::current_paths() {
            self = self.env(format!("{}", var.to_string_lossy()), paths.clone());
        }
        Ok(self)
    }

    fn current_paths() -> impl Iterator<Item = (OsString, Vec<PathBuf>)> {
        std::env::vars_os().filter_map(|(var, val)| {
            if var.to_string_lossy().to_lowercase() == "path" {
                Some((var, std::env::split_paths(&val).collect::<Vec<PathBuf>>()))
            } else {
                None
            }
        })
    }

    fn get_existing_paths() -> Vec<PathBuf> {
        Self::current_paths()
            .map(|(_, paths)| paths)
            .reduce(|mut a, mut b| {
                a.append(&mut b);
                a
            })
            .unwrap_or_default()
    }
}

/// Child process executing a script.
pub struct ScriptChild {
    child: Child,
    pub stdin: Option<ChildStdin>,
    pub stdout: Option<ChildStdout>,
    pub stderr: Option<ChildStderr>,
}

impl ScriptChild {
    fn new(mut child: Child) -> Self {
        Self {
            stdin: child.stdin.take(),
            stdout: child.stdout.take(),
            stderr: child.stderr.take(),
            child,
        }
    }

    /// Returns the OS-assigned process identifier associated with this child.
    pub fn id(&self) -> u32 {
        self.child.id()
    }

    /// Forces the script process to exit.
    pub fn kill(mut self) -> Result<()> {
        self.child
            .kill()
            .map_err(OroScriptError::ScriptProcessError)
    }

    /// Waits for the script to exit completely. If the script exits with a
    /// non-zero status, [`OroScriptError::ScriptError`] is returned.
    pub fn wait(mut self) -> Result<()> {
        self.child
            .wait()
            .map_err(OroScriptError::ScriptProcessError)
            .and_then(|status| {
                if status.success() {
                    Ok(())
                } else {
                    Err(OroScriptError::ScriptError(status, None, None))
                }
            })
    }
}
