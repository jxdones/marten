use std::{
    io,
    time::{Duration, Instant},
};

use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event as CrosstermEvent, KeyEventKind},
    execute,
    terminal::{
        BeginSynchronizedUpdate, EndSynchronizedUpdate, EnterAlternateScreen, LeaveAlternateScreen,
        SetTitle, disable_raw_mode, enable_raw_mode,
    },
};

use ratatui::{DefaultTerminal, TerminalOptions, Viewport, prelude::CrosstermBackend};

use crate::{
    action::Action,
    app::App,
    editor,
    error::{AppError, AppResult},
    event::Event,
    tui,
};

pub fn run(app: &mut App) -> AppResult<()> {
    let mut terminal = init_terminal()?;
    let result = run_loop(&mut terminal, app);
    restore_terminal(terminal)?;
    result
}

fn init_terminal() -> io::Result<DefaultTerminal> {
    enable_raw_mode()?;
    execute!(io::stdout(), EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(io::stdout());
    ratatui::Terminal::with_options(
        backend,
        TerminalOptions {
            viewport: Viewport::Fullscreen,
        },
    )
}

// Mouse wheels/trackpads can emit events faster than once per frame. Drawing on
// every event makes the redraw count scale with input rate instead of frame rate.
// Capping draws to this cadence keeps state updates immediate while decoupling
// how often we actually repaint.
const FRAME_BUDGET: Duration = Duration::from_millis(16);

fn run_loop(terminal: &mut DefaultTerminal, app: &mut App) -> AppResult<()> {
    let mut needs_draw = true;
    let mut last_draw = Instant::now();
    // Draining a resize burst requires reading one event past its end. Keep
    // that event here so keyboard and mouse input are not discarded.
    let mut pending_event = None;

    while !app.should_quit() {
        if app.poll_workers() {
            needs_draw = true;
        }

        if needs_draw && last_draw.elapsed() >= FRAME_BUDGET {
            draw(terminal, app)?;
            needs_draw = false;
            last_draw = Instant::now();
        }

        let poll_timeout = if needs_draw {
            FRAME_BUDGET.saturating_sub(last_draw.elapsed())
        } else {
            Duration::from_millis(50)
        };

        let next_event = if let Some(event) = pending_event.take() {
            Some(event)
        } else if event::poll(poll_timeout)? {
            Some(event::read()?)
        } else {
            None
        };

        if let Some(next_event) = next_event {
            match next_event {
                CrosstermEvent::Resize(w, h) => {
                    // Drain queued resize events and keep only the last size,
                    // so a resize-drag becomes one redraw instead of N.
                    let mut last = (w, h);
                    while event::poll(Duration::ZERO)? {
                        match event::read()? {
                            CrosstermEvent::Resize(w, h) => last = (w, h),
                            other => {
                                pending_event = Some(other);
                                break;
                            }
                        }
                    }
                    let action = app.handle_event(Event::Resize(last.0, last.1));
                    app.update(action)?;
                    // Paint immediately so there's no gap between the terminal
                    // reflowing and marten repainting.
                    draw(terminal, app)?;
                    needs_draw = false;
                    last_draw = Instant::now();
                }
                CrosstermEvent::Key(key) if key.kind == KeyEventKind::Press => {
                    let action = app.handle_event(Event::Key(key));
                    app.update(action)?;

                    if let Some((path, line)) = app.take_pending_editor() {
                        disable_raw_mode()?;
                        execute!(io::stdout(), LeaveAlternateScreen, DisableMouseCapture)?;
                        editor::command(&path, line as usize)
                            .status()
                            .map_err(|source| {
                                AppError::from(source).with_operation("open editor")
                            })?;

                        execute!(io::stdout(), EnterAlternateScreen, EnableMouseCapture)?;
                        enable_raw_mode()?;
                        terminal.clear()?;
                        app.update(Action::Refresh)?;
                    }

                    needs_draw = true;
                }
                CrosstermEvent::Mouse(mouse) => {
                    let action = app.handle_event(Event::Mouse(mouse));
                    app.update(action)?;
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
    execute!(
        terminal.backend_mut(),
        DisableMouseCapture,
        LeaveAlternateScreen,
        SetTitle("")
    )?;
    Ok(())
}
