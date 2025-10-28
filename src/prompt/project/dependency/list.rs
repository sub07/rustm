use std::fmt::Display;

use itertools::Itertools;

pub struct NewVersionInfo {
    pub current_version: String,
    pub latest_version: String,
}

pub struct SelectDep {
    pub name: String,
    pub new_version: Option<NewVersionInfo>,
}

pub enum SelectOption {
    Dep(SelectDep),
    Back,
}

impl Display for SelectOption {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Dep(SelectDep { name, new_version }) => {
                if let Some(new_version) = new_version {
                    write!(
                        f,
                        "{} (new version available: {} -> {})",
                        name, new_version.current_version, new_version.latest_version
                    )
                } else {
                    write!(f, "{name}")
                }
            }
            Self::Back => write!(f, "Back"),
        }
    }
}

pub fn prompt(deps: Vec<SelectDep>) -> anyhow::Result<SelectOption> {
    let mut options: Vec<SelectOption> = deps.into_iter().map(SelectOption::Dep).collect_vec();
    options.push(SelectOption::Back);

    Ok(
        inquire::Select::new("Select a dependency to inspect", options)
            .with_page_size(50)
            .prompt()?,
    )
}
