# Create new Rust project

Creation of a new Rust project. This feature is available in the global mode of the program.

# User story

As a Rust developer, I want to create a new Rust project with the minimum of user input as possible, so that I can quickly start working on my code without manually spinning up a terminal at the right location, typing the needed `cargo new` command and then opening it with my preferred editor.

I want the program to prompt me for the following information:

- Project name
- Project type (binary or library)
- Rust edition (2015, 2018, 2021, 2024) and default to the latest stable edition.

Here are the defaults:

- Project type: binary
- Rust edition: 2024

The new project should be created in the project directory specified in the [configuration](../common/configuration.md). If the directory does not exist or is not writable, I want to see an error message explaining the issue, and then be prompted to enter a new directory for this creation only.

When the project is created, I want the program to prompt if I would like to open this newly created project in my preferred code editor, which is specified in the [configuration](../common/configuration.md). If the editor command is invalid or fails to open the project, I want to see an error message explaining the issue.

# Implementation details

Before calling `cargo new`, set the git `init.defaultBranch` config to `main` globally.

The implementation of this feature will be in its own module: `crate::project::create`, located at `src/project/create.rs`. <!-- Feedback: This prescribes structure inside the requirement doc; acceptable but may become stale if logic/UI separation evolves. Consider moving detailed placement to an architectural/design section. -->
