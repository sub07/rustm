# Configuration

`rustm` persists user configuration across multiple program runs. Both global and project modes rely on these configurations.

## User story

As a Rust developer, I want `rustm` to remember my configurations across program run, so that I don't have to set them every time I use the application. I want the app to ask me for these configurations only once at the first launch.

I want to be able to open a settings screen from the main menu to change these configurations later if needed.

The configurations I want to persist are:

- `projects_directory`: The directory where new Rust projects will be created and existing ones listed from. I want this directory to be validated before saving it. The directory must exists and have read and write permissions. If not, I expect to see an error message explaining the issue, and then be prompted to enter a new directory.
- `editor_cmd`: The preferred code editor command to open Rust projects with.

## Implementation details

The configuration file will be a single YAML file named `config.yaml` and located at the standard configuration directory for the operating system. To determine this directory, we will use the `dirs` crate.

For serialization and deserialization of the configuration file, we will use the `serde` and `serde_norway v0.9` crates as `serde_yaml` is now unmaintained (`serde_norway` has the same interface as `serde_yaml`).

The procedure for loading the configuration will be as follows: Read and deserialize it into a `ConfigInner` struct. The struct will be wrapped by an Arc in a new `Config` struct to allow cheap cloning. The `Config` struct will expose all string-like configuration fields as methods that return `&str`. Also the struct won't use Option types for the fields, as they are required. If the file is missing or any field is missing, we will present the user the initial setup screen. If the file is present but the deserialization fails, we will show an error message and exit the program. I do not accept unstable state like empty or blank strings in the configuration.

The implementation of this feature will be in its own module called `config`, located at `src/config.rs`. The module will expose the `Config` struct and a function to load and save the configuration from file.
