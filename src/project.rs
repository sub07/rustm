use std::path::{Path, PathBuf};

use anyhow::{anyhow, ensure};
use itertools::Itertools;
use joy_error::ResultLogExt;

pub struct Project {
    pub name: String,
    pub path: PathBuf,
}

impl Project {
    pub fn all_in(path: &Path) -> anyhow::Result<Vec<Self>> {
        ensure!(path.is_dir());
        let projects = path
            .read_dir()?
            .filter_map(|e| e.ok())
            .filter_map(|e| e.file_type().ok().zip(Some(e)))
            .filter_map(|(file_type, entry)| file_type.is_dir().then_some(entry))
            .filter_map(|entry| Project::from_path(&entry.path()).log_ok().flatten())
            .collect_vec();
        Ok(projects)
    }

    pub fn from_path(path: &Path) -> anyhow::Result<Option<Self>> {
        if path.join("Cargo.toml").exists() {
            let name = path
                .file_name()
                .ok_or_else(|| anyhow!("Could not extract dir name: {}", path.display()))?
                .to_str()
                .ok_or_else(|| anyhow!("Invalid dir name: {}", path.display()))?
                .to_owned();
            Ok(Some(Project {
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
}
