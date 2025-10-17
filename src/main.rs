use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Arc;

use anyhow::{Context, Result};
use cursive::traits::{Nameable, Resizable};
use cursive::views::{Dialog, EditView, LinearLayout, SelectView, TextView};
use cursive::{Cursive, CursiveExt};
use serde::{Deserialize, Serialize};

fn main() {
    let mut siv = Cursive::default();

    match load_config() {
        Ok(cfg) => {
            let config = SharedConfig::new(cfg);
            show_root_screen(&mut siv, config);
        }
        Err(_) => {
            show_initial_config_prompt_with_retry(&mut siv);
        }
    }

    siv.run();
}

fn is_inside_project() -> bool {
    Path::new("Cargo.toml").exists()
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct Config {
    project_dir: String,
    editor_cmd: String,
}

#[derive(Clone, Debug)]
struct SharedConfig(Arc<Config>);

impl SharedConfig {
    fn new(cfg: Config) -> Self {
        Self(Arc::new(cfg))
    }
    fn project_dir(&self) -> &str {
        &self.0.project_dir
    }
    fn editor_cmd(&self) -> &str {
        &self.0.editor_cmd
    }
}

fn config_file_path() -> Result<PathBuf> {
    let base = dirs::config_dir().context("Could not determine config directory")?;
    Ok(base.join("rustm").join("config.yaml"))
}

fn load_config() -> Result<Config> {
    let path = config_file_path()?;
    if !path.exists() {
        anyhow::bail!("Config file not found");
    }
    let data = fs::read_to_string(&path).context("Failed reading config file")?;
    let cfg: Config = serde_yaml::from_str(&data).context("Failed parsing config yaml")?;
    if cfg.project_dir.trim().is_empty() || cfg.editor_cmd.trim().is_empty() {
        anyhow::bail!("Config file contains empty required values");
    }
    Ok(cfg)
}

fn save_config(cfg: &Config) -> Result<()> {
    let path = config_file_path()?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let yaml = serde_yaml::to_string(cfg)?;
    fs::write(path, yaml)?;
    Ok(())
}

fn show_initial_config_prompt_with_retry(siv: &mut Cursive) {
    siv.add_layer(
        Dialog::around(
            LinearLayout::vertical()
                .child(TextView::new("Initial configuration required"))
                .child(TextView::new("Project root directory:"))
                .child(EditView::new().with_name("cfg_project_dir").fixed_width(50))
                .child(TextView::new("Editor command (e.g. code, nvim, subl):"))
                .child(EditView::new().with_name("cfg_editor_cmd").fixed_width(50)),
        )
        .title("Initial Configuration")
        .button("Save", |s| {
            let project_dir = s
                .call_on_name("cfg_project_dir", |v: &mut EditView| v.get_content())
                .unwrap();
            let editor_cmd = s
                .call_on_name("cfg_editor_cmd", |v: &mut EditView| v.get_content())
                .unwrap();

            if project_dir.is_empty() || editor_cmd.is_empty() {
                s.add_layer(
                    Dialog::text("Both fields are required. Program will exit.")
                        .title("Error")
                        .button("Ok", cursive::Cursive::quit),
                );
                return;
            }

            let new_cfg = Config {
                project_dir: project_dir.to_string(),
                editor_cmd: editor_cmd.to_string(),
            };

            if let Err(e) = save_config(&new_cfg) {
                s.add_layer(
                    Dialog::text(format!("Failed to save config: {e}"))
                        .title("Error")
                        .button("Ok", cursive::Cursive::quit),
                );
                return;
            }

            // Retry load
            match load_config() {
                Ok(loaded) => {
                    s.pop_layer();
                    show_root_screen(s, SharedConfig::new(loaded));
                }
                Err(e) => {
                    s.add_layer(
                        Dialog::text(format!("Failed to load config after saving: {e}\nExiting."))
                            .title("Error")
                            .button("Ok", cursive::Cursive::quit),
                    );
                }
            }
        })
        .button("Quit", Cursive::quit),
    );
}

fn show_outside_menu(siv: &mut Cursive, cfg: SharedConfig) {
    let cfg_for_create = cfg.clone();
    let cfg_for_inside = cfg;
    siv.add_layer(
        Dialog::text("Outside project mode.\nSelect an action.")
            .title("rustm")
            .button("Create new project", move |s| {
                show_create_project_form(s, cfg_for_create.clone());
            })
            .button("Inside mode", move |s| {
                if is_inside_project() {
                    clear_layers(s);
                    let cfg_local = cfg_for_inside.clone();
                    s.add_layer(
                        Dialog::text("Inside project mode (stub)\nFeatures coming soon.")
                            .title("rustm")
                            .button("Outside mode", move |s2| {
                                clear_layers(s2);
                                show_outside_menu(s2, cfg_local.clone());
                            })
                            .button("Quit", Cursive::quit),
                    );
                } else {
                    s.add_layer(Dialog::info(
                        "Not inside a Rust project (Cargo.toml missing).",
                    ));
                }
            })
            .button("Quit", Cursive::quit),
    );
}

fn show_create_project_form(siv: &mut Cursive, cfg: SharedConfig) {
    let mut edition_select = SelectView::<String>::new().popup();
    for ed in ["2015", "2018", "2021", "2024"] {
        edition_select.add_item(format!("Edition {ed}"), ed.to_string());
    }
    edition_select.set_selection(3); // Default to latest

    let mut type_select = SelectView::<String>::new().popup();
    type_select.add_item("Binary (default)", "binary".to_string());
    type_select.add_item("Library", "library".to_string());
    type_select.set_selection(0);

    let cfg_for_create = cfg;

    siv.add_layer(
        Dialog::around(
            LinearLayout::vertical()
                .child(TextView::new("Create new Rust project"))
                .child(TextView::new("Name:"))
                .child(EditView::new().with_name("proj_name").fixed_width(40))
                .child(TextView::new("Type:"))
                .child(type_select.with_name("proj_type"))
                .child(TextView::new("Edition:"))
                .child(edition_select.with_name("proj_edition")),
        )
        .title("New Project")
        .button("Create", move |s| {
            let name = s
                .call_on_name("proj_name", |v: &mut EditView| v.get_content())
                .unwrap();
            let ptype = s
                .call_on_name("proj_type", |v: &mut SelectView<String>| {
                    v.selection()
                        .map_or_else(|| "binary".to_string(), |sel| sel.as_ref().to_string())
                })
                .unwrap();
            let edition = s
                .call_on_name("proj_edition", |v: &mut SelectView<String>| {
                    v.selection()
                        .map_or_else(|| "2024".to_string(), |sel| sel.as_ref().to_string())
                })
                .unwrap();

            if name.is_empty() {
                s.add_layer(Dialog::info("Project name cannot be empty."));
                return;
            }

            let root = cfg_for_create.project_dir();
            if root.is_empty() {
                s.add_layer(Dialog::info("Project directory not set in config."));
                return;
            }

            let project_path = Path::new(root).join(&*name);
            if project_path.exists() {
                s.add_layer(Dialog::info("Target directory already exists."));
                return;
            }

            match create_project(&project_path, &ptype, &edition) {
                Ok(()) => {
                    s.pop_layer(); // remove form
                    let editor_cmd = cfg_for_create.editor_cmd().to_owned();
                    s.add_layer(
                        Dialog::text(format!(
                            "Project '{name}' created at {}. Open in editor?",
                            project_path.display()
                        ))
                        .title("Success")
                        .button("Open", move |s2| {
                            if editor_cmd.is_empty() {
                                s2.add_layer(Dialog::info("Editor command not configured."));
                            } else if let Err(e) = open_in_editor(&editor_cmd, &project_path) {
                                s2.add_layer(Dialog::info(format!("Failed to open editor: {e}")));
                            }
                        })
                        .button("Done", |s2| {
                            s2.pop_layer();
                        }),
                    );
                }
                Err(e) => {
                    s.add_layer(Dialog::info(format!("Failed to create project: {e}")));
                }
            }
        })
        .button("Cancel", |s| {
            s.pop_layer();
        }),
    );
}

fn show_root_screen(siv: &mut Cursive, cfg: SharedConfig) {
    if is_inside_project() {
        let cfg_inside = cfg;
        siv.add_layer(
            Dialog::text("Inside project mode (stub)\nFeatures coming soon.")
                .title("rustm")
                .button("Outside mode", move |s| {
                    clear_layers(s);
                    show_outside_menu(s, cfg_inside.clone());
                })
                .button("Quit", Cursive::quit),
        );
    } else {
        show_outside_menu(siv, cfg);
    }
}

fn clear_layers(s: &mut Cursive) {
    while s.pop_layer().is_some() {}
}

fn create_project(path: &Path, ptype: &str, edition: &str) -> Result<()> {
    let mut cmd = Command::new("cargo");
    cmd.arg("new");
    if ptype == "library" {
        cmd.arg("--lib");
    }
    cmd.arg("--edition").arg(edition);
    cmd.arg(path.file_name().unwrap().to_string_lossy().to_string());
    cmd.current_dir(path.parent().unwrap());
    let output = cmd.output().context("Failed to spawn cargo new")?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("cargo new failed: {stderr}");
    }
    Ok(())
}

fn open_in_editor(editor_cmd: &str, project_path: &Path) -> Result<()> {
    let mut parts = editor_cmd.split_whitespace();
    let cmd = parts.next().context("Empty editor command")?;
    let args: Vec<String> = parts.map(ToString::to_string).collect();

    let status = Command::new(cmd)
        .args(args)
        .arg(project_path)
        .status()
        .context("Failed to launch editor")?;

    if !status.success() {
        anyhow::bail!("Editor command exited with non-zero status");
    }
    Ok(())
}
