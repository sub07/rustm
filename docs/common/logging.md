# Log

I want to keep a log of significant actions performed by the program, such as creating a new project or opening a project in the editor. This log will help me track my activities and troubleshoot any issues that may arise.
We will use the `log` crate for logging, along with the `simplelog` crate to provide a simple logging implementation that writes logs to a file. The log file will be located in the same way as the configuration file, using the standard log directory for the operating system with the `dirs` crate.

During the implementation of features, we will add log statements at appropriate levels (e.g., `Info`, `Warn`, `Error`) to capture significant actions and events. The log file will be rotated when it reaches a certain size to prevent it from growing indefinitely. Every log level above INFO (which is included) will be logged. The size theshold of the rotation strategy will be 5MB.
In debug builds (debug_assertions cfg enabled), we will log _all_ levels.

The configuration of the logger will be located in its own module called `logging`, located at `src/logging.rs`. The module will expose a function to initialize the logger, which will be called at the _absolute_ start of the program.
