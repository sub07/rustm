//! Configuration module.
//!
//! Responsibilities (per spec):
//! - Persist required user configuration across runs in a single YAML file (`config.yaml`)
//! - File lives inside the platform standard configuration directory (`dirs::config_dir()`) under an app subdirectory (`rustm`)
//! - Fields are required (no `Option`). Missing file OR missing field => trigger initial setup ( surfaced as `LoadStatus::NeedsInitialSetup` )
//! - Corrupt / invalid YAML => fatal error (`LoadError::Corrupt`)
//! - Provide cheap cloning via Arc
//! - Provide validation for `projects_directory` (exists, is a directory, readable, writable)
//! - Disallow blank / empty strings
//!
//! UI / TUI integration policy (kept decoupled here):
//! - The TUI layer should call `load()`:
//!     * If it gets `Ok(LoadStatus::Ready(config))` -> proceed
//!     * If it gets `Ok(LoadStatus::NeedsInitialSetup(reason))` -> show initial setup screen, then call `Config::create_and_persist(...)`
//!     * If it gets `Err(LoadError::Corrupt(..))` -> show error & exit
//!
//! Saving:
//! - `Config::create_and_persist` validates, writes atomically (write to temp then rename), then returns a new `Config`.
//!
//! YAML backend: `serde_norway` (spec requirement; API-compatible with `serde_yaml`).

use std::{
    fs,
    io::{self, Write},
    path::{Path, PathBuf},
    sync::Arc,
};

use serde::{Deserialize, Serialize};

/// Public handle to configuration (cheap clone).
#[derive(Clone)]
pub struct Config {
    inner: Arc<ConfigInner>,
}

#[derive(Debug, Serialize, Deserialize)]
struct ConfigInner {
    projects_directory: String,
    editor_cmd: String,
}

/// Status returned when attempting to load config from disk.
pub enum LoadStatus {
    /// Fully loaded & validated configuration.
    Ready(Config),
    /// Need to run initial setup (file missing or has missing fields).
    NeedsInitialSetup(SetupReason),
}

/// Reason the setup screen must be displayed.
pub enum SetupReason {
    MissingFile,
    IncompleteData,
}

#[derive(Debug)]
pub enum LoadError {
    /// YAML exists but is syntactically invalid or semantically unacceptable.
    Corrupt(String),

    Io(io::Error),
}

#[derive(Debug)]
pub enum SaveError {
    Io(io::Error),
    Serialize(String),
    Validation(ValidationError),
}

/// Validation errors for user-provided values.
#[derive(Debug)]
pub enum ValidationError {
    EmptyField(&'static str),
    ProjectsDirDoesNotExist(PathBuf),
    ProjectsDirNotDirectory(PathBuf),
    ProjectsDirNotWritable(PathBuf),
    ProjectsDirNotReadable(PathBuf),
}

impl Config {
    /// Attempt to load configuration from disk.
    ///
    /// Returns:
    /// - `Ok(LoadStatus::Ready)` if file exists, parses, and validates
    /// - `Ok(LoadStatus::NeedsInitialSetup)` if file missing OR some field blank
    /// - `Err(LoadError::Corrupt)` if YAML malformed
    /// - `Err(LoadError::Io)` for unexpected I/O problems
    pub fn load() -> Result<LoadStatus, LoadError> {
        let path = config_file_path();

        if !path.exists() {
            return Ok(LoadStatus::NeedsInitialSetup(SetupReason::MissingFile));
        }

        let raw = fs::read_to_string(&path).map_err(LoadError::Io)?;

        match serde_norway::from_str::<ConfigInner>(&raw) {
            Ok(inner) => {
                // Semantic validation (no blank fields, valid directory)
                if inner.projects_directory.trim().is_empty() {
                    return Ok(LoadStatus::NeedsInitialSetup(SetupReason::IncompleteData));
                }
                if inner.editor_cmd.trim().is_empty() {
                    return Ok(LoadStatus::NeedsInitialSetup(SetupReason::IncompleteData));
                }
                // Validate projects directory (if invalid => request setup again; user can correct)
                let pd = PathBuf::from(&inner.projects_directory);
                if let Err(e) = validate_projects_directory(&pd) {
                    let msg = match e {
                        ValidationError::ProjectsDirDoesNotExist(_) => {
                            "projects_directory does not exist"
                        }
                        ValidationError::ProjectsDirNotDirectory(_) => {
                            "projects_directory is not a directory"
                        }
                        ValidationError::ProjectsDirNotWritable(_) => {
                            "projects_directory not writable"
                        }
                        ValidationError::ProjectsDirNotReadable(_) => {
                            "projects_directory not readable"
                        }
                        ValidationError::EmptyField(_) => "projects_directory blank",
                    };
                    log::warn!("Config validation failed: {msg}");
                    return Ok(LoadStatus::NeedsInitialSetup(SetupReason::IncompleteData));
                }

                Ok(LoadStatus::Ready(Self {
                    inner: Arc::new(inner),
                }))
            }
            Err(err) => {
                // Distinguish between YAML syntax errors (fatal) and missing fields.
                let msg = err.to_string();
                if looks_like_missing_field(&msg) {
                    Ok(LoadStatus::NeedsInitialSetup(SetupReason::IncompleteData))
                } else {
                    Err(LoadError::Corrupt(msg))
                }
            }
        }
    }

