use crate::{
    event_managment::event::{Event, PopupEvent, TabActions, TabEvent, WidgetActions},
    services::read_config,
    widgets::WidgetExt,
};
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    buffer::Buffer,
    layout::Alignment,
    layout::Rect,
    style::{Color, Style},
    text::Line,
    widgets::{Block, Clear, Paragraph, Widget},
};
use std::any::Any;
use serde_json;

const POPUP_MARGIN: u16 = 5;
const MIN_POPUP_WIDTH: u16 = 20;
const MIN_POPUP_HEIGHT: u16 = 10;

#[derive(Clone, Debug)]
pub enum PopupContent {
    Profiles(Vec<String>),
    Details(String),
}

impl PopupContent {
    pub fn len(&self) -> usize {
        match self {
            PopupContent::Profiles(profiles) => profiles.len(),
            PopupContent::Details(_) => 0,
        }
    }

    pub fn get(&self, index: usize) -> Option<&String> {
        match self {
            PopupContent::Profiles(profiles) => profiles.get(index),
            PopupContent::Details(_) => None,
        }
    }
}

#[derive(Debug)]
pub struct PopupWidget {
    title: String,
    profile_name: Option<String>,
    profile_list: PopupContent,
    selected_index: usize,
    active: bool,
    visible: bool,
    event_sender: tokio::sync::mpsc::UnboundedSender<Event>,
}

impl PopupWidget {
    pub fn new(
        title: &str,
        visible: bool,
        active: bool,
        event_sender: tokio::sync::mpsc::UnboundedSender<Event>,
    ) -> Self {
        let profiles = match read_config::get_aws_profiles() {
            Ok(profiles) => PopupContent::Profiles(profiles),
            Err(_) => PopupContent::Profiles(vec!["No profiles found".to_string()]),
        };

        Self {
            title: title.to_string(),
            profile_name: None,
            profile_list: profiles,
            selected_index: 0,
            active,
            visible,
            event_sender,
        }
    }
    pub fn set_profile_list(&mut self, profiles: PopupContent) {
        self.profile_list = profiles;
    }
    fn calculate_popup_area(&self, area: Rect) -> Option<Rect> {
        if area.width <= MIN_POPUP_WIDTH || area.height <= MIN_POPUP_HEIGHT {
            return None;
        }

        // Make the popup larger for JSON content
        let (margin_x, margin_y) = match self.profile_list {
            PopupContent::Details(_) => (POPUP_MARGIN / 2, POPUP_MARGIN / 2),
            _ => (POPUP_MARGIN, POPUP_MARGIN),
        };

        Some(Rect::new(
            area.x.saturating_add(margin_x),
            area.y.saturating_add(margin_y),
            area.width.saturating_sub(margin_x * 2),
            area.height.saturating_sub(margin_y * 2),
        ))
    }

    fn calculate_content_area(&self, popup_area: Rect) -> Rect {
        Rect::new(
            popup_area.x.saturating_add(2),
            popup_area.y.saturating_add(1),
            popup_area.width.saturating_sub(4),
            popup_area.height.saturating_sub(2),
        )
    }

    fn render_profiles(&self) -> String {
        match &self.profile_list {
            PopupContent::Profiles(profiles) => profiles
                .iter()
                .enumerate()
                .map(|(i, profile)| {
                    if i == self.selected_index {
                        format!("> {}", profile)
                    } else {
                        format!("  {}", profile)
                    }
                })
                .collect::<Vec<_>>()
                .join("\n"),
            PopupContent::Details(content) => {
                // Parse the JSON string
                match serde_json::from_str::<serde_json::Value>(content) {
                    Ok(json) => {
                        // Pretty print with proper indentation
                        serde_json::to_string_pretty(&json)
                            .unwrap_or_else(|_| content.clone())
                    }
                    Err(_) => content.clone(),
                }
            }
        }
    }
}

impl WidgetExt for PopupWidget {
    fn render(&self, area: Rect, buf: &mut Buffer) {
        if !self.visible {
            return;
        }

        let popup_area = match self.calculate_popup_area(area) {
            Some(area) => area,
            None => return,
        };

        let content_area = self.calculate_content_area(popup_area);

        // Render popup background and border
        buf.set_style(popup_area, Style::default().bg(Color::Black));
        Clear.render(popup_area, buf);

        let border_style = if self.active {
            Style::default().fg(Color::Red)
        } else {
            Style::default()
        };

        Block::bordered()
            .title(Line::from(self.title.as_str()))
            .border_style(border_style)
            .render(popup_area, buf);

        // Render profiles list
        let profiles_text = self.render_profiles();
        Paragraph::new(profiles_text)
            .block(Block::default())
            .style(Style::default().fg(Color::White).bg(Color::Black))
            .alignment(Alignment::Left)
            .render(content_area, buf);
    }

    fn handle_input(&mut self, key_event: KeyEvent) -> Option<WidgetActions> {
        match key_event.code {
            KeyCode::Up => Some(WidgetActions::PopupEvent(PopupEvent::ArrowUp)),
            KeyCode::Down => Some(WidgetActions::PopupEvent(PopupEvent::ArrowDown)),
            KeyCode::Enter => Some(WidgetActions::PopupEvent(PopupEvent::Enter)),
            KeyCode::Esc => Some(WidgetActions::PopupEvent(PopupEvent::Escape)),
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
    fn process_event(&mut self, event: WidgetActions)  -> Option<WidgetActions> {
        match event {
            WidgetActions::PopupEvent(event) => match event {
                PopupEvent::ArrowUp => {
                    if self.selected_index > 0 {
                        self.selected_index -= 1;
                    }
                    None
                }
                PopupEvent::ArrowDown => {
                    if self.selected_index < self.profile_list.len() - 1 {
                        self.selected_index += 1;
                    }
                    None
                }
                PopupEvent::Enter => {
                    if let Some(profile) = self.profile_list.get(self.selected_index) {
                        self.profile_name = Some(profile.clone());
                        return Some(WidgetActions::PopupEvent(PopupEvent::SelectedItem(
                            self.profile_name.clone().unwrap(),
                        )));
                    }
                    None
                }
                PopupEvent::Escape => {
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
    fn set_title(&mut self, title: String) {
        todo!()
    }
}
