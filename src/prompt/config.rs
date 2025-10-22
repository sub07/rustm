pub fn project_dirs() -> anyhow::Result<String> {
    Ok(inquire::Text::new("Enter projects directory:").prompt()?)
}

pub fn editor_cmd() -> anyhow::Result<String> {
    Ok(inquire::Text::new("Enter editor command:").prompt()?)
}
