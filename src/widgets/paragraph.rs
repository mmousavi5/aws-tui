use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    widgets::{Block, BorderType, Paragraph, Widget},
    style::{Style, Color},
};
use ratatui::layout::Alignment;
use ratatui::widgets::{Borders, Wrap};
use crate::widgets::WidgetExt;
use std::any::Any;
use crate::event_managment::event::{WidgetActions, TabEvent};

pub enum ParagraphEvent {
    ArrowUp,
    ArrowDown,
    Enter,
    Escape,
    Cancel,
}

pub struct ParagraphWidget {
    pub text:  String,
    pub selected_profile_index: usize,
    count: usize,
    active: bool,
    visible: bool,
}
impl ParagraphWidget {
    pub fn new(text: &str, active:bool) -> Self {
        Self {
            text: text.to_string(),
            selected_profile_index: 0,
            count: 0,
            active,
            visible: true,
        }
    }
}
impl WidgetExt for ParagraphWidget {
    fn render(&self, area: Rect, buf: &mut Buffer) {
        if !self.visible {
            return;
        }
        let border_style = if self.active {
            Style::default().fg(Color::Red)
        } else {
            Style::default()
        };

        let block = Block::default()
            .title("Paragraph")
            .border_type(BorderType::Rounded)
            .borders(Borders::ALL)
            .border_style(border_style);

        let paragraph_text = self.text.clone() + &format!("\nCount: {}", self.count);
        let paragraph = Paragraph::new(paragraph_text)
            .block(block)
            .alignment(Alignment::Left)
            .wrap(Wrap { trim: true });

        paragraph.render(area, buf);
    }

    fn handle_input(&mut self, key_event: KeyEvent) {
        match key_event.code {
            KeyCode::Up => {
                self.count += 1;
            }
            KeyCode::Down => {
                if self.count > 0 {
                    self.count -= 1;
                }
            }
            _ => {}
        }
    }
    fn is_visible(&self) -> bool {
        self.visible
    }
    fn set_active(&mut self) {
        self.active = true;
    }
    fn set_inactive(&mut self) {
        self.active = false;
    }
    fn set_visible(&mut self, visible: bool) {
        self.visible = visible;
    }
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
    fn process_event(&mut self, event: WidgetActions) {
        todo!()
    }
}