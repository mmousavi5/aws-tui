use crate::event_managment::event::{WidgetActions, WidgetEventType, WidgetType};
use crate::{
    event_managment::event::{AWSServiceNavigatorEvent, InputBoxEvent},
    widgets::WidgetExt,
};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::{
    buffer::Buffer,
    layout::{Alignment, Rect},
    style::{Color, Style},
    widgets::{Block, BorderType, Borders, Paragraph, Widget},
};
use std::any::Any;

/// Content types that can be displayed in the navigator
/// Services are AWS service types, Records are string entries like log groups
#[derive(Clone)]
pub enum NavigatorContent {
    Services(Vec<WidgetEventType>),
    Records(Vec<String>),
}

/// Widget for navigating AWS services or records with filtering capabilities
/// Handles navigation, selection, and filtering of items
pub struct AWSServiceNavigator {
    title: String,
    widget_type: WidgetType,
    content: NavigatorContent,          // Original unfiltered content
    filtered_content: NavigatorContent, // Content after applying filters
    filter_text: String,                // Current filter string
    selected_index: usize,              // Currently selected item
    scroll_offset: usize,               // Scroll position for viewing large lists
    active: bool,                       // Whether this widget has focus
    visible: bool,                      // Whether this widget should be rendered
    filter_mode: bool,                  // Whether filter input mode is active
}

impl AWSServiceNavigator {
    /// Creates a new navigator with the specified widget type, active state, and content
    pub fn new(widget_type: WidgetType, active: bool, content: NavigatorContent) -> Self {
        Self {
            title: "AWS Services".to_string(),
            widget_type,
            content: content.clone(),
            filtered_content: content,
            filter_text: String::new(),
            selected_index: 0,
            scroll_offset: 0,
            active,
            visible: true,
            filter_mode: false, // Start with filter mode disabled
        }
    }

    /// Returns the number of items in the current (filtered) content
    fn content_len(&self) -> usize {
        match &self.filtered_content {
            NavigatorContent::Services(services) => services.len(),
            NavigatorContent::Records(records) => records.len(),
        }
    }

    /// Returns a widget action for the currently selected item
    fn selected_item(&self) -> Option<WidgetActions> {
        match &self.filtered_content {
            NavigatorContent::Services(services) => {
                if self.selected_index < services.len() {
                    services.get(self.selected_index).map(|service| {
                        WidgetActions::AWSServiceNavigatorEvent(
                            AWSServiceNavigatorEvent::SelectedItem(service.clone()),
                            self.widget_type,
                        )
                    })
                } else {
                    None
                }
            }
            NavigatorContent::Records(records) => {
                if self.selected_index < records.len() {
                    records.get(self.selected_index).map(|record| {
                        WidgetActions::AWSServiceNavigatorEvent(
                            AWSServiceNavigatorEvent::SelectedItem(
                                WidgetEventType::RecordSelected(record.clone()),
                            ),
                            self.widget_type,
                        )
                    })
                } else {
                    None
                }
            }
        }
    }

    /// Adjusts scroll position to keep selected item visible
    fn update_scroll_offset(&mut self, height: usize) {
        // Make sure height is at least 1 to avoid division by zero
        let height = height.max(1);

        // If the selected index is above the current scroll position, scroll up
        if self.selected_index < self.scroll_offset {
            self.scroll_offset = self.selected_index;
            return;
        }

        // Only scroll down when the selected index is beyond the visible area
        // We want to display as many items as possible without unnecessary scrolling
        if self.selected_index >= self.scroll_offset + height {
            // Calculate how far to scroll - position the selected item at the bottom of visible area
            self.scroll_offset = self.selected_index - height + 1;
        }
    }

    /// Applies a filter to the content, showing only items containing the filter text
    pub fn apply_filter(&mut self, filter: &str) {
        self.filter_text = filter.to_lowercase();

        // Reset navigation state when filter changes
        self.selected_index = 0;
        self.scroll_offset = 0;

        // If filter is empty, show all content
        if self.filter_text.is_empty() {
            self.filtered_content = self.content.clone();
            return;
        }

        // Apply filter based on content type
        match &self.content {
            NavigatorContent::Services(services) => {
                let filtered = services
                    .iter()
                    .filter(|service| {
                        service
                            .to_string()
                            .to_lowercase()
                            .contains(&self.filter_text)
                    })
                    .cloned()
                    .collect();
                self.filtered_content = NavigatorContent::Services(filtered);
            }
            NavigatorContent::Records(records) => {
                let filtered = records
                    .iter()
                    .filter(|record| record.to_lowercase().contains(&self.filter_text))
                    .cloned()
                    .collect();
                self.filtered_content = NavigatorContent::Records(filtered);
            }
        }
    }

