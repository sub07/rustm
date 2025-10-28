use std::fmt::Display;

use itertools::Itertools;

pub enum SelectOption {
    Dep { name: String },
    Back,
}

impl Display for SelectOption {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Dep { name, .. } => write!(f, "{name}"),

            Self::Back => write!(f, "Back"),
        }
    }
}

pub fn prompt(deps: Vec<String>) -> anyhow::Result<SelectOption> {
    let mut options = deps
        .into_iter()
        .map(|name| SelectOption::Dep { name })
        .collect_vec();

    options.push(SelectOption::Back);

    Ok(
        inquire::Select::new("Select a dependency to inspect", options)
            .with_page_size(50)
            .prompt()?,
    )
}
