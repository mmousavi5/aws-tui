// pub(crate) mod paragraph;
pub(crate) mod input_box;
pub(crate) mod popup;
pub(crate) mod service_navigator;
// pub(crate) mod input_box;
use crate::event_managment::event::WidgetAction;
use std::any::Any;

use ratatui::{buffer::Buffer, layout::Rect};
pub trait WidgetExt {
    fn render(&self, area: Rect, buf: &mut Buffer);
    fn handle_input(&mut self, key_event: crossterm::event::KeyEvent) -> Option<WidgetAction>;
    fn is_visible(&self) -> bool;
    fn set_active(&mut self, active: bool);
    fn set_inactive(&mut self);
    fn set_visible(&mut self, visible: bool);
    fn as_any_mut(&mut self) -> &mut dyn Any;
    fn process_event(&mut self, event: WidgetAction) -> Option<WidgetAction>;
    fn is_active(&self) -> bool;
    fn set_title(&mut self, title: String);
    fn get_help_items(&self) -> Vec<(String, String)>;
}
