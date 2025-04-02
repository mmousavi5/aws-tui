use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    widgets::{Block, BorderType, Paragraph, Widget, Clear},
    style::{Style, Color},
    layout::Alignment,
    text::Line,
};
use crate::{
    widgets::WidgetExt,
    event_managment::event::{AppEvent, Event, EventHandler},
    services::read_config,
};
use std::any::Any;

const POPUP_MARGIN: u16 = 5;
const MIN_POPUP_WIDTH: u16 = 20;
const MIN_POPUP_HEIGHT: u16 = 10;

#[derive(Debug)]
pub struct PopupWidget {
    title: String,
    profile_name: Option<String>,
    profile_list: Vec<String>,
    selected_index: usize,
    active: bool,
    visible: bool,
    event_sender: tokio::sync::mpsc::UnboundedSender<Event>,
}

impl PopupWidget {
    pub fn new(
        title: &str, 
        active: bool, 
        event_sender: tokio::sync::mpsc::UnboundedSender<Event>
    ) -> Self {
        let profiles = read_config::get_aws_profiles()
            .unwrap_or_else(|_| vec!["No profiles found".to_string()]);
            
        Self {
            title: title.to_string(),
            profile_name: None,
            profile_list: profiles,
            selected_index: 0,
            active,
            visible: true,
            event_sender,
        }
    }

    fn calculate_popup_area(&self, area: Rect) -> Option<Rect> {
        if area.width <= MIN_POPUP_WIDTH || area.height <= MIN_POPUP_HEIGHT {
            return None;
        }

        Some(Rect::new(
            area.x.saturating_add(POPUP_MARGIN),
            area.y.saturating_add(POPUP_MARGIN),
            area.width.saturating_sub(POPUP_MARGIN * 2),
            area.height.saturating_sub(POPUP_MARGIN * 2),
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
        self.profile_list
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
            .join("\n")
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

    fn handle_input(&mut self, key_event: KeyEvent) {
        match key_event.code {
            KeyCode::Up => {
                self.selected_index = self.selected_index.saturating_sub(1);
            }
            KeyCode::Down => {
                if self.selected_index < self.profile_list.len().saturating_sub(1) {
                    self.selected_index += 1;
                }
            }
            KeyCode::Enter => {
                self.visible = false;
                if let Some(profile) = self.profile_list.get(self.selected_index) {
                    self.profile_name = Some(profile.clone());
                    let _ = self.event_sender.send(Event::AWSProfileEvent(profile.clone()));
                }
            }
            KeyCode::Esc => {
                self.visible = false;
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