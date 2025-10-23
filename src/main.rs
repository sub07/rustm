mod config;
mod crate_api;
mod crate_data;
mod log;
mod project;
mod prompt;
mod state;

use std::path::PathBuf;

use anyhow::Context;

use crate::{
    config::{Config, RawConfig},
    project::Project,
    prompt::global::root::SelectOption,
    state::{Action, ViewStateMachine},
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
    log::init()?;

    let config = init_config()?;
    let crate_api = crate_api::Client::new()?;

    let mut view_state = ViewStateMachine::new();

    loop {
        match view_state.state() {
            state::State::Initial => {
                view_state.consume(Action::ChooseProjectOrGlobalMode);
            }
            state::State::ProjectOrGlobalAutomaticDetection => {
                if let Some(project) = Project::current()? {
                    view_state.consume(Action::OpenProjectMode(project));
                } else {
                    view_state.consume(Action::OpenGlobalMode);
                }
            }
            state::State::GlobalView => {
                let response = prompt::global::root::prompt()?;
                match response {
                    SelectOption::NewProject => {
                        view_state.consume(Action::OpenNewProject);
                    }
                    SelectOption::ListProjects => {
                        view_state.consume(Action::OpenProjectList);
                    }
                    SelectOption::CurrentProject(project) => {
                        view_state.consume(Action::OpenProjectMode(project));
                    }
                    SelectOption::Exit => return Ok(()),
                }
            }
            state::State::ProjectListView => {
                let response = prompt::global::project_list::prompt(config.projects_dir())?;
                match response {
                    prompt::global::project_list::SelectOption::SelectProject(project) => {
                        view_state.consume(Action::OpenProjectMode(project));
                    }
                    prompt::global::project_list::SelectOption::Back => {
                        view_state.consume(Action::OpenGlobalMode);
                    }
                }
            }
            state::State::ProjectView(project) => {
                println!("{} [{}]", project.name, project.path.display());
                let response = prompt::project::root::prompt()?;
                match response {
                    prompt::project::root::SelectOption::GlobalMode => {
                        view_state.consume(Action::OpenGlobalMode);
                    }
                    prompt::project::root::SelectOption::Exit => return Ok(()),
                }
            }
            state::State::NewProjectView => {
                let project = prompt::project::create_new::prompt(&config)?;
                println!(
                    "Project {} created at {}",
                    project.name,
                    project.path.display()
                );
                view_state.consume(Action::OpenProjectMode(project));
            }
        }
    }
}
