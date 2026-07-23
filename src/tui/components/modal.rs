use ratatui::{
    Frame,
    layout::{Constraint, Flex, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
};

use crate::tui::{components::panel, theme::Theme};

pub struct Modal {
    screen: Rect,
    area: Rect,
    inner: Rect,
    theme: Theme,
    block: Block<'static>,
}

#[derive(Debug, Clone, Copy)]
pub struct ResponsiveSize {
    percent: u16,
    max: u16,
    margin: u16,
}

impl ResponsiveSize {
    pub const fn new(percent: u16, max: u16) -> Self {
        Self {
            percent,
            max,
            margin: 0,
        }
    }

    pub const fn with_margin(mut self, margin: u16) -> Self {
        self.margin = margin;
        self
    }

    fn resolve(self, available: u16) -> u16 {
        let available_after_margin = available.saturating_sub(self.margin.saturating_mul(2));
        let fluid = available.saturating_mul(self.percent.min(100)) / 100;

        fluid.min(self.max).min(available_after_margin)
    }
}

#[derive(Debug, Clone, Copy)]
pub struct ModalSize {
    width: ResponsiveSize,
    height: ResponsiveSize,
}

impl ModalSize {
    pub const fn new(width: ResponsiveSize, height: ResponsiveSize) -> Self {
        Self { width, height }
    }

    fn resolve(self, area: Rect) -> (u16, u16) {
        (
            self.width.resolve(area.width),
            self.height.resolve(area.height),
        )
    }
}

pub struct ModalConfig {
    size: ModalSize,
}

impl ModalConfig {
    pub const fn new(size: ModalSize) -> Self {
        Self { size }
    }
}

impl Modal {
    pub fn new(screen: Rect, theme: Theme, config: ModalConfig) -> Self {
        let (width, height) = config.size.resolve(screen);
        let area = centered(screen, width, height);
        let block = panel::block(None, theme, Borders::ALL, theme.bg, true);
        let inner = block.inner(area);

        Self {
            screen,
            area,
            inner,
            theme,
            block,
        }
    }

    pub const fn inner(&self) -> Rect {
        self.inner
    }

    pub fn render(&self, frame: &mut Frame) {
        let bg_style = Style::default().fg(self.theme.fg).bg(self.theme.bg);

        dim_background(frame, self.screen, self.area, self.theme.bg, self.theme.dim);
        frame.render_widget(Clear, self.area);
        frame.render_widget(Block::default().style(bg_style), self.area);
        frame.render_widget(self.block.clone(), self.area);
    }
}

pub fn draw_title_bar(frame: &mut Frame, area: Rect, title: &'static str, theme: Theme) {
    let block = Block::default()
        .borders(Borders::BOTTOM)
        .border_style(theme.panel_border())
        .style(Style::default().bg(theme.bg));
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let close = vec![
        Span::styled("esc/q ", theme.accent()),
        Span::styled("close  ", theme.muted()),
    ];
    let close_width = u16::try_from(Line::from(close.clone()).width()).unwrap_or(inner.width);
    let [title_area, close_area] = Layout::horizontal([
        Constraint::Min(0),
        Constraint::Length(close_width.min(inner.width)),
    ])
    .areas(inner);

    frame.render_widget(
        Paragraph::new(Line::from(Span::styled(
            format!("  {title}"),
            theme.accent().add_modifier(Modifier::BOLD),
        ))),
        title_area,
    );
    frame.render_widget(Paragraph::new(Line::from(close)), close_area);
}

fn dim_background(frame: &mut Frame, area: Rect, overlay: Rect, bg: Color, fg: Color) {
    let buffer = frame.buffer_mut();
    let style = Style::default().fg(fg).bg(bg);

    for y in area.y..area.y.saturating_add(area.height) {
        for x in area.x..area.x.saturating_add(area.width) {
            if contains(overlay, x, y) {
                continue;
            }

            if let Some(cell) = buffer.cell_mut((x, y)) {
                cell.set_style(style);
            }
        }
    }
}

fn centered(area: Rect, width: u16, height: u16) -> Rect {
    let width = width.min(area.width);
    let height = height.min(area.height);

    let [area] = Layout::horizontal([Constraint::Length(width)])
        .flex(Flex::Center)
        .areas(area);

    let [area] = Layout::vertical([Constraint::Length(height)])
        .flex(Flex::Center)
        .areas(area);
    area
}

fn contains(rect: Rect, x: u16, y: u16) -> bool {
    x >= rect.x
        && x < rect.x.saturating_add(rect.width)
        && y >= rect.y
        && y < rect.y.saturating_add(rect.height)
}
