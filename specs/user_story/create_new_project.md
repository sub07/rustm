# User Story

**Title:** Create a new project

**Mode**: Outside

**Story:**
As a user, I want to create a new project in my project dir so that I don't have to open a terminal in the project dir and type the command myself.

## Acceptance Criteria

- The user can specify the project type: `binary` or `library`.
- The user can specify the project name.
- The user can specify the project edition (2015, 2018, 2021, 2024) with the latest already selected.
- The project directory is specified in a configuration file that is global to the program. This file must be stored in the standard configuration directory for the operating system.
- If the config file does not exist or the project directory is not set, the user is prompted to set it at the start of the program **only in outside mode**.
- The user is prompted to open the newly created project with his favorite text editor.
- The text editor command is specified in the configuration file. If not set, prompt it at the start of the program **only in outside mode**.

## Notes

- The configuration file should be in yaml format.
- The project creation should use `cargo new` command under the hood.
- The TUI should provide a form to input the project details (name, type, edition).
- The TUI must reduce the bare minimum of inputs to create a project, with sensible defaults.

## Status
Done
