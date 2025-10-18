use std::fs;
use std::path::{Path, PathBuf};

use crate::config::{Config, validate_projects_directory};
use git2::{Repository, StatusOptions};
use log::{info, warn};

/// Information about a discovered Rust project.
#[derive(Debug, Clone)]
pub struct ProjectInfo {
    /// Directory name (project name).
    pub name: String,
    /// Full absolute path to the project directory.
    pub path: PathBuf,
    /// Simple indicator: does the repository have any uncommitted changes?
    pub has_uncommitted_changes: bool,
}
/// Errors that may occur while listing projects.
#[derive(Debug)]
pub enum ListProjectsError {
    /// The configured projects directory failed validation.
    ProjectsDirInvalid(String),
    /// I/O error when scanning filesystem.
    Io(std::io::Error),
}

impl std::fmt::Display for ListProjectsError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ProjectsDirInvalid(msg) => {
                write!(f, "Projects directory invalid: {msg}")
            }
            Self::Io(e) => write!(f, "I/O error listing projects: {e}"),
        }
    }
}

impl std::error::Error for ListProjectsError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io(e) => Some(e),
            Self::ProjectsDirInvalid(_) => None,
        }
    }
}

impl From<std::io::Error> for ListProjectsError {
    fn from(e: std::io::Error) -> Self {
        Self::Io(e)
    }
}

/// List all Rust projects in the configured projects directory.
///
/// Rules (per spec):
/// - A "Rust project" is any immediate subdirectory containing a `Cargo.toml`.
/// - Include all such directories, even if not a git repository.
/// - Provide indicator `*` (represented here by `has_uncommitted_changes = true`)
///   when repo has uncommitted changes.
/// - If git-related checks fail for a given project, log and treat as non-git or clean.
///
/// Returns projects sorted by name (case-insensitive).
pub fn list_projects(config: &Config) -> Result<Vec<ProjectInfo>, ListProjectsError> {
    let root = Path::new(config.projects_directory());

    if let Err(e) = validate_projects_directory(root) {
        return Err(ListProjectsError::ProjectsDirInvalid(e.to_string()));
    }

    info!("Listing Rust projects in {}", root.display());

    let mut projects = Vec::new();

    for entry_res in fs::read_dir(root)? {
        let entry = match entry_res {
            Ok(e) => e,
            Err(e) => {
                warn!("Skipping entry due to read_dir error: {e}");
                continue;
            }
        };

        let path = entry.path();
        let file_type = match entry.file_type() {
            Ok(t) => t,
            Err(e) => {
                warn!("Skipping {:?} (file_type error: {e})", path.display());
                continue;
            }
        };

        if !file_type.is_dir() {
            continue;
        }

        let cargo_toml = path.join("Cargo.toml");
        if !cargo_toml.is_file() {
            continue;
        }

        let name = path
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or_default()
            .to_string();

        // Determine git status if applicable.
        let has_uncommitted_changes = match scan_git_status(&path) {
            Ok(res) => res,
            Err(e) => {
                // Log and degrade gracefully.
                warn!("Git status check failed for {}: {e}", path.display());
                false
            }
        };

        projects.push(ProjectInfo {
            name,
            path,
            has_uncommitted_changes,
        });
    }

    // Sort by lowercased name to provide deterministic order.
    projects.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    Ok(projects)
}

