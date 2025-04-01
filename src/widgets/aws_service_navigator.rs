use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    widgets::{Block, BorderType, Borders, Clear, Paragraph, Widget},
    style::{Style, Color},
    layout::Alignment,
};
use crate::widgets::WidgetExt;

pub struct AWSServiceNavigator {
    services: Vec<String>,
    selected_index: usize,
    active: bool,
    visible: bool,
}

impl AWSServiceNavigator {
    pub fn new(active: bool) -> Self {
        Self {
            services: vec![
                "S3".to_string(),
                "DynamoDB".to_string(),
            ],
            selected_index: 0,
            active,
            visible: true,
        }
    }

    pub fn selected_service(&self) -> Option<&String> {
        self.services.get(self.selected_index)
    }
}

impl WidgetExt for AWSServiceNavigator {
    fn render(&self, area: Rect, buf: &mut Buffer) {
        let border_style = if self.active {
            Style::default().fg(Color::Red)
        } else {
            Style::default()
        };

        // Create outer block with double border
        let outer_block = Block::default()
            .title("AWS Services Panel")
            .borders(Borders::ALL)
            .border_type(BorderType::Double)
            .border_style(border_style);

        // Create inner block with rounded borders
        let inner_block = Block::default()
            .title("Available Services")
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(border_style);

        // Render outer block
        outer_block.render(area, buf);

        // Calculate inner area with padding
        let inner_area = Rect::new(
            area.x + 2,
            area.y + 2,
            area.width - 4,
            area.height - 4,
        );

        // Render inner block
        inner_block.render(inner_area, buf);

        // Calculate text area with additional padding
        let text_area = Rect::new(
            inner_area.x + 2,
            inner_area.y + 1,
            inner_area.width - 4,
            inner_area.height - 2,
        );

        // Create the services text with selection indicator
        let services_text = self.services
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
            .join("\n");

        let paragraph = Paragraph::new(services_text)
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
                if self.selected_index < self.services.len() - 1 {
                    self.selected_index += 1;
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
}