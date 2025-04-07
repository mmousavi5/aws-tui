use crate::event_managment::event::{WidgetActions, WidgetEventType, WidgetType};
use crate::{event_managment::event::AWSServiceNavigatorEvent, widgets::WidgetExt};
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    buffer::Buffer,
    layout::{Alignment, Rect},
    style::{Color, Style},
    widgets::{Block, BorderType, Borders, Paragraph, Widget},
};
use std::any::Any;

#[derive(Clone)]
pub enum NavigatorContent {
    Services(Vec<WidgetEventType>),
    Records(Vec<String>),
}

pub struct AWSServiceNavigator {
    title: String,
    widget_type: WidgetType,
    content: NavigatorContent,
    selected_index: usize,
    scroll_offset: usize,
    active: bool,
    visible: bool,
}

impl AWSServiceNavigator {
    pub fn new(widget_type: WidgetType, active: bool, content: NavigatorContent) -> Self {
        Self {
            title: "AWS Services".to_string(),
            widget_type,
            content,
            selected_index: 0,
            scroll_offset: 0,
            active,
            visible: true,
        }
    }

    fn content_len(&self) -> usize {
        match &self.content {
            NavigatorContent::Services(services) => services.len(),
            NavigatorContent::Records(records) => records.len(),
        }
    }

    fn selected_item(&self) -> Option<WidgetActions> {
        match &self.content {
            NavigatorContent::Services(services) => {
                services.get(self.selected_index).map(|service| {
                    WidgetActions::AWSServiceNavigatorEvent(
                        AWSServiceNavigatorEvent::SelectedItem(service.clone()),
                        self.widget_type,
                    )
                })
            }
            NavigatorContent::Records(records) => records.get(self.selected_index).map(|record| {
                WidgetActions::AWSServiceNavigatorEvent(
                    AWSServiceNavigatorEvent::SelectedItem(WidgetEventType::RecordSelected(
                        record.clone(),
                    )),
                    self.widget_type,
                )
            }),
        }
    }

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

    pub fn set_content(&mut self, content: NavigatorContent) {
        self.content = content;
        self.selected_index = 0;
        self.scroll_offset = 0;
    }
}

impl WidgetExt for AWSServiceNavigator {
    fn render(&self, area: Rect, buf: &mut Buffer) {
        if !self.visible {
            return;
        }

        let border_style = if self.active {
            Style::default().fg(Color::Red)
        } else {
            Style::default().fg(Color::White)
        };

        let outer_block = Block::default()
            .title(self.title.as_str())
            .borders(Borders::ALL)
            .border_type(BorderType::Double)
            .border_style(border_style);

        let inner_block = Block::default()
            .title(match &self.content {
                NavigatorContent::Services(_) => "Available Services",
                NavigatorContent::Records(_) => "Available Records",
            })
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(Color::White));

        outer_block.render(area, buf);

        let inner_area = Rect::new(area.x + 2, area.y + 2, area.width - 4, area.height - 4);

        inner_block.render(inner_area, buf);

        let text_area = Rect::new(
            inner_area.x + 2,
            inner_area.y + 1,
            inner_area.width - 4,
            inner_area.height - 2,
        );

        // Calculate visible height (accounting for borders and padding)
        let visible_height = text_area.height as usize;

        // Generate content with scroll indicators
        let mut content_lines = Vec::new();
        let total_items = self.content_len();

        // Add scroll up indicator if needed
        if self.scroll_offset > 0 {
            content_lines.push("▲ Scroll up for more".to_string());
        }

        // Calculate how many elements to show based on available height and scroll indicators
        let scroll_indicators_height = if self.scroll_offset > 0 { 1 } else { 0 }
            + if self.scroll_offset + visible_height < total_items {
                1
            } else {
                0
            };

        let available_height = visible_height.saturating_sub(scroll_indicators_height);

        // Add visible items with proper scrolling
        match &self.content {
            NavigatorContent::Services(services) => {
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
            NavigatorContent::Records(records) => {
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

        // Add scroll down indicator if needed
        if self.scroll_offset + available_height < total_items {
            content_lines.push("▼ Scroll down for more".to_string());
        }

        let content_text = content_lines.join("\n");
        let paragraph = Paragraph::new(content_text).alignment(Alignment::Left);
        paragraph.render(text_area, buf);
    }

    fn handle_input(&mut self, key_event: KeyEvent) -> Option<WidgetActions> {
        match key_event.code {
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
                    self.selected_index = (self.selected_index + jump_size).min(content_len - 1);
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
                    self.set_visible(false);
                    None
                }
                _ => None,
            },
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

    fn set_title(&mut self, title: String) {
        self.title = title;
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }

    fn is_active(&self) -> bool {
        self.active
    }
}
