use std::fmt::Display;

pub enum SelectOption {
    DependencyList,
    AddCrate,
    OpenWithEditor,
    RestoreManifest,
    GlobalMode,
    Exit,
}

impl Display for SelectOption {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::DependencyList => write!(f, "Dependencies"),
            Self::AddCrate => write!(f, "Add new crate !WIP!"),
            Self::OpenWithEditor => write!(f, "Open project in editor"),
            Self::RestoreManifest => write!(f, "Restore Cargo.toml from backup"),
            Self::GlobalMode => write!(f, "Global Mode"),
            Self::Exit => write!(f, "Exit"),
        }
    }
}

pub fn prompt() -> anyhow::Result<SelectOption> {
    let prompts = vec![
        SelectOption::DependencyList,
        SelectOption::AddCrate,
        SelectOption::OpenWithEditor,
        SelectOption::RestoreManifest,
        SelectOption::GlobalMode,
        SelectOption::Exit,
    ];

    Ok(inquire::Select::new("Choose action", prompts).prompt()?)
}
