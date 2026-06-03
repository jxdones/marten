use crate::app::App;

mod action;
mod app;
mod diff_panel;
mod event;
mod files_panel;
mod git;
mod inline_diff;
mod state;
mod store;
mod syntax;
mod terminal;
mod tui;

fn main() -> std::io::Result<()> {
    let mut app = App::new();
    terminal::run(&mut app)
}
