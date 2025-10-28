use std::path::PathBuf;

use anyhow::Context;

use crate::dir;

fn log_file_path() -> anyhow::Result<PathBuf> {
    Ok(dir()?.join("rustm.log"))
}

pub fn init() -> anyhow::Result<()> {
    let log_file_path = log_file_path()?;

    Ok(simplelog::WriteLogger::init(
        simplelog::LevelFilter::Debug,
        simplelog::ConfigBuilder::new()
            .set_time_format_rfc3339()
            .set_time_offset_to_local()
            .unwrap_or_else(|builder| builder)
            .build(),
        std::fs::File::create(log_file_path.clone()).with_context(|| {
            format!(
                "Could not create / open log file ({})",
                log_file_path.display(),
            )
        })?,
    )?)
}
