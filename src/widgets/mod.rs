// pub(crate) mod paragraph;
pub(crate) mod aws_service_navigator;
pub(crate) mod input_box;
pub(crate) mod popup;
// pub(crate) mod input_box;
use crate::event_managment::event::{Event, WidgetActions};
use std::any::Any;

use futures::channel;
use ratatui::{buffer::Buffer, layout::Rect};
pub trait WidgetExt {
    fn render(&self, area: Rect, buf: &mut Buffer);
    fn handle_input(&mut self, key_event: crossterm::event::KeyEvent) -> Option<WidgetActions>;
    fn is_visible(&self) -> bool;
    fn set_active(&mut self, active: bool);
    fn set_inactive(&mut self);
    fn set_visible(&mut self, visible: bool);
    fn as_any_mut(&mut self) -> &mut dyn Any;
    fn process_event(&mut self, event: WidgetActions);
    fn is_active(&self) -> bool;
}
