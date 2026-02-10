use std::{
    path::{Path, PathBuf},
    result::Result,
};

use anyhow::{anyhow, ensure};
use cargo_toml::{Dependency, Manifest};
use itertools::Itertools;
use joy_error::ResultLogExt;
use log::info;

pub enum ProjectType {
    Binary,
    Library,
}

/// Represents the kind of Cargo project detected
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProjectKind {
    /// A workspace root (has [workspace] section in Cargo.toml)
    Workspace,
    /// A workspace member (has Cargo.toml and is inside a workspace)
    WorkspaceMember,
    /// A standalone project (not part of a workspace)
    Standalone,
}

/// Actions available for a project based on its kind
#[derive(Debug, Clone)]
pub struct AvailableActions {
    /// Can manage dependencies
    pub can_manage_dependencies: bool,
    /// Can list/add workspace members
    pub can_manage_members: bool,
    /// Can run workspace-level commands
    pub can_run_workspace_commands: bool,
    /// Can open individual members
    pub can_open_members: bool,
    /// Can navigate back to workspace root
    pub can_open_workspace_root: bool,
}

impl AvailableActions {
    pub fn for_kind(kind: &ProjectKind) -> Self {
        match kind {
            ProjectKind::Workspace => Self {
                can_manage_dependencies: false, // Workspace root typically doesn't have dependencies
                can_manage_members: true,
                can_run_workspace_commands: true,
                can_open_members: true,
                can_open_workspace_root: false,
            },
            ProjectKind::WorkspaceMember => Self {
                can_manage_dependencies: true,
                can_manage_members: false,
                can_run_workspace_commands: false,
                can_open_members: false,
                can_open_workspace_root: true,
            },
            ProjectKind::Standalone => Self {
                can_manage_dependencies: true,
                can_manage_members: false,
                can_run_workspace_commands: false,
                can_open_members: false,
                can_open_workspace_root: false,
            },
        }
    }

    /// Get a list of available actions as strings
    pub fn list(&self) -> Vec<&str> {
        let mut actions = Vec::new();
        if self.can_manage_dependencies {
            actions.push("Manage dependencies");
        }
        if self.can_manage_members {
            actions.push("Manage workspace members");
        }
        if self.can_run_workspace_commands {
            actions.push("Run workspace commands");
        }
        if self.can_open_members {
            actions.push("Open workspace members");
        }
        if self.can_open_workspace_root {
            actions.push("Open workspace root");
        }
        actions
    }
}

pub struct Project {
    pub name: String,
    pub path: PathBuf,
    pub kind: ProjectKind,
    pub available_actions: AvailableActions,
}

impl Project {
    pub fn create(
        name: &str,
        project_type: &ProjectType,
        project_dir: &Path,
    ) -> anyhow::Result<Self> {
        let mut cargo_cmd = std::process::Command::new("cargo");
        cargo_cmd.current_dir(project_dir);
        cargo_cmd.arg("new");
        cargo_cmd.arg(name);
        match project_type {
            ProjectType::Binary => cargo_cmd.arg("--bin"),
            ProjectType::Library => cargo_cmd.arg("--lib"),
        };
        info!("Creating new cargo project: {cargo_cmd:?}");
        ensure!(
            cargo_cmd.status()?.success(),
            "Could not create cargo project"
        );
        Self::from_path(project_dir.join(name))?
            .ok_or_else(|| anyhow!("Project {} should exist at {}", name, project_dir.display()))
    }

    pub fn all_in(path: &Path) -> anyhow::Result<Vec<Self>> {
        ensure!(path.is_dir());
        let projects = path
            .read_dir()?
            .filter_map(Result::ok)
            .filter_map(|e| e.file_type().ok().zip(Some(e)))
            .filter_map(|(file_type, entry)| file_type.is_dir().then_some(entry))
            .filter_map(|entry| Self::from_path(entry.path()).log_ok().flatten())
            .collect_vec();
        Ok(projects)
    }

    pub fn from_path<P: AsRef<Path>>(path: P) -> anyhow::Result<Option<Self>> {
        let path = path.as_ref();
        let cargo_toml_path = path.join("Cargo.toml");
        if cargo_toml_path.exists() {
            let name = path
                .file_name()
                .ok_or_else(|| anyhow!("Could not extract dir name: {}", path.display()))?
                .to_str()
                .ok_or_else(|| anyhow!("Invalid dir name: {}", path.display()))?
                .to_owned();

            // Detect project kind by analyzing the manifest
            let kind = Self::detect_kind(path)?;
            let available_actions = AvailableActions::for_kind(&kind);

            Ok(Some(Self {
                name,
                path: path.to_owned(),
                kind,
                available_actions,
            }))
        } else {
            Ok(None)
        }
    }

