use crossterm::event::{KeyEvent, MouseEvent};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Event {
    Key(KeyEvent),
    Resize(u16, u16),
    Mouse(MouseEvent),
}
