# List projects

List all Rust projects in the projects directory. This feature is available in the global mode of the program.

# User story

As a Rust developer, I want to list all my existing Rust projects, so that I can quickly find and open the one I want to work on without manually browsing the filesystem and open it with my preferred editor.

I want the program to read the projects directory specified in the [configuration](../common/configuration.md) and list all subdirectories that contain a `Cargo.toml` file, which indicates they are Rust projects.

When listing the projects, I want to see the project name (the name of the directory) and the path to the project. I also want an indicator to whether the project has any kind of local changes in its git repository (if it is indeed a git repository) or not.

# Implementation details

The uncommitted changes indicator will be a simple `*` character next to the project name. To determine if a project has uncommitted changes, we will check if the project directory is a git repository (by checking for the presence of a `.git` directory) and then use the `git2` crate to check the status of the repository. If there are any uncommitted changes, we will display the `*` character next to the project name. If an error arise, log and assume no changes.

The implementation of this feature will be in its own module: `crate::project::list`, located at `src/project/list.rs`.
