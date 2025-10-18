//! Project creation feature (spec: feature 0001).
//!
//! This module encapsulates the logic for creating a new Rust project based on user
//! input collected by the TUI layer. It is deliberately UI-agnostic and returns rich
//! error types so the caller can decide how to present issues.
//!
//! Steps performed (aligned with spec):
//! 1. Validate supplied parameters (name format, edition, project type).
//! 2. Re‑validate the configured projects directory (existence, permissions).
//! 3. Ensure the target project path does NOT already exist.
//! 4. Set `git config --global init.defaultBranch main` (best effort; warn on failure).
//! 5. Invoke `cargo new` with the chosen edition and type.
//! 6. (Optional) Open the project in the configured editor command.
//!
//! Logging:
//! - Significant actions are logged at INFO.
//! - Failures are logged at ERROR or WARN (non-fatal steps).
//!
//! Integration guidance:
//! - The TUI should build a `CreateProjectParams` from user inputs (applying defaults
//!   when fields are omitted).
//! - Call `create_project(&config, params)`.
//! - If `open_in_editor` flag is chosen by the user, call `maybe_open_in_editor` on
//!   the returned result (or use `create_and_optionally_open` helper).
//!
//! NOTE: Editor command tokenization here is intentionally simple (whitespace split)
//! to avoid adding parsing dependencies. If you need robust shell-like parsing, you
//! can introduce a crate (e.g. `shlex`) and adjust the implementation accordingly.

use std::fmt;
use std::path::{Path, PathBuf};
use std::process::Command;

use log::{error, info, warn};

use crate::config::{Config, validate_projects_directory};

/// Supported project types (maps to `cargo new --bin/--lib`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProjectType {
    Binary,
    Library,
}

impl ProjectType {
    const fn cargo_flag(self) -> &'static str {
        match self {
            Self::Binary => "--bin",
            Self::Library => "--lib",
        }
    }
}

/// Supported Rust editions the UI can offer.
/// (Spec: 2015, 2018, 2021, 2024 with default = latest stable (2024).)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProjectEdition {
    E2015,
    E2018,
    E2021,
    E2024,
}

impl ProjectEdition {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::E2015 => "2015",
            Self::E2018 => "2018",
            Self::E2021 => "2021",
            Self::E2024 => "2024",
        }
    }
}

impl Default for ProjectEdition {
    fn default() -> Self {
        Self::E2024
    }
}

impl Default for ProjectType {
    fn default() -> Self {
        Self::Binary
    }
}

/// Parameters provided by the caller (TUI) to create a project.
#[derive(Debug, Clone)]
pub struct CreateProjectParams {
    pub name: String,
    pub project_type: ProjectType,
    pub edition: ProjectEdition,
}

impl CreateProjectParams {
    /// Build with defaults (binary, 2024) for convenience.
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            project_type: ProjectType::default(),
            edition: ProjectEdition::default(),
        }
    }
}

/// Result structure describing a successfully created project.
#[derive(Debug, Clone)]
pub struct CreateProjectResult {
    pub project_path: PathBuf,
    pub params: CreateProjectParams,
}

impl CreateProjectResult {
    /// Attempt to open the project in the configured editor.
    pub fn maybe_open_in_editor(&self, config: &Config) -> Result<(), OpenEditorError> {
        open_in_editor(config.editor_cmd(), &self.project_path)
    }
}
/// Error category for project creation failures.
#[derive(Debug)]
pub enum CreateProjectError {
    InvalidName(String),
    ProjectsDirInvalid(String),
    AlreadyExists(PathBuf),
    CargoNotFound,
    CargoFailed { status: i32, stderr: String },
    Io(std::io::Error),
}

