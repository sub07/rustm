pub mod root {
    use std::fmt::Display;

    pub enum SelectOption {
        GlobalMode,
        Exit,
    }

    impl Display for SelectOption {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            match self {
                Self::GlobalMode => write!(f, "Global Mode"),
                Self::Exit => write!(f, "Exit"),
            }
        }
    }

    pub fn prompt() -> anyhow::Result<SelectOption> {
        let prompts = vec![SelectOption::GlobalMode, SelectOption::Exit];

        Ok(inquire::Select::new("Choose action", prompts).prompt()?)
    }
}

pub mod create_new {
    use std::{fmt::Display, path::Path};

    use crate::project::{Project, ProjectType};

    impl Display for ProjectType {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            match self {
                Self::Binary => write!(f, "Binary (application)"),
                Self::Library => write!(f, "Library"),
            }
        }
    }

    pub fn prompt(project_dir: &Path) -> anyhow::Result<Project> {
        let project_name = inquire::Text::new("Project name:").prompt()?;
        let project_type = inquire::Select::new(
            "Project type:",
            vec![ProjectType::Binary, ProjectType::Library],
        )
        .prompt()?;

        Project::create(&project_name, &project_type, project_dir)
    }
}
