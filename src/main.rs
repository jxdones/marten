use crate::app::App;

mod action;
mod app;
mod event;
mod state;
mod terminal;
mod tui;

fn main() -> std::io::Result<()> {
    let mut app = App::new();
    terminal::run(&mut app)
}