impl fmt::Display for CreateProjectError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidName(n) => {
                write!(f, "Invalid project name '{n}'")
            }
            Self::ProjectsDirInvalid(msg) => {
                write!(f, "Projects directory invalid: {msg}")
            }
            Self::AlreadyExists(p) => {
                write!(f, "Target directory already exists: {}", p.display())
            }
            Self::CargoNotFound => {
                write!(f, "Unable to locate `cargo` in PATH")
            }
            Self::CargoFailed { status, stderr } => {
                write!(f, "`cargo new` failed (exit code {status}): {stderr}")
            }
            Self::Io(e) => write!(f, "I/O error: {e}"),
        }
    }
}

impl std::error::Error for CreateProjectError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io(e) => Some(e),
            _ => None,
        }
    }
}

impl From<std::io::Error> for CreateProjectError {
    fn from(e: std::io::Error) -> Self {
        Self::Io(e)
    }
}

/// Error category for editor opening failures.
#[derive(Debug)]
pub enum OpenEditorError {
    EditorCommandEmpty,
    Spawn(std::io::Error),
    Failed(i32),
}

impl fmt::Display for OpenEditorError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EditorCommandEmpty => write!(f, "Editor command is empty"),
            Self::Spawn(e) => write!(f, "Failed to spawn editor command: {e}"),
            Self::Failed(code) => write!(f, "Editor command exited with status {code}"),
        }
    }
}

impl std::error::Error for OpenEditorError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Spawn(e) => Some(e),
            _ => None,
        }
    }
}

/// Main entry point: create a new Rust project.
///
/// This does not open the project in the editor. Use `CreateProjectResult::maybe_open_in_editor`
/// or `create_and_optionally_open` for that workflow.
pub fn create_project(
    config: &Config,
    params: CreateProjectParams,
) -> Result<CreateProjectResult, CreateProjectError> {
    info!(
        "Starting project creation: name='{}', type={:?}, edition={}",
        params.name,
        params.project_type,
        params.edition.as_str()
    );

    validate_name(&params.name).map_err(CreateProjectError::InvalidName)?;

    // Ensure projects directory still valid (defense in depth).
    if let Err(e) = validate_projects_directory(Path::new(config.projects_directory())) {
        return Err(CreateProjectError::ProjectsDirInvalid(e.to_string()));
    }

    let project_path = Path::new(config.projects_directory()).join(&params.name);

    if project_path.exists() {
        return Err(CreateProjectError::AlreadyExists(project_path));
    }

    // Best effort: configure git default branch.
    set_global_git_default_branch();

    // Run cargo new
    run_cargo_new(&project_path, &params).map_err(|e| {
        error!("cargo new failed: {e}");
        e
    })?;

    info!("Project successfully created at {}", project_path.display());

    Ok(CreateProjectResult {
        project_path,
        params,
    })
}

/// Convenience function: create and optionally open the project in the editor
/// depending on the `open_in_editor` flag.
///
/// If opening fails, the creation result is still returned inside the Err(OpenAfterCreate).
pub fn create_and_optionally_open(
    config: &Config,
    params: CreateProjectParams,
    open_in_editor: bool,
) -> Result<CreateProjectResult, CreateAndOpenError> {
    let result = create_project(config, params).map_err(CreateAndOpenError::CreateFailed)?;

    if let Err(e) = result.maybe_open_in_editor(config)
        && open_in_editor
    {
        return Err(CreateAndOpenError::OpenAfterCreate { result, error: e });
    }
    Ok(result)
}

/// Composite error for `create_and_optionally_open`.
#[derive(Debug)]
pub enum CreateAndOpenError {
    CreateFailed(CreateProjectError),
    OpenAfterCreate {
        result: CreateProjectResult,
        error: OpenEditorError,
    },
}

impl fmt::Display for CreateAndOpenError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::CreateFailed(e) => write!(f, "Project creation failed: {e}"),
            Self::OpenAfterCreate { error, .. } => {
                write!(f, "Project created but failed to open editor: {error}")
            }
        }
    }
}

impl std::error::Error for CreateAndOpenError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::CreateFailed(e) => Some(e),
            Self::OpenAfterCreate { error, .. } => Some(error),
        }
    }
}

