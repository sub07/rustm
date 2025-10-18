//! Entry point for `rustm`.
//!
//! Current state:
//! - Initializes logging ASAP.
//! - Loads configuration (see `config` module).
//! - If initial setup is required, shows a placeholder fullscreen prompt
//!   allowing the user to input the required fields (very minimal for now).
//! - After configuration is available, shows a placeholder main menu in a
//!   cursive TUI with two global actions:
//!     * Create new project (placeholder flow)
//!     * List projects (placeholder list dialog)
//!
//! This is intentionally skeletal; real feature wiring (nicer UI, error
//! surfaces, navigation) can be layered atop these scaffolds.

mod config;

mod logging;

mod theme;
mod project {

    pub mod create;

    pub mod list;
}

use config::{Config, LoadError, LoadStatus, SetupReason};
use cursive::Cursive;
use cursive::view::{Nameable, Resizable, Scrollable};
use cursive::views::{Dialog, EditView, LinearLayout, SelectView, TextView};
use log::{error, info};
use std::fmt::Write;
use std::process::Command;
fn main() {
    // 1. Initialize logging first.
    if let Err(e) = logging::init_logging() {
        eprintln!("Failed to initialize logging: {e}");
        // Continue anyway; not fatal for user experience.
    }

    // 2. Attempt to load configuration.
    let config = match Config::load() {
        Ok(LoadStatus::Ready(cfg)) => {
            info!("Configuration loaded successfully");
            cfg
        }
        Ok(LoadStatus::NeedsInitialSetup(reason)) => {
            info!("Initial setup required: {:?}", reason_variant(&reason));
            // Launch minimal setup TUI to collect required fields.
            return initial_setup_flow(&reason);
        }
        Err(e) => match e {
            LoadError::Corrupt(msg) => {
                error!("Corrupt configuration: {msg}");
                eprintln!(
                    "Configuration file is corrupt: {msg}\nPlease fix or delete it, then restart."
                );
                std::process::exit(1);
            }
            LoadError::Io(ioe) => {
                error!("I/O error loading config: {ioe}");
                eprintln!("I/O error loading config: {ioe}");
                std::process::exit(1);
            }
        },
    };

    // 3. Run main TUI (global mode placeholder).
    run_main_tui(config);
}

// Translate SetupReason for nicer logging.
const fn reason_variant(r: &SetupReason) -> &'static str {
    match r {
        SetupReason::MissingFile => "MissingFile",
        SetupReason::IncompleteData => "IncompleteData",
    }
}

/// Minimal initial setup flow: ask user for two fields and persist.
/// Extremely bare-bones; no validation feedback loop beyond error dialog.
fn initial_setup_flow(reason: &SetupReason) {
    let mut siv = cursive::default();
    theme::apply_theme(&mut siv);

    let msg = match reason {
        SetupReason::MissingFile => "Welcome! Let's set up rustm.".to_string(),
        SetupReason::IncompleteData => {
            "Configuration incomplete. Please re-enter required fields.".to_string()
        }
    };

    let form = LinearLayout::vertical()
        .child(TextView::new(msg))
        .child(TextView::new("Projects directory:"))
        .child(
            EditView::new()
                .with_name("projects_directory")
                .fixed_width(50),
        )
        .child(TextView::new("Editor command (e.g. code, code -n, vim):"))
        .child(EditView::new().with_name("editor_cmd").fixed_width(50));

    siv.add_layer(
        Dialog::around(form)
            .title("Initial Setup")
            .button("Save", |s| {
                let projects_directory = s
                    .call_on_name("projects_directory", |v: &mut EditView| v.get_content())
                    .unwrap()
                    .to_string();
                let editor_cmd = s
                    .call_on_name("editor_cmd", |v: &mut EditView| v.get_content())
                    .unwrap()
                    .to_string();

                match Config::create_and_persist(&projects_directory, &editor_cmd) {
                    Ok(cfg) => {
                        info!("Initial configuration saved.");
                        s.pop_layer();
                        launch_post_setup(s, cfg);
                    }
                    Err(e) => {
                        error!("Failed to save configuration: {e}");
                        s.add_layer(Dialog::info(format!(
                            "Error saving configuration:\n{e}\nPlease adjust and try again."
                        )));
                    }
                }
            })
            .button("Quit", cursive::Cursive::quit),
    );

    siv.run();
}

/// After saving config from initial setup, proceed to main TUI without restarting.
fn launch_post_setup(siv: &mut Cursive, config: Config) {
    siv.add_layer(main_menu_view(config));
}

/// Run the main TUI with a simple global menu.
fn run_main_tui(config: Config) {
    let mut siv = cursive::default();
    theme::apply_theme(&mut siv);
    siv.add_layer(main_menu_view(config));
    siv.run();
}

/// Build the main menu view.
fn main_menu_view(config: Config) -> Dialog {
    let mut menu = SelectView::<&'static str>::new()
        .item("Create new project", "create")
        .item("List projects", "list")
        .item("Quit", "quit");

    menu.set_on_submit(move |s, choice| match *choice {
        "create" => show_create_project_dialog(s, config.clone()),
        "list" => show_list_projects(s, &config),
        "quit" => s.quit(),
        _ => {}
    });

    Dialog::around(menu.scrollable().fixed_size((40, 10))).title("rustm - Global Mode")
}

