use crate::event_managment::event::{InputBoxEvent, WidgetActions};
use crate::widgets::WidgetExt;
use clipboard::{ClipboardContext, ClipboardProvider};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::layout::Alignment;
use ratatui::widgets::{Borders, Wrap};
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Style},
    text::Line,
    widgets::{Block, BorderType, Paragraph, Widget},
};
use std::any::Any;

pub struct InputBoxWidget {
    content: String,
    cursor_position: usize,
    active: bool,
    visible: bool,
    title: String,
    clipboard: Option<ClipboardContext>,
}

impl InputBoxWidget {
    pub fn new(title: &str, active: bool) -> Self {
        Self {
            content: String::new(),
            cursor_position: 0,
            active,
            visible: true,
            title: title.to_string(),
            clipboard: ClipboardProvider::new().ok(),
        }
    }

    fn paste_from_clipboard(&mut self) {
        if let Some(ref mut ctx) = self.clipboard {
            if let Ok(contents) = ctx.get_contents() {
                self.content.insert_str(self.cursor_position, &contents);
                self.cursor_position += contents.len();
            }
        }
    }

    fn copy_to_clipboard(&mut self) {
        if let Some(ref mut ctx) = self.clipboard {
            let _ = ctx.set_contents(self.content.clone());
        }
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

    fn handle_input(&mut self, key_event: KeyEvent) -> Option<WidgetActions> {
        // if !self.active {
        //     return;
        // }
        match key_event.code {
            KeyCode::Char('v') if key_event.modifiers == KeyModifiers::CONTROL => {
                self.paste_from_clipboard();
                Some(WidgetActions::InputBoxEvent(InputBoxEvent::Written(
                    self.content.clone(),
                )))
            }
            KeyCode::Char('c') if key_event.modifiers == KeyModifiers::CONTROL => {
                self.copy_to_clipboard();
                Some(WidgetActions::InputBoxEvent(InputBoxEvent::Written(
                    self.content.clone(),
                )))
            }
            KeyCode::Char(ref _c) => Some(WidgetActions::InputBoxEvent(InputBoxEvent::KeyPress(
                key_event,
            ))),
            KeyCode::Backspace => Some(WidgetActions::InputBoxEvent(InputBoxEvent::Backspace)),
            KeyCode::Delete => Some(WidgetActions::InputBoxEvent(InputBoxEvent::Delete)),
            KeyCode::Left => Some(WidgetActions::InputBoxEvent(InputBoxEvent::Left)),
            KeyCode::Right => Some(WidgetActions::InputBoxEvent(InputBoxEvent::Right)),
            KeyCode::Enter => Some(WidgetActions::InputBoxEvent(InputBoxEvent::Enter)),

            _ => None,
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
    fn process_event(&mut self, event: WidgetActions) -> Option<WidgetActions> {
        match event {
            WidgetActions::InputBoxEvent(input_event) => match input_event {
                InputBoxEvent::KeyPress(key_event) => {
                    if let KeyCode::Char(c) = key_event.code {
                        self.content.insert(self.cursor_position, c);
                        self.cursor_position += 1;
                    }
                    None
                }
                InputBoxEvent::Backspace => {
                    if self.cursor_position > 0 {
                        self.cursor_position -= 1;
                        self.content.remove(self.cursor_position);
                    }
                    None
                }
                InputBoxEvent::Delete => {
                    if self.cursor_position < self.content.len() {
                        self.content.remove(self.cursor_position);
                    }
                    None
                }
                InputBoxEvent::Left => {
                    if self.cursor_position > 0 {
                        self.cursor_position -= 1;
                    }
                    None
                }
                InputBoxEvent::Right => {
                    if self.cursor_position < self.content.len() {
                        self.cursor_position += 1;
                    }
                    None
                }
                InputBoxEvent::Enter => {
                    // Handle enter key event
                    // For example, you can send the content to an event sender or process it
                    Some(WidgetActions::InputBoxEvent(InputBoxEvent::Written(
                        self.content.clone(),
                    )))
                }
                _ => None,
            },
            _ => None,
        }
    }
    fn is_active(&self) -> bool {
        self.active
    }
    fn set_title(&mut self, title: String) {
        self.title = title;
    }
}
