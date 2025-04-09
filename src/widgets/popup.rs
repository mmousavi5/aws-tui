//! Popup widget module
//!
//! Implements a centered popup overlay for displaying profiles and detailed content.
//! Handles user interactions, rendering, and event processing for popup dialogs.

use crate::{
    event_managment::event::{PopupAction, WidgetAction},
    widgets::WidgetExt,
};
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::layout::{Constraint, Direction, Layout};
use ratatui::{
    buffer::Buffer,
    layout::Alignment,
    layout::Rect,
    style::{Color, Style},
    text::Line,
    widgets::{Block, Clear, Paragraph, Widget},
};
use serde_json;
use std::any::Any;

// Minimum dimensions for popup to ensure it's usable
const MIN_POPUP_WIDTH: u16 = 20;
const MIN_POPUP_HEIGHT: u16 = 10;

/// Content types for the popup dialog
///
/// Profiles displays a selectable list of AWS profiles
/// Details displays formatted text content (often JSON)
#[derive(Clone, Debug)]
pub enum PopupContent {
    Profiles(Vec<String>),
    Details(String),
}

impl PopupContent {
    /// Returns the number of selectable items in the content
    pub fn len(&self) -> usize {
        match self {
            PopupContent::Profiles(profiles) => profiles.len(),
            PopupContent::Details(_) => 0, // Details are not selectable
        }
    }

    /// Gets an item at the specified index
    pub fn get(&self, index: usize) -> Option<&String> {
        match self {
            PopupContent::Profiles(profiles) => profiles.get(index),
            PopupContent::Details(_) => None, // Cannot select individual details
        }
    }
}

/// Widget for displaying popup dialogs with different content types
#[derive(Debug)]
pub struct PopupWidget {
    content: PopupContent,        // Content displayed in the popup
    title: String,                // Title displayed in the popup border
    selected_item: Option<String>, // Currently selected item (if applicable)
    selection_index: usize,       // Index of currently selected item (for lists)
    active: bool,                 // Whether popup has input focus
    visible: bool,                // Whether popup is currently displayed
}

impl PopupWidget {
    /// Creates a new popup widget with optional initial visibility and active state
    pub fn new(content:PopupContent, title: &str, visible: bool, active: bool) -> Self {
        // Load AWS profiles by default

        Self {
            title: title.to_string(),
            selected_item: None,
            content: content,
            selection_index: 0,
            active,
            visible,
        }
    }
    
    /// Updates the content of the popup
    pub fn set_content(&mut self, content: PopupContent) {
        self.content = content;
    }
    
    /// Calculates the area for the popup based on parent area and content type
    fn calculate_popup_area(&self, area: Rect) -> Option<Rect> {
        if area.width <= MIN_POPUP_WIDTH || area.height <= MIN_POPUP_HEIGHT {
            return None;
        }

        // Define percentage constraints based on popup type
        let (width_percent, height_percent) = match self.content {
            PopupContent::Details(_) => (80, 80), // Larger popup for details
            _ => (60, 60),                        // Smaller popup for profiles
        };

        // Create layout splits for both directions
        let vertical_split = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Percentage((100 - height_percent) / 2),
                Constraint::Percentage(height_percent),
                Constraint::Percentage((100 - height_percent) / 2),
            ])
            .split(area);

        let horizontal_split = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage((100 - width_percent) / 2),
                Constraint::Percentage(width_percent),
                Constraint::Percentage((100 - width_percent) / 2),
            ])
            .split(vertical_split[1]);

        // Return the center rectangle
        Some(horizontal_split[1])
    }

    /// Calculates the inner content area with padding from popup borders
    fn calculate_content_area(&self, popup_area: Rect) -> Rect {
        Rect::new(
            popup_area.x.saturating_add(2),
            popup_area.y.saturating_add(1),
            popup_area.width.saturating_sub(4),
            popup_area.height.saturating_sub(2),
        )
    }

    /// Renders content as a list or formats details content with JSON pretty printing
    fn render_content(&self) -> String {
        match &self.content {
            PopupContent::Profiles(items) => items
                .iter()
                .enumerate()
                .map(|(i, item)| {
                    if i == self.selection_index {
                        format!("> {}", item) // Selected item has an indicator
                    } else {
                        format!("  {}", item)
                    }
                })
                .collect::<Vec<_>>()
                .join("\n"),
            PopupContent::Details(content) => {
                // Check if the content starts with a timestamp pattern like [YYYY-MM-DD HH:MM:SS]
                if let Some(json_start) = content.find("] {") {
                    // Extract the potential JSON part (skipping timestamp)
                    let json_str = &content[(json_start + 1).min(content.len())..].trim();

                    // Parse the JSON string
                    match serde_json::from_str::<serde_json::Value>(json_str) {
                        Ok(json) => {
                            // Pretty print with proper indentation
                            serde_json::to_string_pretty(&json).unwrap_or_else(|_| content.clone())
                        }
                        Err(_) => {
                            // Try the original content if JSON extraction failed
                            match serde_json::from_str::<serde_json::Value>(content) {
                                Ok(json) => serde_json::to_string_pretty(&json)
                                    .unwrap_or_else(|_| content.clone()),
                                Err(_) => content.clone(),
                            }
                        }
                    }
                } else {
                    // If no timestamp pattern, try parsing the entire string as JSON
                    match serde_json::from_str::<serde_json::Value>(content) {
                        Ok(json) => {
                            serde_json::to_string_pretty(&json).unwrap_or_else(|_| content.clone())
                        }
                        Err(_) => content.clone(),
                    }
                }
            }
        }
    }
}