    /// Detects the kind of project at the given path
    /// This is workspace-first: we check for workspace first, then check if we're in a workspace
    fn detect_kind(path: &Path) -> anyhow::Result<ProjectKind> {
        let cargo_toml_path = path.join("Cargo.toml");
        let manifest = Manifest::from_path(&cargo_toml_path)?;

        // First check: is this a workspace root?
        if manifest.workspace.is_some() {
            return Ok(ProjectKind::Workspace);
        }

        // Second check: are we inside a workspace?
        // Walk up the directory tree looking for a workspace root
        let mut current = path.parent();
        while let Some(parent) = current {
            let parent_cargo_toml = parent.join("Cargo.toml");
            if parent_cargo_toml.exists() {
                if let Ok(parent_manifest) = Manifest::from_path(&parent_cargo_toml) {
                    if parent_manifest.workspace.is_some() {
                        // We're a member of a workspace
                        return Ok(ProjectKind::WorkspaceMember);
                    }
                }
            }
            current = parent.parent();
        }

        // Not a workspace and not in a workspace
        Ok(ProjectKind::Standalone)
    }

    pub fn current() -> anyhow::Result<Option<Self>> {
        let current_dir = std::env::current_dir()?;
        Self::from_path(&current_dir)
    }

    pub fn manifest(&self) -> anyhow::Result<Manifest> {
        let cargo_toml_path = self.path.join("Cargo.toml");
        Ok(Manifest::from_path(&cargo_toml_path)?)
    }

    pub fn dep(&self, dep_name: &str) -> anyhow::Result<Dependency> {
        let manifest = self.manifest()?;
        manifest
            .dependencies
            .get(dep_name)
            .cloned()
            .ok_or_else(|| anyhow!("Dependency {} not found in project {}", dep_name, self.name))
    }

    pub fn open_in_editor(&self, editor_cmd: &str) -> anyhow::Result<()> {
        let mut cmd = std::process::Command::new(editor_cmd);
        cmd.arg(&self.path);
        info!("Opening project in editor: {cmd:?}");
        ensure!(cmd.status()?.success(), "Could not open project in editor");
        Ok(())
    }

    /// Get workspace members if this is a workspace
    pub fn workspace_members(&self) -> anyhow::Result<Vec<Self>> {
        if self.kind != ProjectKind::Workspace {
            return Ok(Vec::new());
        }

        let manifest = self.manifest()?;
        let workspace = manifest
            .workspace
            .ok_or_else(|| anyhow!("Expected workspace in manifest"))?;

        let mut members = Vec::new();
        for member_glob in workspace.members {
            let member_path = self.path.join(&member_glob);

            // First try direct path (no glob pattern)
            if member_path.exists() && member_path.is_dir() {
                if let Some(project) = Self::from_path(&member_path)? {
                    members.push(project);
                }
            } else {
                // Handle glob patterns using the glob crate
                let glob_pattern = member_path
                    .to_str()
                    .ok_or_else(|| anyhow!("Invalid path in workspace member: {}", member_glob))?;

                for entry in glob::glob(glob_pattern)? {
                    let path = entry?;
                    if path.is_dir() {
                        if let Some(project) = Self::from_path(&path)? {
                            members.push(project);
                        }
                    }
                }
            }
        }

        Ok(members)
    }

    /// Get the workspace root if this is a workspace member
    pub fn workspace_root(&self) -> anyhow::Result<Option<Self>> {
        if self.kind != ProjectKind::WorkspaceMember {
            return Ok(None);
        }

        let mut current = self.path.parent();
        while let Some(parent) = current {
            let parent_cargo_toml = parent.join("Cargo.toml");
            if parent_cargo_toml.exists() {
                if let Ok(parent_manifest) = Manifest::from_path(&parent_cargo_toml) {
                    if parent_manifest.workspace.is_some() {
                        return Self::from_path(parent);
                    }
                }
            }
            current = parent.parent();
        }

        Ok(None)
    }

    /// Check if workspace features should be hidden (not a workspace)
    pub fn hide_workspace_features(&self) -> bool {
        self.kind != ProjectKind::Workspace
    }

    /// Get a human-readable description of the project kind
    pub fn kind_description(&self) -> &str {
        match self.kind {
            ProjectKind::Workspace => "Workspace",
            ProjectKind::WorkspaceMember => "Workspace Member",
            ProjectKind::Standalone => "Standalone Project",
        }
    }
}
