use crate::event_managment::event::{Event, TabEvent, WidgetActions, WidgetEventType, WidgetType};
use crate::{
    event_managment::event::{AWSServiceNavigatorEvent, TabActions},
    widgets::WidgetExt,
};
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
    widget_type: WidgetType,
    content: NavigatorContent,
    selected_index: usize,
    active: bool,
    visible: bool,
    event_sender: tokio::sync::mpsc::UnboundedSender<Event>,
}

impl AWSServiceNavigator {
    pub fn new(
        widget_type: WidgetType,
        active: bool,
        event_sender: tokio::sync::mpsc::UnboundedSender<Event>,
        content: NavigatorContent,
    ) -> Self {
        Self {
            widget_type,
            content,
            selected_index: 0,
            active,
            visible: true,
            event_sender,
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
                        self.widget_type
                    )
                })
            }
            NavigatorContent::Records(records) => records.get(self.selected_index).map(|record| {
                WidgetActions::AWSServiceNavigatorEvent(
                    AWSServiceNavigatorEvent::SelectedItem(
                        WidgetEventType::RecordSelected(record.clone())
                    ),
                    self.widget_type
                )
            }),
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
            Style::default().fg(Color::White)
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

        let paragraph = Paragraph::new(content_text).alignment(Alignment::Left);

        paragraph.render(text_area, buf);
    }

    fn handle_input(&mut self, key_event: KeyEvent) -> Option<WidgetActions> {
        match key_event.code {
            KeyCode::Up => Some(WidgetActions::AWSServiceNavigatorEvent(
                AWSServiceNavigatorEvent::ArrowUp,
                self.widget_type.clone(),
            )),
            KeyCode::Down => Some(WidgetActions::AWSServiceNavigatorEvent(
                AWSServiceNavigatorEvent::ArrowDown,
                self.widget_type.clone(),
            )),
            KeyCode::Enter => Some(WidgetActions::AWSServiceNavigatorEvent(
                AWSServiceNavigatorEvent::Enter,
                self.widget_type.clone(),
            )),
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
    fn process_event(&mut self, event: WidgetActions)  -> Option<WidgetActions>{
        match event {
            WidgetActions::AWSServiceNavigatorEvent(event, _) => match event {
                AWSServiceNavigatorEvent::ArrowUp => {
                    if self.selected_index > 0 {
                        self.selected_index -= 1;
                    }
                    None
                }
                AWSServiceNavigatorEvent::ArrowDown => {
                    if self.selected_index < self.content_len().saturating_sub(1) {
                        self.selected_index += 1;
                    }
                    None
                }
                AWSServiceNavigatorEvent::Enter => {
                    self.selected_item()
                }
                AWSServiceNavigatorEvent::Escape => {
                    self.set_visible(false);
                    None
                }
                _ => {None}
            },
            _ => {None}
        }
    }
    fn is_active(&self) -> bool {
        self.active
    }
}