impl WidgetExt for PopupWidget {
    /// Renders the popup with a bordered box, title and content
    fn render(&self, area: Rect, buf: &mut Buffer) {
        if !self.visible {
            return;
        }

        let popup_area = match self.calculate_popup_area(area) {
            Some(area) => area,
            None => return, // Screen too small for popup
        };

        let content_area = self.calculate_content_area(popup_area);

        // Render popup background and border
        buf.set_style(popup_area, Style::default().bg(Color::Black));
        Clear.render(popup_area, buf); // Clear any content beneath popup

        // Set border style based on focus state
        let border_style = if self.active {
            Style::default().fg(Color::Red)
        } else {
            Style::default()
        };

        Block::bordered()
            .title(Line::from(self.title.as_str()))
            .border_style(border_style)
            .render(popup_area, buf);

        // Render profiles list or details content
        let content_text = self.render_content();
        Paragraph::new(content_text)
            .block(Block::default())
            .style(Style::default().fg(Color::White).bg(Color::Black))
            .alignment(Alignment::Left)
            .render(content_area, buf);
    }

    /// Handles keyboard input for popup navigation
    fn handle_input(&mut self, key_event: KeyEvent) -> Option<WidgetAction> {
        match key_event.code {
            KeyCode::Up => Some(WidgetAction::PopupAction(PopupAction::ArrowUp)),
            KeyCode::Down => Some(WidgetAction::PopupAction(PopupAction::ArrowDown)),
            KeyCode::Enter => Some(WidgetAction::PopupAction(PopupAction::Enter)),
            KeyCode::Esc => Some(WidgetAction::PopupAction(PopupAction::Escape)),
            _ => None,
        }
    }

    /// Returns whether the popup is currently visible
    fn is_visible(&self) -> bool {
        self.visible
    }

    /// Sets the focus state of the popup
    fn set_active(&mut self, active: bool) {
        self.active = active;
    }

    /// Removes focus from the popup
    fn set_inactive(&mut self) {
        self.active = false;
    }

    /// Controls visibility of the popup
    fn set_visible(&mut self, visible: bool) {
        self.visible = visible;
    }
    
    /// Returns self as Any for downcasting
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
    
    /// Processes popup events and updates state accordingly
    fn process_event(&mut self, event: WidgetAction) -> Option<WidgetAction> {
        match event {
            WidgetAction::PopupAction(event) => match event {
                PopupAction::ArrowUp => {
                    if self.selection_index > 0 {
                        self.selection_index -= 1;
                    }
                    None
                }
                PopupAction::ArrowDown => {
                    if self.selection_index < self.content.len() - 1 {
                        self.selection_index += 1;
                    }
                    None
                }
                PopupAction::Enter => {
                    if let Some(item) = self.content.get(self.selection_index) {
                        self.selected_item = Some(item.clone());
                        return Some(WidgetAction::PopupAction(PopupAction::ItemSelected(
                            self.selected_item.clone().unwrap(),
                        )));
                    }
                    None
                }
                PopupAction::Escape => {
                    self.set_visible(false);
                    None
                }
                _ => None,
            },
            _ => None,
        }
    }

    /// Returns help items for the help toolbar
    fn get_help_items(&self) -> Vec<(String, String)> {
        let mut items = vec![];

        match self.content {
            PopupContent::Profiles(_) => {
                items.push(("Enter".to_string(), "Select profile".to_string()));
            }
            PopupContent::Details(_) => {
                items.push(("PgUp/PgDn".to_string(), "Scroll content".to_string()));
            }
        }

        items.push(("Esc".to_string(), "Close popup".to_string()));
        items.push(("↑/↓".to_string(), "Navigate".to_string()));

        items
    }

    /// Returns whether the popup has focus
    fn is_active(&self) -> bool {
        self.active
    }
    
    /// Updates the popup's title
    fn set_title(&mut self, title: String) {
        self.title = title;
    }
}