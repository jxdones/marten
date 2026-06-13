use std::{io, time::Duration};

use crossterm::{
    event::{self, Event as CrosstermEvent, KeyEventKind},
    execute,
    terminal::{
        BeginSynchronizedUpdate, EndSynchronizedUpdate, EnterAlternateScreen, LeaveAlternateScreen,
        disable_raw_mode, enable_raw_mode,
    },
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
    let mut needs_draw = true;

    while !app.should_quit() {
        if app.poll_workers() {
            needs_draw = true;
        }

        if needs_draw {
            draw(terminal, app)?;
            needs_draw = false;
        }

        if event::poll(Duration::from_millis(50))? {
            match event::read()? {
                CrosstermEvent::Resize(w, h) => {
                    // Drain queued resize events and keep only the last size,
                    // so a resize-drag becomes one redraw instead of N.
                    let mut last = (w, h);
                    while event::poll(Duration::from_secs(0))? {
                        if let CrosstermEvent::Resize(w, h) = event::read()? {
                            last = (w, h);
                        }
                    }
                    let action = app.handle_event(Event::Resize(last.0, last.1));
                    app.update(action);
                    // Paint immediately so there's no gap between the terminal
                    // reflowing and marten repainting.
                    draw(terminal, app)?;
                    needs_draw = false;
                }
                CrosstermEvent::Key(key) if key.kind == KeyEventKind::Press => {
                    let action = app.handle_event(Event::Key(key));
                    app.update(action);
                    needs_draw = true;
                }
                _ => {}
            }
        }
    }

    Ok(())
}

fn draw(terminal: &mut DefaultTerminal, app: &mut App) -> io::Result<()> {
    // use synchronization mode to avoid the tearing effect when resizing
    execute!(io::stdout(), BeginSynchronizedUpdate)?;
    let res = terminal.draw(|frame| tui::draw(frame, app));
    execute!(io::stdout(), EndSynchronizedUpdate)?;
    res?;
    Ok(())
}

fn restore_terminal(mut terminal: DefaultTerminal) -> io::Result<()> {
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    Ok(())
}
