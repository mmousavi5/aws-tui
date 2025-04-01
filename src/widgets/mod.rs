
pub(crate) mod paragraph;
use ratatui::{
    buffer::Buffer,
    layout::Rect,
};
pub trait WidgetExt {
    fn render(&self, area: Rect, buf: &mut Buffer);
    fn handle_input(&mut self, key_event: crossterm::event::KeyEvent);
}