    /// Adds a character to the filter and applies it
    fn add_to_filter(&mut self, c: char) {
        self.filter_text.push(c);
        let filter_text_clone = self.filter_text.clone();
        self.apply_filter(&filter_text_clone);
    }

    /// Removes the last character from the filter and applies it
    fn remove_from_filter(&mut self) {
        if let Some(_) = self.filter_text.pop() {
            let filter_text_clone = self.filter_text.clone();
            self.apply_filter(&filter_text_clone);
        }
    }

    /// Clears the filter and shows all content
    fn clear_filter(&mut self) {
        self.filter_text.clear();
        self.filtered_content = self.content.clone();
        self.filter_mode = false;
    }

    /// Sets new content for the navigator
    /// If a filter is active, it will be applied to the new content
    pub fn set_content(&mut self, content: NavigatorContent) {
        self.content = content.clone();

        // Apply existing filter to new content
        if !self.filter_text.is_empty() {
            let filter_text_clone = self.filter_text.clone();
            self.apply_filter(&filter_text_clone);
        } else {
            self.filtered_content = content;
        }

        self.selected_index = 0;
        self.scroll_offset = 0;
    }
}

impl WidgetExt for AWSServiceNavigator {
    /// Renders the navigator widget to the buffer
    fn render(&self, area: Rect, buf: &mut Buffer) {
        if !self.visible {
            return;
        }

        // Set border style based on active state
        let border_style = if self.active {
            Style::default().fg(Color::Red)
        } else {
            Style::default().fg(Color::White)
        };

        // Modify title to show filter status
        let mut title = self.title.clone();
        if self.filter_mode {
            title = format!("[Filter: {}] {} ", self.filter_text, title);
        } else if !self.filter_text.is_empty() {
            title = format!("[Filtered: {}] {} ", self.filter_text, title);
        }

        // Create outer block with title and active border
        let outer_block = Block::default()
            .title(title)
            .borders(Borders::ALL)
            .border_type(BorderType::Double)
            .border_style(border_style);

        // Create inner block for content area
        let inner_block = Block::default()
            .title(match &self.content {
                NavigatorContent::Services(_) => "Available Services",
                NavigatorContent::Records(_) => "Available Records",
            })
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(Color::White));

        outer_block.render(area, buf);

        // Calculate inner area for content
        let inner_area = Rect::new(area.x + 2, area.y + 2, area.width - 4, area.height - 4);
        inner_block.render(inner_area, buf);

        // Text content area with padding
        let text_area = Rect::new(
            inner_area.x + 2,
            inner_area.y + 1,
            inner_area.width - 4,
            inner_area.height - 2,
        );

        // Calculate visible height (accounting for borders and padding)
        let visible_height = text_area.height as usize;

        // If there's no content after filtering, show a message
        let total_items = self.content_len();
        if total_items == 0 {
            let message = if !self.filter_text.is_empty() {
                "No items match your filter"
            } else {
                "No items available"
            };

            let paragraph = Paragraph::new(message)
                .alignment(Alignment::Center)
                .style(Style::default().fg(Color::Yellow));
            paragraph.render(text_area, buf);
            return;
        }

        // Generate content with scroll indicators and filtered items
        let mut content_lines = Vec::new();

        // Add filter help text at top if in filter mode
        if self.filter_mode {
            content_lines.push("Type to filter, Esc to exit filter mode".to_string());
        }

        // Add scroll up indicator if needed
        if self.scroll_offset > 0 {
            content_lines.push("▲ Scroll up for more".to_string());
        }

        // Calculate how many elements to show based on available height and scroll indicators
        let filter_bar_height = if self.filter_mode { 1 } else { 0 };
        let scroll_indicators_height = if self.scroll_offset > 0 { 1 } else { 0 }
            + if self.scroll_offset + visible_height < total_items {
                1
            } else {
                0
            };

        let available_height =
            visible_height.saturating_sub(scroll_indicators_height + filter_bar_height);

        // Add visible items with proper scrolling
        match &self.filtered_content {
            NavigatorContent::Services(services) => {
                if services.is_empty() && !self.filter_text.is_empty() {
                    content_lines.push("No matching services found".to_string());
                } else {
                    let visible_services = services
                        .iter()
                        .skip(self.scroll_offset)
                        .take(available_height)
                        .enumerate()
                        .map(|(i, service)| {
                            let actual_index = i + self.scroll_offset;
                            if actual_index == self.selected_index {
                                format!("> {}", service)
                            } else {
                                format!("  {}", service)
                            }
                        });

                    content_lines.extend(visible_services);
                }
            }
            NavigatorContent::Records(records) => {
                if records.is_empty() && !self.filter_text.is_empty() {
                    content_lines.push("No matching records found".to_string());
                } else {
                    let visible_records = records
                        .iter()
                        .skip(self.scroll_offset)
                        .take(available_height)
                        .enumerate()
                        .map(|(i, record)| {
                            let actual_index = i + self.scroll_offset;
                            if actual_index == self.selected_index {
                                format!("> {}", record)
                            } else {
                                format!("  {}", record)
                            }
                        });

                    content_lines.extend(visible_records);
                }
            }
        }

