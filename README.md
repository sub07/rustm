# `rustm`

## Description

A TUI (Terminal User Interface) for managing Rust projects.

## Installation

To install `rustm`, run the following command:

```bash
cargo install --git https://github.com/sub07/rustm.git
```

## Features

`rustm` can run in two modes: `outside` and `inside` a project. `outside` should be accessible from inside a project.

### Outside project

- Register a project directory to operate on
- Create a new project

### Inside project

- Search for crates
- Add crates to project
- Toggle features on project crates
- Format Cargo.toml
- Only monocrate for now

## Stack
Of course made with Rust.

The TUI is built using [Cursive](https://github.com/gyscos/Cursive).
