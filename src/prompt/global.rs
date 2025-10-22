use std::fmt::Display;

use crate::project::Project;

pub enum GlobalModeOption {
    ListProjects,
    CurrentProject(Project),
    Exit,
}

impl Display for GlobalModeOption {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ListProjects => write!(f, "List Projects"),
            Self::CurrentProject(current_project) => {
                write!(f, "Switch to current Project ({})", current_project.name)
            }
            Self::Exit => write!(f, "Exit"),
        }
    }
}

pub fn root<'a>() -> anyhow::Result<GlobalModeOption> {
    let mut prompts = Vec::new();
    prompts.push(GlobalModeOption::ListProjects);

    if let Some(current_project) = Project::current()? {
        prompts.push(GlobalModeOption::CurrentProject(current_project));
    }

    prompts.push(GlobalModeOption::Exit);

    Ok(inquire::Select::new("Choose action", prompts).prompt()?)
}
