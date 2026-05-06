use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{DefaultTerminal, TerminalOptions, Viewport, prelude::CrosstermBackend};
use std::io;

use crate::tui;

pub fn run() -> io::Result<()> {
    let mut terminal = init_terminal()?;
    let result = run_loop(&mut terminal);
    restore_terminal(terminal)?;
    result
}

fn init_terminal() -> io::Result<DefaultTerminal> {
    enable_raw_mode()?;
    execute!(io::stdout(), EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(io::stdout());
    ratatui::Terminal::with_options(
        backend,
        TerminalOptions {
            viewport: Viewport::Inline(30),
        },
    )
}

fn run_loop(terminal: &mut DefaultTerminal) -> io::Result<()> {
    loop {
        terminal.draw(tui::draw)?;

        if let Event::Key(key) = event::read()?
            && key.kind == KeyEventKind::Press
            && matches!(key.code, KeyCode::Char('q'))
        {
            break;
        }
    }
    Ok(())
}

fn restore_terminal(mut terminal: DefaultTerminal) -> io::Result<()> {
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    Ok(())
}
