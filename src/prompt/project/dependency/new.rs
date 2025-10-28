use std::fmt::Display;

use anyhow::anyhow;
use inquire::validator::{ErrorMessage, Validation};
use itertools::Itertools;

pub fn prompt_search_query() -> anyhow::Result<Option<String>> {
    let search_query = inquire::Text::new("Enter the name of the dependency to add")
        .with_placeholder("e.g., serde")
        .with_validator(|search_query: &str| {
            if search_query.trim().is_empty() {
                Ok(Validation::Invalid(ErrorMessage::Custom(
                    "The search query should not be empty".into(),
                )))
            } else {
                Ok(Validation::Valid)
            }
        })
        .prompt();

    match search_query {
        Ok(res) => Ok(Some(res.trim().to_string())),
        Err(e) => match e {
            inquire::InquireError::OperationCanceled => Ok(None),
            _ => Err(anyhow!(e)),
        },
    }
}

pub struct CrateOption {
    pub name: String,
    pub description: String,
    pub version: String,
    pub downloads: u64,
}

pub enum PaginationOption {
    Crate(CrateOption),
    NextPage,
    PreviousPage,
    Back,
}

impl Display for PaginationOption {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Crate(crate_option) => {
                write!(
                    f,
                    "{} (v{}) - {} - {} downloads",
                    crate_option.name,
                    crate_option.version,
                    crate_option.description,
                    crate_option.downloads
                )
            }
            Self::NextPage => write!(f, "Next Page"),
            Self::PreviousPage => write!(f, "Previous Page"),
            Self::Back => write!(f, "Back"),
        }
    }
}

pub fn prompt_paginated_select(
    res: Vec<CrateOption>,
    is_first_page: bool,
    is_last_page: bool,
) -> anyhow::Result<PaginationOption> {
    let mut options = res.into_iter().map(PaginationOption::Crate).collect_vec();

    if !is_last_page {
        options.push(PaginationOption::NextPage);
    }
    if !is_first_page {
        options.push(PaginationOption::PreviousPage);
    }

    options.push(PaginationOption::Back);

    let page_size = options.len();

    Ok(
        inquire::Select::new("Select a crate to add as dependency", options)
            .with_page_size(page_size)
            .prompt()?,
    )
}

pub fn prompt_confirmation_for_editing_feature() -> anyhow::Result<bool> {
    Ok(inquire::prompt_confirmation(
        "Would you like to edit features now?",
    )?)
}
