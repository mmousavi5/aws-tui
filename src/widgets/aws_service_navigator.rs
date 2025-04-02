use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    buffer::Buffer,
    layout::{Alignment, Rect},
    style::{Color, Style},
    widgets::{self, Block, BorderType, Borders, Clear, Paragraph, Widget},
};
use crate::{services, widgets::WidgetExt};
use crate::event_managment::event::WidgetEventType;
use crate::event_managment::event::Event;
use std::any::Any;

#[derive(Clone)]
pub enum NavigatorContent {
    Services(Vec<WidgetEventType>),
    Records(Vec<String>),
}

#[derive(Hash, Eq, PartialEq, Clone)]
pub enum WidgetType {
    Default,
    AWSServiceNavigator,
    AWSService,
    S3,
    DynamoDB,
}

pub struct AWSServiceNavigator {
    widget_type: WidgetType,
    content: NavigatorContent,
    selected_index: usize,
    active: bool,
    visible: bool,
    pub unbounded_channel_sender: tokio::sync::mpsc::UnboundedSender<Event>,
}

impl AWSServiceNavigator {
    pub fn new(
        widget_type: WidgetType,
        active: bool,
        unbounded_channel_sender: tokio::sync::mpsc::UnboundedSender<Event>,
        content: NavigatorContent,
    ) -> Self {
        Self {
            widget_type,
            content,
            selected_index: 0,
            active,
            visible: true,
            unbounded_channel_sender,
        }
    }

    fn content_len(&self) -> usize {
        match &self.content {
            NavigatorContent::Services(services) => services.len(),
            NavigatorContent::Records(records) => records.len(),
        }
    }

    fn selected_item(&self) -> Option<Event> {
        match &self.content {
            NavigatorContent::Services(services) => services
                .get(self.selected_index)
                .map(|service| Event::WidgetEvent(service.clone())),
            NavigatorContent::Records(records) => records
                .get(self.selected_index)
                .map(|record| Event::WidgetEvent(WidgetEventType::RecordSelected(record.clone()))),
        }
    }

    pub fn set_content(&mut self, content: NavigatorContent) {
        self.content = content;
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
            Style::default()
        };

        let outer_block = Block::default()
            .title("AWS Services Panel")
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
            .border_style(border_style);

        outer_block.render(area, buf);

        let inner_area = Rect::new(
            area.x + 2,
            area.y + 2,
            area.width - 4,
            area.height - 4,
        );

        inner_block.render(inner_area, buf);

        let text_area = Rect::new(
            inner_area.x + 2,
            inner_area.y + 1,
            inner_area.width - 4,
            inner_area.height - 2,
        );

        let content_text = match &self.content {
            NavigatorContent::Services(services) => services
                .iter()
                .enumerate()
                .map(|(i, service)| {
                    if i == self.selected_index {
                        format!("> {}", service)
                    } else {
                        format!("  {}", service)
                    }
                })
                .collect::<Vec<_>>()
                .join("\n"),
            NavigatorContent::Records(records) => records
                .iter()
                .enumerate()
                .map(|(i, record)| {
                    if i == self.selected_index {
                        format!("> {}", record)
                    } else {
                        format!("  {}", record)
                    }
                })
                .collect::<Vec<_>>()
                .join("\n"),
        };

        let paragraph = Paragraph::new(content_text)
            .alignment(Alignment::Left);

        paragraph.render(text_area, buf);
    }

    fn handle_input(&mut self, key_event: KeyEvent) {
        match key_event.code {
            KeyCode::Up => {
                if self.selected_index > 0 {
                    self.selected_index -= 1;
                }
            }
            KeyCode::Down => {
                if self.selected_index < self.content_len().saturating_sub(1) {
                    self.selected_index += 1;
                }
            }
            KeyCode::Enter => {
                if let Some(event) = self.selected_item() {
                    if let Err(e) = self.unbounded_channel_sender.send(event) {
                        eprintln!("Error sending event: {}", e);
                    }
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
}