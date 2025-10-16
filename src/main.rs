use cursive::{Cursive, CursiveExt};

fn main() {
    let mut siv = Cursive::default();

    // TODO: Detect if running inside or outside a Rust project
    // For now, just show a placeholder menu

    siv.add_layer(
        cursive::views::Dialog::text("Welcome to rustm!\n\n[Stub] Choose mode:\n- Outside project: Register/Create project\n- Inside project: Manage crates, features, format Cargo.toml")
            .title("rustm")
            .button("Quit", |s| s.quit()),
    );

    siv.run();
}
