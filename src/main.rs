use crate::app::App;

mod action;
mod app;
mod event;
mod git;
mod inline_diff;
mod state;
mod syntax;
mod terminal;
mod tui;

fn main() -> std::io::Result<()> {
    let mut app = App::new();
    terminal::run(&mut app)
}
