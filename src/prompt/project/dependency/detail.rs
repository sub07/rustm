use std::fmt::Display;

pub enum SelectOption {
    Features,
    DisableDefaultFeatures,
    EnableDefaultFeatures,
    RestoreManifest,
    Back,
}

impl Display for SelectOption {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Features => write!(f, "Toggle features"),
            Self::DisableDefaultFeatures => write!(f, "Disable default features"),
            Self::EnableDefaultFeatures => write!(f, "Enable default features"),
            Self::RestoreManifest => write!(f, "Restore manifest backup \u{26A0}"),
            Self::Back => write!(f, "Back"),
        }
    }
}

pub fn prompt(default_features_enabled: bool, has_features: bool) -> anyhow::Result<SelectOption> {
    let mut options = Vec::new();

    if has_features {
        options.push(SelectOption::Features);
    }

    if default_features_enabled {
        options.push(SelectOption::DisableDefaultFeatures);
    } else {
        options.push(SelectOption::EnableDefaultFeatures);
    }

    options.push(SelectOption::RestoreManifest);
    options.push(SelectOption::Back);

    Ok(inquire::Select::new("Choose action", options).prompt()?)
}
