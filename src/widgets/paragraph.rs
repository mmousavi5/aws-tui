use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    widgets::{Block, BorderType, Paragraph, Widget},
};
use ratatui::layout::Alignment;
use ratatui::widgets::{Borders, Wrap};
use crate::widgets::WidgetExt;

pub struct ParagraphWidget {
    pub text:  String,
    pub show_popup: bool,
    pub selected_profile_index: usize,
}
impl ParagraphWidget {
    pub fn new(text: &str) -> Self {
        Self {
            text: text.to_string(),
            show_popup: true,
            selected_profile_index: 0,
        }
    }

    pub fn handle_input(&mut self, key_event: KeyEvent) {
        match key_event.code {
            KeyCode::Up => {
                if self.selected_profile_index > 0 {
                    self.selected_profile_index -= 1;
                }
            }
            _ => {}
        }
    }
}
impl WidgetExt for ParagraphWidget {
    fn render(&self, area: Rect, buf: &mut Buffer) {
        let block = Block::default()
            .title("Paragraph")
            .border_type(BorderType::Rounded)
            .borders(Borders::ALL);

        let paragraph = Paragraph::new(self.text.as_str())
            .block(block)
            .alignment(Alignment::Left)
            .wrap(Wrap { trim: true });

        paragraph.render(area, buf);
    }
}