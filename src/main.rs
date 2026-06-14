use clap::Parser;

use crate::{app::App, cli::Cli};

mod action;
mod app;
mod cli;
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
    let cli = Cli::parse();
    let mut app = App::new(cli.command);
    terminal::run(&mut app)
}
