use ratatui::{
    Frame,
    layout::{Constraint, Flex, Layout, Rect},
    style::{Color, Style},
    text::Line,
    widgets::{Block, Borders, Clear},
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
    title: Option<Line<'static>>,
}

impl ModalConfig {
    pub const fn new(size: ModalSize) -> Self {
        Self { size, title: None }
    }

    pub fn title(mut self, title: impl Into<Line<'static>>) -> Self {
        self.title = Some(title.into());
        self
    }
}

impl Modal {
    pub fn new(screen: Rect, theme: Theme, config: ModalConfig) -> Self {
        let (width, height) = config.size.resolve(screen);
        let area = centered(screen, width, height);
        let block = panel::block(config.title, theme, Borders::ALL, theme.bg, true);
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
