mod config;
mod project;

use std::{fmt::Display, path::PathBuf};

use anyhow::bail;
use log::warn;

use crate::{
    config::{Config, RawConfig},
    project::Project,
};

fn dir() -> anyhow::Result<PathBuf> {
    Ok(dirs::config_dir()
        .ok_or_else(|| anyhow::anyhow!("Could not determine config directory"))?
        .join("rustm"))
}

pub enum Mode {
    Global {
        projects_path: PathBuf,
        projects: Vec<Project>,
        current_project: Option<Project>,
        list_projects: bool,
    },
    Project {
        project: Project,
    },
}

impl Mode {
    pub fn load_global(projects_path: PathBuf) -> anyhow::Result<Self> {
        let projects = Project::all_in(&projects_path)?;
        Ok(Self::Global {
            projects_path,
            projects,
            current_project: Project::current()?,
            list_projects: false,
        })
    }

    pub fn from_env(config: Config) -> anyhow::Result<Self> {
        Project::current()?.map_or_else(
            || Self::load_global(config.projects_dir().to_owned()),
            |project| Ok(Self::Project { project }),
        )
    }

    pub fn switch_to_global(&mut self, config: Config) -> anyhow::Result<()> {
        if matches!(self, Self::Global { .. }) {
            warn!("Switching to global mode while already in global mode");
        }
        *self = Self::load_global(config.projects_dir().to_owned())?;
        Ok(())
    }

    pub fn switch_to_project(&mut self) -> anyhow::Result<()> {
        match self {
            Self::Global {
                current_project, ..
            } => {
                if let Some(current_project) = current_project.take() {
                    *self = Self::Project {
                        project: current_project,
                    };
                } else {
                    bail!("No current project to switch to project mode");
                }
            }
            Self::Project { .. } => {
                warn!("Switching to project mode while already in project mode");
            }
        }
        Ok(())
    }
}

fn init_config() -> anyhow::Result<Config> {
    let mut raw_config = RawConfig::check()?;

    if raw_config.projects_dir.is_none() {
        let projects_dir: String = inquire::Text::new("Enter projects directory:").prompt()?;
        raw_config.projects_dir = Some(projects_dir);
    }

    if raw_config.editor_cmd.is_none() {
        let editor_cmd: String = inquire::Text::new("Enter editor command:").prompt()?;
        raw_config.editor_cmd = Some(editor_cmd);
    }

    raw_config.save()?;

    Config::load()
}

fn main() -> anyhow::Result<()> {
    simplelog::WriteLogger::init(
        simplelog::LevelFilter::Warn,
        simplelog::Config::default(),
        std::fs::File::create(dir()?.join("rustm.log"))?,
    )?;

    let config = init_config()?;

    let mut mode = Mode::from_env(config.clone())?;
    loop {
        match mode {
            Mode::Global {
                ref projects_path,
                ref current_project,
                ref projects,
                ref mut list_projects,
            } => {
                if *list_projects {
                    enum ProjectListOption<'a> {
                        SelectProject(&'a str),
                        Back,
                    }

                    impl Display for ProjectListOption<'_> {
                        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                            match self {
                                Self::SelectProject(project_name) => {
                                    write!(f, "{project_name}")
                                }
                                Self::Back => write!(f, "Back"),
                            }
                        }
                    }

                    let mut prompts = projects
                        .iter()
                        .map(|project| ProjectListOption::SelectProject(project.name.as_str()))
                        .collect::<Vec<_>>();

                    prompts.push(ProjectListOption::Back);

                    let prompt_response = inquire::Select::new("Projects", prompts).prompt()?;

                    match prompt_response {
                        ProjectListOption::SelectProject(project) => println!("{project} selected"),
                        ProjectListOption::Back => *list_projects = false,
                    }
                } else {
                    enum GlobalModeOption<'a> {
                        ListProjects,
                        CurrentProject(&'a str),
                        Exit,
                    }

                    impl Display for GlobalModeOption<'_> {
                        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                            match self {
                                Self::ListProjects => write!(f, "List Projects"),
                                Self::CurrentProject(current_project) => {
                                    write!(f, "Current Project ({current_project})")
                                }
                                Self::Exit => write!(f, "Exit"),
                            }
                        }
                    }

                    let mut prompts = Vec::new();
                    prompts.push(GlobalModeOption::ListProjects);

                    if let Some(current_project) = current_project {
                        prompts.push(GlobalModeOption::CurrentProject(
                            current_project.name.as_str(),
                        ));
                    }

                    prompts.push(GlobalModeOption::Exit);

                    let prompt_response =
                        inquire::Select::new("Choose action", prompts).prompt()?;
                    match prompt_response {
                        GlobalModeOption::ListProjects => {
                            *list_projects = true;
                        }
                        GlobalModeOption::CurrentProject(_) => mode.switch_to_project()?,
                        GlobalModeOption::Exit => return Ok(()),
                    }
                }
            }
            Mode::Project { .. } => {
                enum ProjectModeOption {
                    GlobalMode,
                    Exit,
                }

                impl Display for ProjectModeOption {
                    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                        match self {
                            Self::GlobalMode => write!(f, "Global Mode"),
                            Self::Exit => write!(f, "Exit"),
                        }
                    }
                }

                let prompts = vec![ProjectModeOption::GlobalMode, ProjectModeOption::Exit];

                let prompt_response = inquire::Select::new("Project mode", prompts).prompt()?;
                match prompt_response {
                    ProjectModeOption::GlobalMode => mode.switch_to_global(config.clone())?,
                    ProjectModeOption::Exit => return Ok(()),
                }
            }
        }
    }
}
