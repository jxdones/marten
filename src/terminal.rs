use std::{io, time::Duration};

use crossterm::{
    event::{self, Event as CrosstermEvent, KeyEventKind},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};

use ratatui::{DefaultTerminal, TerminalOptions, Viewport, prelude::CrosstermBackend};

use crate::{app::App, event::Event, tui};

pub fn run(app: &mut App) -> io::Result<()> {
    let mut terminal = init_terminal()?;
    let result = run_loop(&mut terminal, app);
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
            viewport: Viewport::Fullscreen,
        },
    )
}

fn run_loop(terminal: &mut DefaultTerminal, app: &mut App) -> io::Result<()> {
    while !app.should_quit() {
        app.poll_workers();
        terminal.draw(|frame| tui::draw(frame, app))?;

        if event::poll(Duration::from_millis(50))? {
            let event = read_event()?;
            let action = app.handle_event(event);
            app.update(action);
        }
    }

    Ok(())
}

fn read_event() -> io::Result<Event> {
    loop {
        match event::read()? {
            CrosstermEvent::Key(key) if key.kind == KeyEventKind::Press => {
                return Ok(Event::Key(key));
            }
            CrosstermEvent::Resize(width, height) => {
                return Ok(Event::Resize(width, height));
            }
            _ => {}
        }
    }
}

fn restore_terminal(mut terminal: DefaultTerminal) -> io::Result<()> {
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    Ok(())
}
