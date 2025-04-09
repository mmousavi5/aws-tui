//! Input box widget module
//!
//! Provides a text input box with clipboard support and cursor positioning.
//! Used for search queries, filters, and other text input needs.

use crate::event_managment::event::{InputBoxEvent, InputBoxType, WidgetAction};
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

/// Widget for text input with cursor positioning and clipboard integration
pub struct InputBoxWidget {
    input_type: InputBoxType,            // Type of widget (e.g., InputBox)
    content: String,                     // Current text content
    cursor_position: usize,              // Position of cursor within the text
    active: bool,                        // Whether this widget has input focus
    visible: bool,                       // Whether this widget should be rendered
    title: String,                       // Title displayed in the border
    clipboard: Option<ClipboardContext>, // Clipboard access for copy/paste
}

impl InputBoxWidget {
    /// Creates a new input box with the specified title and active state
    pub fn new(input_type: InputBoxType, title: &str, active: bool) -> Self {
        Self {
            input_type,
            content: String::new(),
            cursor_position: 0,
            active,
            visible: true,
            title: title.to_string(),
            clipboard: ClipboardProvider::new().ok(), // Initialize clipboard or None if unavailable
        }
    }

    /// Pastes text from the system clipboard at the current cursor position
    fn paste_from_clipboard(&mut self) {
        if let Some(ref mut ctx) = self.clipboard {
            if let Ok(contents) = ctx.get_contents() {
                self.content.insert_str(self.cursor_position, &contents);
                self.cursor_position += contents.len();
            }
        }
    }

    /// Copies the current input text to the system clipboard
    fn copy_to_clipboard(&mut self) {
        if let Some(ref mut ctx) = self.clipboard {
            let _ = ctx.set_contents(self.content.clone());
        }
    }

    /// Returns the current text content of the input box
    pub fn get_content(&self) -> Option<String> {
        if self.content.is_empty() {
            None
        } else {
            Some(self.content.clone())
        }
    }
}

impl WidgetExt for InputBoxWidget {
    /// Renders the input box with cursor at the current position
    fn render(&self, area: Rect, buf: &mut Buffer) {
        if !self.visible {
            return;
        }

        // Set border color based on focus state
        let border_style = if self.active {
            Style::default().fg(Color::Red)
        } else {
            Style::default()
        };

        // Create the visual container block with border
        let block = Block::default()
            .title(Line::from(self.title.as_str()))
            .border_type(BorderType::Rounded)
            .borders(Borders::ALL)
            .border_style(border_style);

        // Create display text with cursor indicator
        let mut display_text = self.content.clone();
        if self.active {
            display_text.insert(self.cursor_position, '|');
        }

        // Create and render paragraph with the content and cursor
        let paragraph = Paragraph::new(display_text)
            .block(block)
            .alignment(Alignment::Left)
            .wrap(Wrap { trim: true });

        paragraph.render(area, buf);
    }

    /// Handles keyboard input and triggers appropriate actions
    fn handle_input(&mut self, key_event: KeyEvent) -> Option<WidgetAction> {
        // if !self.active {
        //     return;
        // }
        match key_event.code {
            // Clipboard operations with Ctrl modifiers
            KeyCode::Char('v') if key_event.modifiers == KeyModifiers::CONTROL => {
                self.paste_from_clipboard();
                Some(WidgetAction::InputBoxEvent(
                    InputBoxEvent::Written(self.content.clone()),
                    self.input_type.clone(),
                ))
            }
            KeyCode::Char('c') if key_event.modifiers == KeyModifiers::CONTROL => {
                self.copy_to_clipboard();
                Some(WidgetAction::InputBoxEvent(
                    InputBoxEvent::Written(self.content.clone()),
                    self.input_type.clone(),
                ))
            }
            // Pass through regular character input
            KeyCode::Char(ref _c) => Some(WidgetAction::InputBoxEvent(
                InputBoxEvent::KeyPress(key_event),
                self.input_type.clone(),
            )),
            // Text editing commands
            KeyCode::Backspace => Some(WidgetAction::InputBoxEvent(
                InputBoxEvent::Backspace,
                self.input_type.clone(),
            )),
            KeyCode::Delete => Some(WidgetAction::InputBoxEvent(
                InputBoxEvent::Delete,
                self.input_type.clone(),
            )),
            // Cursor movement
            KeyCode::Left => Some(WidgetAction::InputBoxEvent(
                InputBoxEvent::Left,
                self.input_type.clone(),
            )),
            KeyCode::Right => Some(WidgetAction::InputBoxEvent(
                InputBoxEvent::Right,
                self.input_type.clone(),
            )),
            // Submit content
            KeyCode::Enter => Some(WidgetAction::InputBoxEvent(
                InputBoxEvent::Enter,
                self.input_type.clone(),
            )),

            _ => None,
        }
    }

    /// Returns whether the widget is currently visible
    fn is_visible(&self) -> bool {
        self.visible
    }

    /// Sets whether the widget is active (has focus)
    fn set_active(&mut self, active: bool) {
        self.active = active;
    }

    /// Removes focus from the widget
    fn set_inactive(&mut self) {
        self.active = false;
    }

    /// Controls visibility of the widget
    fn set_visible(&mut self, visible: bool) {
        self.visible = visible;
    }

    /// Provides access to this widget as Any for downcasting
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }

    /// Processes input events and modifies content accordingly
    fn process_event(&mut self, event: WidgetAction) -> Option<WidgetAction> {
        match event {
            WidgetAction::InputBoxEvent(input_event, _) => match input_event {
                // Add character at cursor position
                InputBoxEvent::KeyPress(key_event) => {
                    if let KeyCode::Char(c) = key_event.code {
                        self.content.insert(self.cursor_position, c);
                        self.cursor_position += 1;
                    }
                    None
                }
                // Delete character to the left of cursor
                InputBoxEvent::Backspace => {
                    if self.cursor_position > 0 {
                        self.cursor_position -= 1;
                        self.content.remove(self.cursor_position);
                    }
                    None
                }
                // Delete character under cursor
                InputBoxEvent::Delete => {
                    if self.cursor_position < self.content.len() {
                        self.content.remove(self.cursor_position);
                    }
                    None
                }
                // Move cursor left
                InputBoxEvent::Left => {
                    if self.cursor_position > 0 {
                        self.cursor_position -= 1;
                    }
                    None
                }
                // Move cursor right
                InputBoxEvent::Right => {
                    if self.cursor_position < self.content.len() {
                        self.cursor_position += 1;
                    }
                    None
                }
                // Submit current content
                InputBoxEvent::Enter => {
                    // Handle enter key event
                    // For example, you can send the content to an event sender or process it
                    Some(WidgetAction::InputBoxEvent(
                        InputBoxEvent::Written(self.content.clone()),
                        self.input_type.clone(),
                    ))
                }
                _ => None,
            },
            _ => None,
        }
    }

    /// Returns available keyboard shortcuts for the help toolbar
    fn get_help_items(&self) -> Vec<(String, String)> {
        vec![
            ("Ctrl+V".to_string(), "Paste".to_string()),
            ("Ctrl+C".to_string(), "Copy".to_string()),
            ("Enter".to_string(), "Submit".to_string()),
            ("Esc".to_string(), "Close".to_string()),
        ]
    }

    /// Checks if the widget currently has focus
    fn is_active(&self) -> bool {
        self.active
    }

    /// Updates the widget's title
    fn set_title(&mut self, title: String) {
        self.title = title;
    }
}
