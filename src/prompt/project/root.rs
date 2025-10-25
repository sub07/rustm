use std::fmt::Display;

pub enum SelectOption {
    DependencyList,
    GlobalMode,
    Exit,
}

impl Display for SelectOption {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::DependencyList => write!(f, "Dependencies"),
            Self::GlobalMode => write!(f, "Global Mode"),
            Self::Exit => write!(f, "Exit"),
        }
    }
}

pub fn prompt() -> anyhow::Result<SelectOption> {
    let prompts = vec![
        SelectOption::DependencyList,
        SelectOption::GlobalMode,
        SelectOption::Exit,
    ];

    Ok(inquire::Select::new("Choose action", prompts).prompt()?)
}
