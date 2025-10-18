# Log

We maintain a log of significant actions performed by the program (e.g. creating a new project, listing projects, opening a project in the editor) to aid traceability and troubleshooting.

We use the `log` crate together with `simplelog` (WriteLogger) writing to a single file `rustm.log`.

Location: the log file lives in the same configuration directory as `config.yaml` (`<platform_config_dir>/rustm/rustm.log`), resolved via `dirs::config_dir()`. No separate platform log dir is used to keep operational artifacts co-located.

`cursive_core` should not log to this file; only application-level events are recorded.

Rotation: removed (no size-based rotation). The file simply grows; future optimization can introduce rotation if required.

Levels:

- Release builds: log all events with level >= INFO (INFO, WARN, ERROR).
- Debug builds (cfg(debug_assertions)): log all levels including TRACE and DEBUG.

Initialization:

- A module `logging` at `src/logging.rs` exposes an `init_logging()` function.
- It MUST be invoked at the absolute start of `main` before other subsystems so early failures are captured.
- The initializer is idempotent (subsequent calls are no-ops) and appends to the existing file.

Usage guidance:

- Log user-facing, state-changing actions at INFO (project creation start/success, list operation).
- Log recoverable anomalies at WARN (failed attempt to set git default branch, non-fatal git status errors).
- Log unrecoverable errors or abort conditions at ERROR.
- Use DEBUG/TRACE (debug builds only) for deeper diagnostics when implementing or investigating issues.
