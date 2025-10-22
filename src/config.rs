use std::{
    env,
    fs::{self, File},
    io::Write,
    path::{Path, PathBuf},
    process::Command,
    sync::Arc,
};

use anyhow::{Context, ensure};
use log::{debug, info, warn};
use serde::{Deserialize, Serialize};

use crate::dir;

#[derive(Clone)]
pub struct Config {
    inner: Arc<ConfigInner>,
}

struct ConfigInner {
    projects_dir: PathBuf,
    editor_cmd: String,
}

#[derive(Default, Debug, Serialize, Deserialize)]
pub struct RawConfig {
    pub projects_dir: Option<String>,
    pub editor_cmd: Option<String>,
}

impl Config {
    pub fn load() -> anyhow::Result<Self> {
        let raw_config = RawConfig::check()?;
        raw_config.validate()?;
        raw_config.validate_if_exists()?;
        Ok(Self {
            inner: Arc::new(ConfigInner {
                projects_dir: PathBuf::from(
                    raw_config
                        .projects_dir
                        .expect("projects_dir should be set after validation"),
                ),
                editor_cmd: raw_config
                    .editor_cmd
                    .expect("editor_cmd should be set after validation"),
            }),
        })
    }

    pub fn projects_dir(&self) -> &Path {
        &self.inner.projects_dir
    }

    pub fn editor_cmd(&self) -> &str {
        &self.inner.editor_cmd
    }
}

impl RawConfig {
    pub fn check() -> anyhow::Result<Self> {
        let config_file_path = config_file_path()?;
        if !config_file_path.exists() {
            info!("Config file does not exist");
            return Ok(Self::default());
        }

        let config_file_content = fs::read_to_string(&config_file_path).with_context(|| {
            format!(
                "Config file might be corrupted or inaccessible. Try to delete it at {}",
                config_file_path.display()
            )
        })?;

        info!("Config file read:\n{config_file_content}");

        let raw_config = serde_norway::from_str::<Self>(&config_file_content);
        let mut raw_config = match raw_config {
            Ok(c) => c,
            Err(err) => {
                warn!(
                    "Could not parse config file at {} ({}):\n{}. Deleting corrupted config file.",
                    config_file_path.display(),
                    err,
                    config_file_content,
                );
                fs::remove_file(&config_file_path).with_context(|| {
                    format!(
                        "Failed to delete corrupted config file at {}",
                        config_file_path.display()
                    )
                })?;
                return Ok(Self::default());
            }
        };

        info!("Config file parsed:\n{raw_config:#?}");

        if let Err(err) = raw_config.validate_projects_dir_if_exists() {
            warn!(
                "Invalid projects_dir in config file at {} ({}):\n{}. Removing value for reprompt",
                config_file_path.display(),
                err,
                raw_config
                    .projects_dir
                    .expect("Can only fail validation if Some"),
            );
            raw_config.projects_dir = None;
        }

        if let Err(err) = raw_config.validate_editor_cmd_if_exists() {
            warn!(
                "Invalid editor_cmd in config file at {} ({}):\n{}. Removing value for reprompt",
                config_file_path.display(),
                err,
                raw_config
                    .editor_cmd
                    .expect("Can only fail validation if Some"),
            );
            raw_config.editor_cmd = None;
        }

        debug!("Returning raw config: {raw_config:#?}");

        Ok(raw_config)
    }

    /// Validate the configuration when writing.
    fn validate(&self) -> anyhow::Result<()> {
        macro_rules! ensure_valid_string {
            ($t:ident) => {
                ensure!(
                    self.$t.as_ref().is_some_and(|s| !s.trim().is_empty()),
                    concat!(stringify!($t), " is empty")
                );
            };
        }

        ensure_valid_string!(projects_dir);
        ensure_valid_string!(editor_cmd);

        Ok(())
    }

    /// Validate the configuration when reading.
    fn validate_if_exists(&self) -> anyhow::Result<()> {
        self.validate_projects_dir_if_exists()?;
        self.validate_editor_cmd_if_exists()?;
        Ok(())
    }

    fn validate_projects_dir_if_exists(&self) -> anyhow::Result<()> {
        if let Some(projects_dir) = self.projects_dir.as_deref() {
            ensure!(
                !projects_dir.trim().is_empty(),
                "projects_dir cannot be an empty string"
            );
            let path = PathBuf::from(projects_dir);
            ensure!(
                path.exists(),
                "projects_dir '{projects_dir}' does not exist"
            );
            ensure!(
                path.is_dir(),
                "projects_dir '{projects_dir}' is not a directory"
            );
        }
        Ok(())
    }

    fn validate_editor_cmd_if_exists(&self) -> anyhow::Result<()> {
        if let Some(editor_cmd) = self.editor_cmd.as_deref() {
            ensure!(
                !editor_cmd.trim().is_empty(),
                "editor_cmd cannot be an empty string"
            );

            if env::var("RUSTM_SKIP_EDITOR_CHECK").ok().is_none() {
                let mut editor_process = Command::new(editor_cmd)
                    .arg("--version")
                    .spawn()
                    .with_context(|| {
                        format!(
                            "Failed to execute editor command '{editor_cmd} --version'. Ensure the editor is installed and accessible in your PATH."
                        )
                    })?;
                let _ = editor_process.kill();
            } else {
                info!(
                    "Skipping editor command validation due to RUSTM_SKIP_EDITOR_CHECK being set"
                );
            }
        }
        Ok(())
    }

    pub fn save(&self) -> anyhow::Result<()> {
        self.validate()?;
        self.validate_if_exists()?;
        let config_file_path = config_file_path()?;
        let serialized = serde_norway::to_string(&self)
            .with_context(|| "Failed to serialize configuration to YAML format")?;
        let mut file = File::create(&config_file_path).with_context(|| {
            format!(
                "Failed to create config file at {}",
                config_file_path.display()
            )
        })?;
        file.write_all(serialized.as_bytes()).with_context(|| {
            format!(
                "Failed to write to config file at {}",
                config_file_path.display()
            )
        })?;
        Ok(())
    }
}

fn config_file_path() -> anyhow::Result<PathBuf> {
    Ok(dir()?.join("config.yaml"))
}