/// Create project dialog: prompts for project name, project type, and Rust edition.
fn show_create_project_dialog(s: &mut Cursive, config: Config) {
    use project::create::{ProjectEdition, ProjectType};

    // Select for project type (default Binary)
    let mut type_select = SelectView::<&'static str>::new()
        .popup()
        .item("Binary (--bin)", "bin")
        .item("Library (--lib)", "lib");
    type_select.set_selection(0);

    // Select for Rust edition (default latest: 2024)
    let mut edition_select = SelectView::<&'static str>::new()
        .popup()
        .item("2015", "2015")
        .item("2018", "2018")
        .item("2021", "2021")
        .item("2024 (latest)", "2024");
    edition_select.set_selection(3);

    let form = LinearLayout::vertical()
        .child(TextView::new("Project name:"))
        .child(
            EditView::new()
                .with_name("new_project_name")
                .fixed_width(30),
        )
        .child(TextView::new("Project type:"))
        .child(type_select.with_name("project_type").fixed_width(24))
        .child(TextView::new("Rust edition:"))
        .child(edition_select.with_name("project_edition").fixed_width(24));

    s.add_layer(
        Dialog::around(form)
            .title("Create Project")
            .button("Create", move |siv| {
                use project::create::{CreateProjectParams, create_project};

                let name = siv
                    .call_on_name("new_project_name", |v: &mut EditView| v.get_content())
                    .unwrap()
                    .to_string();

                let selected_type = siv
                    .call_on_name("project_type", |v: &mut SelectView<&'static str>| {
                        v.selection().map(|s| *s)
                    })
                    .flatten()
                    .unwrap_or("bin");

                let selected_edition = siv
                    .call_on_name("project_edition", |v: &mut SelectView<&'static str>| {
                        v.selection().map(|s| *s)
                    })
                    .flatten()
                    .unwrap_or("2024");

                if name.trim().is_empty() {
                    siv.add_layer(Dialog::info("Project name cannot be empty."));

                    return;
                }

                let project_type = match selected_type {
                    "lib" => ProjectType::Library,
                    _ => ProjectType::Binary,
                };

                let edition = match selected_edition {
                    "2015" => ProjectEdition::E2015,
                    "2018" => ProjectEdition::E2018,
                    "2021" => ProjectEdition::E2021,
                    _ => ProjectEdition::E2024,
                };

                // Build params with defaults then override fields explicitly.
                let mut params = CreateProjectParams::new(name);

                params.project_type = project_type;
                params.edition = edition;

                match create_project(&config, params) {
                    Ok(res) => {
                        siv.pop_layer();
                        let project_path = res.project_path.clone();
                        let editor_cmd = config.editor_cmd().to_string();

                        siv.add_layer(
                            Dialog::around(TextView::new(format!(
                                "Project created at:\n{}\n\nOpen in editor?",
                                project_path.display()
                            )))
                            .title("Project Created")
                            .button("Open", move |s2| {
                                if editor_cmd.trim().is_empty() {
                                    s2.add_layer(Dialog::info("Editor command not set."));
                                    return;
                                }
                                let mut parts = editor_cmd.split_whitespace();
                                if let Some(program) = parts.next() {
                                    let mut cmd = Command::new(program);
                                    for arg in parts {
                                        cmd.arg(arg);
                                    }
                                    cmd.arg(&project_path);
                                    match cmd.spawn() {
                                        Ok(_) => {
                                            s2.add_layer(Dialog::info("Editor launched."));
                                        }
                                        Err(e) => {
                                            s2.add_layer(Dialog::info(format!(
                                                "Failed to launch editor: {e}"
                                            )));
                                        }
                                    }
                                } else {
                                    s2.add_layer(Dialog::info("Invalid editor command."));
                                }
                            })
                            .button("Skip", |s2| {
                                s2.pop_layer();
                                s2.add_layer(Dialog::info("Project creation complete."));
                            }),
                        );
                    }

                    Err(e) => {
                        siv.add_layer(Dialog::info(format!("Failed to create project:\n{e}")));
                    }
                }
            })
            .button("Cancel", |siv| {
                siv.pop_layer();
            }),
    );
}

/// Show a simple list of projects discovered.
fn show_list_projects(s: &mut Cursive, config: &Config) {
    use project::list::list_projects;

    match list_projects(config) {
        Ok(projects) => {
            if projects.is_empty() {
                s.add_layer(Dialog::info("No Rust projects found."));
                return;
            }
            let mut text = String::new();
            for p in projects {
                let mut line = p.name.to_string();
                if p.has_uncommitted_changes {
                    line.push_str(" *");
                }
                writeln!(line, "  {}", p.path.display()).unwrap();
                text.push_str(&line);
            }
            s.add_layer(
                Dialog::around(TextView::new(text).scrollable().fixed_size((60, 20)))
                    .title("Projects")
                    .button("Close", |siv| {
                        siv.pop_layer();
                    }),
            );
        }
        Err(e) => {
            s.add_layer(Dialog::info(format!("Failed to list projects:\n{e}")));
        }
    }
}
