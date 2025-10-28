mod config;
mod crate_api;
mod crate_data;
mod logger;
mod manifest_editor;
mod project;
mod prompt;
mod state;
mod state_handler;

use std::path::PathBuf;

use anyhow::Context;

use crate::{
    config::{Config, RawConfig},
    state::View,
};

fn dir() -> anyhow::Result<PathBuf> {
    Ok(dirs::config_dir()
        .ok_or_else(|| anyhow::anyhow!("Could not determine config directory"))?
        .join("rustm"))
}

fn init_dir() -> anyhow::Result<()> {
    let dir = dir()?;
    if !dir.exists() {
        std::fs::create_dir_all(&dir)
            .with_context(|| format!("Could not create rustm directory at {}", dir.display()))?;
    }
    Ok(())
}

fn init_config() -> anyhow::Result<Config> {
    let mut raw_config = RawConfig::check()?;

    if raw_config.projects_dir.is_none() {
        let projects_dir: String = prompt::config::project_dirs()?;
        raw_config.projects_dir = Some(projects_dir);
    }

    if raw_config.editor_cmd.is_none() {
        let editor_cmd: String = prompt::config::editor_cmd()?;
        raw_config.editor_cmd = Some(editor_cmd);
    }
    raw_config.save()?;

    Config::load()
}
fn main() -> anyhow::Result<()> {
    init_dir()?;
    logger::init()?;

    let config = init_config()?;
    let crate_api = crate_api::Client::new()?;

    let mut view = View::Initial;

    macro_rules! handle {
        ($h:ident $(, $args:expr )* ) => {
            match crate::state_handler::$h($($args),*) {
                crate::state_handler::ControlFlow::UpdateView(v) => {
                    view = v;
                }
                crate::state_handler::ControlFlow::Exit => return Ok(()),
            }
        };
    }

    loop {
        match view {
            View::Initial => handle!(initial),
            View::Global => handle!(global),
            View::ProjectList => handle!(project_list, &config),
            View::Project(project) => handle!(project, &config, project),
            View::NewProject => handle!(new_project, &config),
            View::ProjectDependencyList(project) => {
                handle!(project_dependency_list, project, &crate_api);
            }
            View::ProjectDependencyDetail(project, crate_data) => {
                handle!(project_dependency_detail, project, crate_data);
            }
            View::ProjectDependencyFeatureToggle(project, crate_data, dependency) => {
                handle!(
                    project_dependency_feature_toggle,
                    project,
                    crate_data,
                    &dependency
                );
            }
        }
    }
}