/// Validate crate / project name (simple heuristic).
fn validate_name(name: &str) -> Result<(), String> {
    if name.trim().is_empty() {
        return Err("name cannot be blank".into());
    }
    if name.chars().any(char::is_whitespace) {
        return Err("name cannot contain whitespace".into());
    }
    // Basic crate name pattern (letters/numbers/_/- and must start with letter)
    let first = name.chars().next().unwrap();
    if !first.is_ascii_alphabetic() {
        return Err("name must start with an ASCII alphabetic character".into());
    }
    if !name
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
    {
        return Err("name can only contain ASCII alphanumeric, '_' or '-'".into());
    }
    Ok(())
}

/// Attempt to set global git default branch, logging warnings on failure.
fn set_global_git_default_branch() {
    match Command::new("git")
        .args(["config", "--global", "init.defaultBranch", "main"])
        .status()
    {
        Ok(status) => {
            if status.success() {
                info!("Ensured global git default branch is 'main'");
            } else {
                warn!(
                    "git config command exited with non-zero status: {:?}",
                    status.code()
                );
            }
        }
        Err(e) => {
            warn!("Unable to run git to set default branch: {e}");
        }
    }
}

/// Run `cargo new` to create the project directory.
fn run_cargo_new(
    project_path: &Path,
    params: &CreateProjectParams,
) -> Result<(), CreateProjectError> {
    let mut cmd = Command::new("cargo");
    cmd.arg("new")
        .arg(params.project_type.cargo_flag())
        .arg("--edition")
        .arg(params.edition.as_str())
        .arg(&params.name)
        .current_dir(
            project_path
                .parent()
                .expect("project path should have parent"),
        );

    info!("Executing: {cmd:?}");

    let output = cmd.output().map_err(|e| {
        if e.kind() == std::io::ErrorKind::NotFound {
            CreateProjectError::CargoNotFound
        } else {
            CreateProjectError::Io(e)
        }
    })?;

    if !output.status.success() {
        let code = output.status.code().unwrap_or(-1);
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        return Err(CreateProjectError::CargoFailed {
            status: code,
            stderr,
        });
    }

    Ok(())
}

/// Open the project in the provided editor command (string).
///
/// Strategy:
/// - Split editor command by ASCII whitespace (basic, not shell quoting aware).
/// - First token is program, remainder are args.
/// - Append the project directory path.
/// - Spawn and wait (blocking). If asynchronous desired, adapt logic accordingly.
fn open_in_editor(editor_cmd: &str, project_path: &Path) -> Result<(), OpenEditorError> {
    if editor_cmd.trim().is_empty() {
        return Err(OpenEditorError::EditorCommandEmpty);
    }

    let mut parts = editor_cmd.split_whitespace();
    let program = parts.next().ok_or(OpenEditorError::EditorCommandEmpty)?;
    let mut cmd = Command::new(program);
    for arg in parts {
        cmd.arg(arg);
    }
    cmd.arg(project_path);

    info!(
        "Opening project '{}' with editor command: {}",
        project_path.display(),
        editor_cmd
    );

    let status = cmd.status().map_err(OpenEditorError::Spawn)?;

    if !status.success() {
        return Err(OpenEditorError::Failed(status.code().unwrap_or(-1)));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn name_validation_ok() {
        assert!(validate_name("my_crate").is_ok());
        assert!(validate_name("crate2").is_ok());
        assert!(validate_name("my-crate").is_ok());
    }

    #[test]
    fn name_validation_failures() {
        assert!(validate_name("").is_err());
        assert!(validate_name("  ").is_err());
        assert!(validate_name("9start").is_err());
        assert!(validate_name("bad name").is_err());
        assert!(validate_name("bad*char").is_err());
    }

    #[test]
    fn defaults_applied() {
        let p = CreateProjectParams::new("abc");
        assert_eq!(p.project_type, ProjectType::Binary);
        assert_eq!(p.edition, ProjectEdition::E2024);
    }
}
