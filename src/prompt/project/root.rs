use std::fmt::Display;

use crate::project::Project;

pub enum SelectOption {
    DependencyList,
    AddCrate,
    OpenWithEditor,
    WorkspaceRoot,
    RestoreManifest,
    GlobalMode,
    Exit,
}

impl Display for SelectOption {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::DependencyList => write!(f, "Dependencies"),
            Self::AddCrate => write!(f, "Add new crate"),
            Self::OpenWithEditor => write!(f, "Open project in editor"),
            Self::WorkspaceRoot => write!(f, "Open workspace root"),
            Self::RestoreManifest => write!(f, "Restore Cargo.toml from backup"),
            Self::GlobalMode => write!(f, "Global Mode"),
            Self::Exit => write!(f, "Exit"),
        }
    }
}

pub fn prompt(project: &Project) -> anyhow::Result<SelectOption> {
    let mut prompts = vec![
        SelectOption::AddCrate,
        SelectOption::DependencyList,
        SelectOption::OpenWithEditor,
    ];

    // Add workspace root option for workspace members
    if project.available_actions.can_open_workspace_root {
        prompts.push(SelectOption::WorkspaceRoot);
    }

    prompts.extend(vec![
        SelectOption::RestoreManifest,
        SelectOption::GlobalMode,
        SelectOption::Exit,
    ]);

    Ok(inquire::Select::new("Choose action", prompts).prompt()?)
}
