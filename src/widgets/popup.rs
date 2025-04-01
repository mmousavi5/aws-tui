use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    widgets::{Block, BorderType, Paragraph, Widget, Clear},
    style::{Style, Color},
};
use ratatui::layout::Alignment;
use crate::widgets::WidgetExt;
use crate::event_managment::event::{AppEvent, Event, EventHandler};
use crate::services::read_config;

pub struct PopupWidget { 
    pub text: String,
    pub profile_name: Option<String>,
    pub profile_list: Vec<String>,
    pub selected_profile_index: usize,
    count: usize,
    active: bool,
    visible: bool,
    pub unbounded_channel_sender: tokio::sync::mpsc::UnboundedSender<Event>,
}

impl PopupWidget {
    pub fn new(text: &str, active:bool, unbounded_channel_sender: tokio::sync::mpsc::UnboundedSender<Event>) -> Self {
        let aws_profiles = read_config::get_aws_profiles()
            .unwrap_or_else(|_| vec!["No profiles found".to_string()]);
        Self {
            text: text.to_string(),
            profile_name: None,
            profile_list: aws_profiles,
            selected_profile_index: 0,
            count: 0,
            active,
            visible: true,
            unbounded_channel_sender,  // Add this line
        }
    }
}

impl WidgetExt for PopupWidget {
    fn render(&self, area: Rect, buf: &mut Buffer) {
        let border_style = if self.active {
            Style::default().fg(Color::Red)
        } else {
            Style::default()
        };

        let popup_block = Block::bordered()
            .title("AWS Profiles")
            .border_style(border_style);
            
        let popup_area = Rect::new(area.x + 5, area.y + 5, area.width - 10, area.height - 10);
        let paragraph_area = Rect::new(
            popup_area.x + 2, 
            popup_area.y + 1, 
            popup_area.width - 5, 
            popup_area.height - 2
        );

        buf.set_style(popup_area, Style::default().bg(Color::Black));
        Clear.render(popup_area, buf);
        popup_block.render(popup_area, buf);

        // Create the profiles text with selection indicator
        let profiles_text = self.profile_list
            .iter()
            .enumerate()
            .map(|(i, profile)| {
                if i == self.selected_profile_index {
                    format!("> {}", profile)
                } else {
                    format!("  {}", profile)
                }
            })
            .collect::<Vec<_>>()
            .join("\n");

        let profiles_paragraph = Paragraph::new(profiles_text)
            .block(Block::default())
            .style(Style::default().fg(Color::White).bg(Color::Black))
            .alignment(Alignment::Left);

        profiles_paragraph.render(paragraph_area, buf);
    }

    fn handle_input(&mut self, key_event: KeyEvent) {
        match key_event.code {
            KeyCode::Up => {
                if self.selected_profile_index > 0 {
                    self.selected_profile_index -= 1;
                }
            }
            KeyCode::Down => {
                if self.selected_profile_index < self.profile_list.len() - 1 {
                    self.selected_profile_index += 1;
                }
            }
            KeyCode::Enter => {
                // Handle profile selection
                self.visible = false;
                let selected_profile = self.profile_list[self.selected_profile_index].clone();
                self.profile_name = Some(selected_profile.clone());
                // Send the profile event
                self.unbounded_channel_sender.send(Event::AWSProfileEvent(selected_profile));
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
}