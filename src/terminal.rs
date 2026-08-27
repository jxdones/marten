use std::{
    io::{self, Write},
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

use ratatui::{
    DefaultTerminal, TerminalOptions, Viewport, prelude::CrosstermBackend, style::Color,
};

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
    let mut terminal_background = None;
    let result = run_loop(&mut terminal, app, &mut terminal_background);
    restore_terminal(terminal, terminal_background.is_some())?;
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

fn run_loop(
    terminal: &mut DefaultTerminal,
    app: &mut App,
    terminal_background: &mut Option<(u8, u8, u8)>,
) -> AppResult<()> {
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
            draw(terminal, app, terminal_background)?;
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
                    draw(terminal, app, terminal_background)?;
                    needs_draw = false;
                    last_draw = Instant::now();
                }
                CrosstermEvent::Key(key) if key.kind == KeyEventKind::Press => {
                    let action = app.handle_event(Event::Key(key));
                    app.update(action)?;

                    if let Some((path, line)) = app.take_pending_editor() {
                        if terminal_background.take().is_some() {
                            reset_terminal_background(terminal.backend_mut())?;
                        }
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

fn draw(
    terminal: &mut DefaultTerminal,
    app: &mut App,
    terminal_background: &mut Option<(u8, u8, u8)>,
) -> io::Result<()> {
    sync_to_terminal_background(
        terminal.backend_mut(),
        app.terminal_background(),
        terminal_background,
    )?;
    // use synchronization mode to avoid the tearing effect when resizing
    execute!(io::stdout(), BeginSynchronizedUpdate)?;
    let res = terminal.draw(|frame| tui::draw(frame, app));
    execute!(io::stdout(), EndSynchronizedUpdate)?;
    res?;
    Ok(())
}

fn restore_terminal(mut terminal: DefaultTerminal, reset_background: bool) -> io::Result<()> {
    disable_raw_mode()?;
    if reset_background {
        reset_terminal_background(terminal.backend_mut())?;
    }
    execute!(
        terminal.backend_mut(),
        DisableMouseCapture,
        LeaveAlternateScreen,
        SetTitle("")
    )?;
    Ok(())
}

fn sync_to_terminal_background<W: Write>(
    writer: &mut W,
    color: Option<Color>,
    current: &mut Option<(u8, u8, u8)>,
) -> io::Result<()> {
    let Some(Color::Rgb(red, green, blue)) = color else {
        if current.take().is_some() {
            reset_terminal_background(writer)?;
        }
        return Ok(());
    };

    let rgb = (red, green, blue);
    if *current == Some(rgb) {
        return Ok(());
    }

    // OSC 11 changes the terminal emulator's default background color. Some
    // terminals also use that color for padding outside the addressable cell
    // grid. The final `\x1b\\` encodes ST: an ESC byte followed by `\`.
    write!(writer, "\x1b]11;#{red:02x}{green:02x}{blue:02x}\x1b\\")?;
    writer.flush()?;
    *current = Some(rgb);
    Ok(())
}

fn reset_terminal_background<W: Write>(writer: &mut W) -> io::Result<()> {
    // OSC 111 restores the terminal's configured default background, undoing
    // the temporary OSC 11 override. The final `\x1b\\` is the ST terminator.
    writer.write_all(b"\x1b]111\x1b\\")?;
    writer.flush()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn osc_11_uses_the_rgb_theme_background() {
        let mut output = Vec::new();
        let mut current = None;

        sync_to_terminal_background(&mut output, Some(Color::Rgb(245, 239, 228)), &mut current)
            .unwrap();

        assert_eq!(output, b"\x1b]11;#f5efe4\x1b\\");
        assert_eq!(current, Some((245, 239, 228)));
    }

    #[test]
    fn unchanged_background_is_not_emitted_again() {
        let mut output = Vec::new();
        let mut current = Some((22, 17, 13));

        sync_to_terminal_background(&mut output, Some(Color::Rgb(22, 17, 13)), &mut current)
            .unwrap();

        assert!(output.is_empty());
    }

    #[test]
    fn disabled_background_sync_emits_nothing() {
        let mut output = Vec::new();
        let mut current = None;

        sync_to_terminal_background(&mut output, None, &mut current).unwrap();

        assert!(output.is_empty());
    }

    #[test]
    fn reset_color_restores_a_previously_changed_background() {
        let mut output = Vec::new();
        let mut current = Some((22, 17, 13));

        sync_to_terminal_background(&mut output, None, &mut current).unwrap();

        assert_eq!(output, b"\x1b]111\x1b\\");
        assert_eq!(current, None);
    }
}
