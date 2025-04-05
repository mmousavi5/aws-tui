use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    widgets::{Block, BorderType, Paragraph, Widget},
    style::{Style, Color},
    text::Line,
};
use ratatui::layout::Alignment;
use ratatui::widgets::{Borders, Wrap};
use crate::widgets::WidgetExt;
use std::any::Any;
use crate::event_managment::event::{WidgetActions, InputBoxEvent};

pub struct InputBoxWidget {
    content: String,
    cursor_position: usize,
    active: bool,
    visible: bool,
    title: String,
}

impl InputBoxWidget {
    pub fn new(title: &str, active: bool) -> Self {
        Self {
            content: String::new(),
            cursor_position: 0,
            active,
            visible: true,
            title: title.to_string(),
        }
    }

    pub fn get_content(&self) -> &str {
        &self.content
    }
}

impl WidgetExt for InputBoxWidget {
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
            .title(Line::from(self.title.as_str()))
            .border_type(BorderType::Rounded)
            .borders(Borders::ALL)
            .border_style(border_style);

        // Create display text with cursor
        let mut display_text = self.content.clone();
        if self.active {
            display_text.insert(self.cursor_position, '|');
        }

        let paragraph = Paragraph::new(display_text)
            .block(block)
            .alignment(Alignment::Left)
            .wrap(Wrap { trim: true });

        paragraph.render(area, buf);
    }

    fn handle_input(&mut self, key_event: KeyEvent)  -> Option<WidgetActions>  {
        // if !self.active {
        //     return;
        // }
        match key_event.code {
            KeyCode::Char(ref _c) => Some(WidgetActions::InputBoxEvent(InputBoxEvent::KeyPress(key_event))),
            KeyCode::Backspace => Some(WidgetActions::InputBoxEvent(InputBoxEvent::Backspace)),
            KeyCode::Delete => Some(WidgetActions::InputBoxEvent(InputBoxEvent::Delete)),
            KeyCode::Left => Some(WidgetActions::InputBoxEvent(InputBoxEvent::Left)),
            _ => None
            
        }
    }

    fn is_visible(&self) -> bool {
        self.visible
    }

    fn set_active(&mut self, active: bool) {
        self.active = active;
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

        match event {
            WidgetActions::InputBoxEvent(input_event) => match input_event {
                InputBoxEvent::KeyPress(key_event) => {
                    if let KeyCode::Char(c) = key_event.code {
                        self.content.insert(self.cursor_position, c);
                        self.cursor_position += 1;
                    }
                }
                InputBoxEvent::Backspace => {
                    if self.cursor_position > 0 {
                        self.cursor_position -= 1;
                        self.content.remove(self.cursor_position);
                    }
                }
                InputBoxEvent::Delete => {
                    if self.cursor_position < self.content.len() {
                        self.content.remove(self.cursor_position);
                    }
                }
                InputBoxEvent::Left => {
                    if self.cursor_position > 0 {
                        self.cursor_position -= 1;
                    }
                }
                _ => {}
            },
            _ => {}
        }
        
    }
    fn is_active(&self) -> bool {
        self.active
    }
}