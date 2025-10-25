use std::{
    path::{Path, PathBuf},
    result::Result,
};

use anyhow::{anyhow, ensure};
use cargo_toml::Manifest;
use itertools::Itertools;
use joy_error::ResultLogExt;
use log::info;

pub enum ProjectType {
    Binary,
    Library,
}

pub struct Project {
    pub name: String,
    pub path: PathBuf,
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

            Ok(Some(Self {
                name,
                path: path.to_owned(),
            }))
        } else {
            Ok(None)
        }
    }

    pub fn current() -> anyhow::Result<Option<Self>> {
        let current_dir = std::env::current_dir()?;
        Self::from_path(&current_dir)
    }

    pub fn manifest(&self) -> anyhow::Result<Manifest> {
        let cargo_toml_path = self.path.join("Cargo.toml");
        Ok(Manifest::from_path(&cargo_toml_path)?)
    }

    pub fn open_in_editor(&self, editor_cmd: &str) -> anyhow::Result<()> {
        let mut cmd = std::process::Command::new(editor_cmd);
        cmd.arg(&self.path);
        info!("Opening project in editor: {cmd:?}");
        ensure!(cmd.status()?.success(), "Could not open project in editor");
        Ok(())
    }
}
