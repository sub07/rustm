use std::fs::{self, OpenOptions};
use std::path::{Path, PathBuf};
use std::sync::Once;

use log::{LevelFilter, info};
use simplelog::{Config as LogConfig, ConfigBuilder, WriteLogger};

use crate::config::Config; // For deriving the config directory path.

// One–time initialization guard.
static INIT: Once = Once::new();

/// Initialize the application logging subsystem.
///
/// Spec (updated):
/// - Use the standard configuration directory (same directory as `config.yaml`) for the log file.
/// - Log file name: `rustm.log`.
/// - No rotation (rotation requirement removed).
/// - In debug builds (`cfg(debug_assertions)`) log ALL levels (Trace).
/// - In release builds log everything >= INFO.
/// - Must be safe / idempotent to call multiple times (subsequent calls are no-ops).
///
/// Returns:
/// - Ok(true)  => logger was initialized this call.
/// - Ok(false) => logger had already been initialized (no change).
/// - Err(e)    => an error creating the log file or setting logger (very first call only).
pub fn init_logging() -> Result<bool, InitLogError> {
    let mut result: Result<(), InitLogError> = Ok(());

    let mut initialized = false;
    INIT.call_once(|| match real_init() {
        Ok(()) => {
            initialized = true;
        }
        Err(e) => {
            result = Err(e);
        }
    });

    match result {
        Ok(()) => Ok(initialized),
        Err(e) => Err(e),
    }
}

/// Filtering logger that excludes records whose target starts with `cursive_core`.
struct FilteringLogger {
    inner: Box<dyn log::Log>,
}

impl log::Log for FilteringLogger {
    fn enabled(&self, metadata: &log::Metadata) -> bool {
        if metadata.target().starts_with("cursive_core") {
            return false;
        }

        self.inner.enabled(metadata)
    }
    fn log(&self, record: &log::Record) {
        if self.enabled(record.metadata()) {
            self.inner.log(record);
        }
    }
    fn flush(&self) {
        self.inner.flush();
    }
}

/// Perform the actual logger setup (invoked only once).
fn real_init() -> Result<(), InitLogError> {
    let log_path = log_file_path();

    if let Some(parent) = log_path.parent() {
        fs::create_dir_all(parent).map_err(InitLogError::Io)?;
    }

    let file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log_path)
        .map_err(InitLogError::Io)?;

    let mut builder = ConfigBuilder::new();
    builder.set_time_level(LevelFilter::Error); // Remove time granularity spam (Error => effectively disabled timestamp).

    if builder.set_time_offset_to_local().is_err() {

        // If determining local offset fails, proceed without it.
    }
    let log_cfg: LogConfig = builder.build();

    let level = if cfg!(debug_assertions) {
        LevelFilter::Trace
    } else {
        LevelFilter::Info
    };

    let inner = WriteLogger::new(level, log_cfg, file);

    let inner: Box<dyn log::Log> = inner;
    let filtering = FilteringLogger { inner };

    log::set_boxed_logger(Box::new(filtering))
        .map_err(|e| InitLogError::SetLogger(e.to_string()))?;

    log::set_max_level(level);

    info!("Logger initialized at {}", log_path.display());

    Ok(())
}

/// Determine the log file path: same directory as `config.yaml`.
fn log_file_path() -> PathBuf {
    let cfg_file = Config::file_path();
    cfg_file
        .parent()
        .map_or_else(Config::file_path, Path::to_path_buf)
        .join("rustm.log")
}

/// Errors that can occur during logging initialization.
#[derive(Debug)]
pub enum InitLogError {
    Io(std::io::Error),
    SetLogger(String),
}

impl std::fmt::Display for InitLogError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(e) => write!(f, "I/O error initializing logger: {e}"),
            Self::SetLogger(e) => write!(f, "Failed to set global logger: {e}"),
        }
    }
}

impl std::error::Error for InitLogError {}

#[cfg(test)]
mod tests {
    use super::*;
    use log::{error, trace};

    #[test]
    fn init_is_idempotent() {
        let first = init_logging().expect("first init should succeed");
        let second = init_logging().expect("second init should not fail");
        // first call either initialized or we raced with another test (should be true in isolation)
        // second call must be false (no re-init)
        assert!(first);
        assert!(!second);
        trace!("trace after init");
        error!("error after init");
    }
}
