use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Style},
    widgets::{Block, BorderType, Paragraph, Widget},
};
use ratatui::layout::Alignment;
use ratatui::text::{Line, Span};
use ratatui::widgets::Tabs;
use crate::widgets::WidgetExt;
use crate::widgets::paragraph::ParagraphWidget;

pub struct Tab {
    pub name: String,
    pub show_popup: bool,
    pub selected_profile_index: usize,
    pub widget: Vec<Box<dyn WidgetExt>>,
}

impl Tab {
    pub fn new(name: &str, content: &str) -> Self {
        Self {
            name: name.to_string(),
            show_popup: true,
            selected_profile_index: 0,
            widget: vec![
                Box::new(ParagraphWidget::new(content)),
            ],
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

    pub fn render(&self, area: Rect, buf: &mut Buffer, tab_titles: Vec<String>, active_tab: usize) {
        let block = Block::bordered()
            .title_alignment(Alignment::Center)
            .border_type(BorderType::Rounded);

        let tab_titles_component: Vec<Line> = tab_titles
            .iter()
            .map(|t| Line::from(Span::styled(t, Style::default().fg(Color::Yellow))))
            .collect();

        let tabs = Tabs::new(tab_titles_component)
            .block(block.clone())
            .highlight_style(Style::default().fg(Color::LightGreen))
            .select(active_tab);

        tabs.render(Rect::new(area.x, area.y, area.width, 3), buf);
        
        // Render widgets
        let widget_area = Rect::new(area.x, area.y + 3, area.width, area.height - 3);
        for widget in &self.widget {
            widget.render(widget_area, buf);
        }
    }
}