    /// Create, validate, persist, and return a new Config.
    pub fn create_and_persist(
        projects_directory: impl AsRef<Path>,
        editor_cmd: impl AsRef<str>,
    ) -> Result<Self, SaveError> {
        let projects_directory = projects_directory.as_ref();
        let editor_cmd = editor_cmd.as_ref();

        if editor_cmd.trim().is_empty() {
            return Err(SaveError::Validation(ValidationError::EmptyField(
                "editor_cmd",
            )));
        }
        validate_projects_directory(projects_directory).map_err(SaveError::Validation)?;

        let inner = ConfigInner {
            projects_directory: projects_directory.to_string_lossy().into_owned(),
            editor_cmd: editor_cmd.trim().to_string(),
        };

        let yaml =
            serde_norway::to_string(&inner).map_err(|e| SaveError::Serialize(e.to_string()))?;

        let path = config_file_path();
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(SaveError::Io)?;
        }

        let tmp_path = path.with_extension("yaml.tmp");
        {
            let mut f = fs::File::create(&tmp_path).map_err(SaveError::Io)?;
            f.write_all(yaml.as_bytes()).map_err(SaveError::Io)?;
            f.sync_all().ok();
        }
        fs::rename(&tmp_path, &path).map_err(SaveError::Io)?;

        Ok(Self {
            inner: Arc::new(inner),
        })
    }

    /// Persist current state (validation already assumed correct).
    #[allow(dead_code)]
    pub fn save(&self) -> Result<(), SaveError> {
        validate_projects_directory(Path::new(&self.inner.projects_directory))
            .map_err(SaveError::Validation)?;
        if self.inner.editor_cmd.trim().is_empty() {
            return Err(SaveError::Validation(ValidationError::EmptyField(
                "editor_cmd",
            )));
        }

        let yaml = serde_norway::to_string(&*self.inner)
            .map_err(|e| SaveError::Serialize(e.to_string()))?;

        let path = config_file_path();
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(SaveError::Io)?;
        }
        let tmp_path = path.with_extension("yaml.tmp");
        {
            let mut f = fs::File::create(&tmp_path).map_err(SaveError::Io)?;
            f.write_all(yaml.as_bytes()).map_err(SaveError::Io)?;
            f.sync_all().ok();
        }
        fs::rename(&tmp_path, &path).map_err(SaveError::Io)?;
        Ok(())
    }

    /// Accessor: projects directory (guaranteed non-empty).
    pub fn projects_directory(&self) -> &str {
        &self.inner.projects_directory
    }

    pub fn editor_cmd(&self) -> &str {
        &self.inner.editor_cmd
    }

    /// Path to the on-disk configuration file.
    pub fn file_path() -> PathBuf {
        config_file_path()
    }
}

