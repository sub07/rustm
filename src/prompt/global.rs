pub mod root {
    use std::fmt::Display;

    use crate::project::Project;

    pub enum SelectOption {
        NewProject,
        ListProjects,
        CurrentProject(Project),
        Exit,
    }

    impl Display for SelectOption {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            match self {
                Self::NewProject => write!(f, "Create project"),
                Self::ListProjects => write!(f, "List Projects"),
                Self::CurrentProject(current_project) => {
                    write!(f, "Switch to current Project ({})", current_project.name)
                }
                Self::Exit => write!(f, "Exit"),
            }
        }
    }

    pub fn prompt() -> anyhow::Result<SelectOption> {
        let mut prompts = vec![SelectOption::ListProjects, SelectOption::NewProject];

        if let Some(current_project) = Project::current()? {
            prompts.push(SelectOption::CurrentProject(current_project));
        }

        prompts.push(SelectOption::Exit);

        Ok(inquire::Select::new("Choose action", prompts).prompt()?)
    }
}

pub mod project_list {
    use std::{fmt::Display, path::Path};

    use crate::project::Project;

    pub enum SelectOption {
        SelectProject(Project),
        Back,
    }

    impl Display for SelectOption {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            match self {
                Self::SelectProject(project) => {
                    write!(f, "{}", project.name)
                }
                Self::Back => write!(f, "Back"),
            }
        }
    }

    pub fn prompt(projects_dir: &Path) -> anyhow::Result<SelectOption> {
        let mut prompts = Project::all_in(projects_dir)?
            .into_iter()
            .map(SelectOption::SelectProject)
            .collect::<Vec<_>>();
        prompts.push(SelectOption::Back);

        Ok(inquire::Select::new("Projects", prompts).prompt()?)
    }
}
