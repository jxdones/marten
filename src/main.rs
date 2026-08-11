use clap::Parser;

use crate::{app::App, cli::Cli, error::AppResult};

mod action;
mod app;
mod cli;
mod command_palette;
mod commits_finder;
mod config;
mod diff_panel;
mod editor;
mod error;
mod event;
mod file_finder;
mod files_panel;
mod git;
mod glob;
mod inline_diff;
mod state;
mod store;
mod syntax;
mod terminal;
mod theme;
mod tui;

fn main() {
    if let Err(error) = run() {
        eprintln!("marten: {error}");
        std::process::exit(1);
    }
}

fn run() -> AppResult<()> {
    let cli = Cli::parse();
    let config = config::load()?;
    let mut app = App::new(cli.command, &config)?;
    terminal::run(&mut app)
}