/// Build canonical path to config.yaml
fn config_file_path() -> PathBuf {
    app_config_dir().join("config.yaml")
}

/// Determine application config directory: `<platform_config_dir>/rustm`
fn app_config_dir() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| fallback_home_join(".config"))
        .join("rustm")
}

/// Fallback: if `config_dir` unavailable, attempt to use HOME; else current directory.
fn fallback_home_join(child: &str) -> PathBuf {
    dirs::home_dir().map_or_else(|| PathBuf::from(".").join(child), |h| h.join(child))
}

/// Validate the projects directory according to spec.
pub fn validate_projects_directory(path: &Path) -> Result<(), ValidationError> {
    if path.as_os_str().is_empty() {
        return Err(ValidationError::EmptyField("projects_directory"));
    }
    if !path.exists() {
        return Err(ValidationError::ProjectsDirDoesNotExist(path.to_path_buf()));
    }
    if !path.is_dir() {
        return Err(ValidationError::ProjectsDirNotDirectory(path.to_path_buf()));
    }

    // Readability check: try to read metadata / list (non-fatal nuance simplified).
    if fs::read_dir(path).is_err() {
        return Err(ValidationError::ProjectsDirNotReadable(path.to_path_buf()));
    }

    // Writability check: create & remove a temp file.
    let probe = path.join(".rustm_write_probe");
    match fs::File::create(&probe) {
        Ok(_) => {
            let _ = fs::remove_file(&probe);
        }
        Err(_) => {
            return Err(ValidationError::ProjectsDirNotWritable(path.to_path_buf()));
        }
    }
    Ok(())
}

/// Heuristic to detect missing-field style serde messages.
fn looks_like_missing_field(msg: &str) -> bool {
    msg.contains("missing field")
}

impl From<io::Error> for LoadError {
    fn from(e: io::Error) -> Self {
        Self::Io(e)
    }
}

impl std::fmt::Display for LoadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Corrupt(s) => write!(f, "Corrupt config YAML: {s}"),
            Self::Io(e) => write!(f, "I/O error loading config: {e}"),
        }
    }
}
impl std::error::Error for LoadError {}

impl std::fmt::Display for ValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::EmptyField(field) => write!(f, "Field '{field}' cannot be empty"),
            Self::ProjectsDirDoesNotExist(p) => {
                write!(f, "Projects directory does not exist: {}", p.display())
            }
            Self::ProjectsDirNotDirectory(p) => {
                write!(f, "Projects directory is not a directory: {}", p.display())
            }
            Self::ProjectsDirNotWritable(p) => {
                write!(f, "Projects directory not writable: {}", p.display())
            }
            Self::ProjectsDirNotReadable(p) => {
                write!(f, "Projects directory not readable: {}", p.display())
            }
        }
    }
}
impl std::error::Error for ValidationError {}

impl std::fmt::Display for SaveError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(e) => write!(f, "I/O error saving config: {e}"),
            Self::Serialize(e) => write!(f, "Serialization error: {e}"),
            Self::Validation(e) => write!(f, "Validation error: {e}"),
        }
    }
}
impl std::error::Error for SaveError {}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_dir() -> PathBuf {
        let mut d = std::env::temp_dir();
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        d.push(format!("rustm_test_{nonce}"));
        fs::create_dir_all(&d).unwrap();
        d
    }

    #[test]
    fn validate_projects_directory_ok() {
        let d = temp_dir();
        assert!(validate_projects_directory(&d).is_ok());
    }

    #[test]
    fn validate_projects_directory_missing() {
        let d = temp_dir().join("nope");
        let e = validate_projects_directory(&d).unwrap_err();
        matches!(e, ValidationError::ProjectsDirDoesNotExist(_));
    }

    #[test]
    fn create_and_persist_roundtrip() {
        let d = temp_dir();
        let cfg = Config::create_and_persist(&d, "code").unwrap();
        assert_eq!(cfg.projects_directory(), d.to_string_lossy());
        assert_eq!(cfg.editor_cmd(), "code");
    }
}