        // Add scroll down indicator if needed
        if self.scroll_offset + available_height < total_items {
            content_lines.push("▼ Scroll down for more".to_string());
        }

        // Render the content
        let content_text = content_lines.join("\n");
        let paragraph = Paragraph::new(content_text).alignment(Alignment::Left);
        paragraph.render(text_area, buf);
    }

    /// Handles keyboard input and returns appropriate widget actions
    fn handle_input(&mut self, key_event: KeyEvent) -> Option<WidgetActions> {
        // If we're in filter mode, handle text input
        if self.filter_mode {
            match key_event.code {
                KeyCode::Char(c) => {
                    // Add character to filter unless it's a control character
                    if !key_event.modifiers.contains(KeyModifiers::CONTROL) {
                        self.add_to_filter(c);
                    }
                    Some(WidgetActions::InputBoxEvent(InputBoxEvent::Written(
                        self.filter_text.clone(),
                    )))
                }
                KeyCode::Backspace => {
                    // Remove last character from filter
                    self.remove_from_filter();
                    Some(WidgetActions::InputBoxEvent(InputBoxEvent::Backspace))
                }
                KeyCode::Delete => {
                    // Also remove character
                    self.remove_from_filter();
                    Some(WidgetActions::InputBoxEvent(InputBoxEvent::Delete))
                }
                KeyCode::Esc => {
                    // Exit filter mode but keep the current filter
                    self.filter_mode = false;
                    Some(WidgetActions::AWSServiceNavigatorEvent(
                        AWSServiceNavigatorEvent::Escape,
                        self.widget_type.clone(),
                    ))
                }
                KeyCode::Enter => {
                    // Exit filter mode and keep the filter
                    self.filter_mode = false;
                    Some(WidgetActions::AWSServiceNavigatorEvent(
                        AWSServiceNavigatorEvent::Enter,
                        self.widget_type.clone(),
                    ))
                }
                _ => None,
            }
        } else {
            // Normal navigation mode
            match key_event.code {
                KeyCode::Char('f') if key_event.modifiers.contains(KeyModifiers::CONTROL) => {
                    // Enter filter mode with Ctrl+F
                    self.filter_mode = true;
                    Some(WidgetActions::AWSServiceNavigatorEvent(
                        AWSServiceNavigatorEvent::Enter,
                        self.widget_type.clone(),
                    ))
                }
                KeyCode::Char('/') => {
                    // Alternative way to enter filter mode
                    self.filter_mode = true;
                    Some(WidgetActions::AWSServiceNavigatorEvent(
                        AWSServiceNavigatorEvent::Enter,
                        self.widget_type.clone(),
                    ))
                }
                KeyCode::Esc => {
                    // Clear filter with escape when not in filter mode
                    if !self.filter_text.is_empty() {
                        Some(WidgetActions::AWSServiceNavigatorEvent(
                            AWSServiceNavigatorEvent::Escape,
                            self.widget_type.clone(),
                        ))
                    } else {
                        None
                    }
                }
                KeyCode::Up => {
                    if self.selected_index > 0 {
                        self.selected_index -= 1;
                        self.update_scroll_offset(10); // Will be refined in render
                    }
                    Some(WidgetActions::AWSServiceNavigatorEvent(
                        AWSServiceNavigatorEvent::ArrowUp,
                        self.widget_type.clone(),
                    ))
                }
                KeyCode::Down => {
                    let content_len = self.content_len();
                    if content_len > 0 && self.selected_index < content_len - 1 {
                        self.selected_index += 1;
                        self.update_scroll_offset(10); // Will be refined in render
                    }
                    Some(WidgetActions::AWSServiceNavigatorEvent(
                        AWSServiceNavigatorEvent::ArrowDown,
                        self.widget_type.clone(),
                    ))
                }
                KeyCode::PageUp => {
                    // Jump multiple lines up
                    let jump_size = 5;
                    if self.selected_index > 0 {
                        self.selected_index = self.selected_index.saturating_sub(jump_size);
                        self.update_scroll_offset(10); // Will be refined in render
                    }
                    Some(WidgetActions::AWSServiceNavigatorEvent(
                        AWSServiceNavigatorEvent::PageUp,
                        self.widget_type.clone(),
                    ))
                }
                KeyCode::PageDown => {
                    // Jump multiple lines down
                    let jump_size = 5;
                    let content_len = self.content_len();
                    if content_len > 0 && self.selected_index < content_len - 1 {
                        self.selected_index =
                            (self.selected_index + jump_size).min(content_len - 1);
                        self.update_scroll_offset(10); // Will be refined in render
                    }
                    Some(WidgetActions::AWSServiceNavigatorEvent(
                        AWSServiceNavigatorEvent::PageDown,
                        self.widget_type.clone(),
                    ))
                }
                KeyCode::Enter => Some(WidgetActions::AWSServiceNavigatorEvent(
                    AWSServiceNavigatorEvent::Enter,
                    self.widget_type.clone(),
                )),
                KeyCode::Home => {
                    // Jump to start
                    if self.selected_index > 0 {
                        self.selected_index = 0;
                        self.scroll_offset = 0;
                    }
                    Some(WidgetActions::AWSServiceNavigatorEvent(
                        AWSServiceNavigatorEvent::Home,
                        self.widget_type.clone(),
                    ))
                }
                KeyCode::End => {
                    // Jump to end
                    let content_len = self.content_len();
                    if content_len > 0 && self.selected_index < content_len - 1 {
                        self.selected_index = content_len - 1;
                        self.update_scroll_offset(10); // Will be refined in render
                    }
                    Some(WidgetActions::AWSServiceNavigatorEvent(
                        AWSServiceNavigatorEvent::End,
                        self.widget_type.clone(),
                    ))
                }
                _ => None,
            }
        }
    }

    /// Processes widget events and returns actions as needed
    fn process_event(&mut self, event: WidgetActions) -> Option<WidgetActions> {
        match event {
            WidgetActions::AWSServiceNavigatorEvent(event, _) => match event {
                AWSServiceNavigatorEvent::ArrowUp => {
                    // Already handled in handle_input
                    None
                }
                AWSServiceNavigatorEvent::ArrowDown => {
                    // Already handled in handle_input
                    None
                }
                AWSServiceNavigatorEvent::PageUp => {
                    // Already handled in handle_input
                    None
                }
                AWSServiceNavigatorEvent::PageDown => {
                    // Already handled in handle_input
                    None
                }
                AWSServiceNavigatorEvent::Home => {
                    // Already handled in handle_input
                    None
                }
                AWSServiceNavigatorEvent::End => {
                    // Already handled in handle_input
                    None
                }
                AWSServiceNavigatorEvent::Enter => self.selected_item(),
                AWSServiceNavigatorEvent::Escape => {
                    if self.filter_mode {
                        self.filter_mode = false;
                        self.clear_filter(); // Clear the filter text when exiting filter mode
                    }
                    None
                }
                _ => None,
            },
            WidgetActions::InputBoxEvent(input_event) => match input_event {
                InputBoxEvent::Written(text) => {
                    if self.filter_mode {
                        self.apply_filter(&text);
                    }
                    None
                }
                InputBoxEvent::Backspace | InputBoxEvent::Delete => {
                    if self.filter_mode {
                        self.remove_from_filter();
                    }
                    None
                }
                _ => None,
            },
            _ => None,
        }
    }

    /// Returns help items based on the current state
    fn get_help_items(&self) -> Vec<(String, String)> {
        let mut items = vec![];

        if self.filter_mode {
            // Filter mode help
            items.push(("Type".to_string(), "Filter".to_string()));
            items.push(("Esc".to_string(), "Exit filter".to_string()));
            items.push(("Enter".to_string(), "Apply filter".to_string()));
        } else {
            // Standard navigation help
            items.push(("Enter".to_string(), "Select".to_string()));
            items.push(("Ctrl+F".to_string(), "Filter".to_string()));
            items.push(("/".to_string(), "Filter".to_string()));

            if !self.filter_text.is_empty() {
                items.push(("Esc".to_string(), "Clear filter".to_string()));
            }

            items.push(("↑/↓".to_string(), "Navigate".to_string()));
            items.push(("PgUp/PgDn".to_string(), "Scroll".to_string()));
            items.push(("Home/End".to_string(), "Jump to start/end".to_string()));
        }

        items
    }

    /// Returns whether the widget is visible
    fn is_visible(&self) -> bool {
        self.visible
    }

    /// Sets the active state of the widget
    fn set_active(&mut self, active: bool) {
        self.active = active;
    }

    /// Sets the widget to inactive
    fn set_inactive(&mut self) {
        self.active = false;
    }

    /// Sets the visibility of the widget
    fn set_visible(&mut self, visible: bool) {
        self.visible = visible;
    }

    /// Sets the title of the widget
    fn set_title(&mut self, title: String) {
        self.title = title;
    }

    /// Returns self as Any for downcasting
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }

    /// Returns whether the widget is active
    fn is_active(&self) -> bool {
        self.active
    }
}