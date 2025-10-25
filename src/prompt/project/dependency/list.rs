use std::fmt::Display;

use cargo_toml::Dependency;
use itertools::Itertools;

use crate::project::Project;

pub enum SelectOption {
    Dep {
        name: String,
        is_crate_io: bool,
        dep: Dependency,
    },
    Back,
}

impl Display for SelectOption {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Dep {
                name, is_crate_io, ..
            } => {
                if *is_crate_io {
                    write!(f, "{name} (crates.io)")
                } else {
                    write!(f, "{name} (unsupported source)")
                }
            }
            Self::Back => write!(f, "Back"),
        }
    }
}

pub fn prompt(project: &Project) -> anyhow::Result<SelectOption> {
    let manifest = project.manifest()?;
    let mut options = manifest
        .dependencies
        .into_iter()
        .map(|(name, dep)| SelectOption::Dep {
            name,
            is_crate_io: dep.is_crates_io(),
            dep,
        })
        .collect_vec();

    options.push(SelectOption::Back);

    Ok(
        inquire::Select::new("Select a dependency to inspect", options)
            .with_page_size(50)
            .prompt()?,
    )
}