/// Internal helper: examine a directory for git status.
///
/// Returns `true` if `dir` is a Git repository that has any uncommitted (including untracked) changes; otherwise returns `false`.
fn scan_git_status(dir: &Path) -> Result<bool, git2::Error> {
    // Quick existence check for .git to reduce error noise.
    if !dir.join(".git").exists() {
        return Ok(false);
    }

    let repo = Repository::open(dir)?;
    let mut opts = StatusOptions::new();
    // Include untracked changes for a more accurate "dirty" indicator.
    opts.include_untracked(true)
        .recurse_untracked_dirs(true)
        .renames_head_to_index(true);

    let statuses = repo.statuses(Some(&mut opts))?;
    let dirty = statuses.iter().any(|s| {
        let st = s.status();
        // Any status bit that indicates differences counts.
        st.intersects(
            git2::Status::INDEX_NEW
                | git2::Status::INDEX_MODIFIED
                | git2::Status::INDEX_DELETED
                | git2::Status::INDEX_RENAMED
                | git2::Status::INDEX_TYPECHANGE
                | git2::Status::WT_NEW
                | git2::Status::WT_MODIFIED
                | git2::Status::WT_DELETED
                | git2::Status::WT_TYPECHANGE
                | git2::Status::WT_RENAMED
                | git2::Status::CONFLICTED,
        )
    });

    Ok(dirty)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_dir() -> PathBuf {
        let mut d = std::env::temp_dir();
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        d.push(format!("rustm_list_projects_test_{nonce}"));
        fs::create_dir_all(&d).unwrap();
        d
    }

    // Minimal in-memory Config substitute for tests (we only need its accessor).
    struct DummyConfig {
        dir: String,
    }
    impl DummyConfig {
        fn new(dir: String) -> Self {
            Self { dir }
        }
    }
    impl DummyConfig {
        fn as_config_like(&self) -> TestConfigLike<'_> {
            TestConfigLike { dir: &self.dir }
        }
    }
    struct TestConfigLike<'a> {
        dir: &'a str,
    }
    impl TestConfigLike<'_> {
        fn projects_directory(&self) -> &str {
            self.dir
        }
    }

    // Adapter so we can reuse list_projects logic with a fake config.
    // (We don't want to pull full real Config in unit tests.)
    fn list_with_fake(config_like: &TestConfigLike) -> Result<Vec<ProjectInfo>, ListProjectsError> {
        // Inline duplicate of list_projects first lines (subset) to avoid coupling to real Config.
        let root = Path::new(config_like.projects_directory());
        if !root.exists() {
            return Err(ListProjectsError::ProjectsDirInvalid(
                "does not exist".into(),
            ));
        }
        let mut projects = Vec::new();
        for entry_res in fs::read_dir(root)? {
            let entry = entry_res?;
            let path = entry.path();
            if !entry.file_type()?.is_dir() {
                continue;
            }
            if !path.join("Cargo.toml").is_file() {
                continue;
            }
            let name = path.file_name().unwrap().to_string_lossy().into_owned();
            let has_uncommitted_changes = scan_git_status(&path).unwrap_or(false);
            projects.push(ProjectInfo {
                name,
                path,
                has_uncommitted_changes,
            });
        }
        projects.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
        Ok(projects)
    }

    #[test]
    fn lists_simple_projects() {
        let base = temp_dir();

        // project1 (non-git)
        let p1 = base.join("project1");
        fs::create_dir(&p1).unwrap();
        fs::write(
            p1.join("Cargo.toml"),
            b"[package]\nname='project1'\nversion='0.1.0'\nedition='2021'\n",
        )
        .unwrap();

        // project2 (git dirty)
        let p2 = base.join("project2");
        fs::create_dir(&p2).unwrap();
        fs::write(
            p2.join("Cargo.toml"),
            b"[package]\nname='project2'\nversion='0.1.0'\nedition='2021'\n",
        )
        .unwrap();
        // Create an untracked file
        let mut f = fs::File::create(p2.join("src_lib.rs")).unwrap();
        write!(f, "pub fn x() -> i32 {{ 1 }}").unwrap();
        // Do not add/commit to keep it untracked (dirty)

        let cfg = DummyConfig::new(base.to_string_lossy().into_owned());
        let list = list_with_fake(&cfg.as_config_like()).unwrap();

        assert_eq!(list.len(), 2);
        let p2i = list.iter().find(|p| p.name == "project2").unwrap();
        assert!(p2i.has_uncommitted_changes); // Should detect untracked file
    }
}
