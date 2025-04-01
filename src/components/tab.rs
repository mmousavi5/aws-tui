use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Style},
    widgets::{Block, BorderType, Widget},
};
use ratatui::layout::Alignment;
use ratatui::text::{Line, Span};
use ratatui::widgets::Tabs;
use crate::widgets::WidgetExt;
use crate::widgets::paragraph::ParagraphWidget;
use crate::widgets::popup::PopupWidget;

pub struct Tab {
    pub name: String,
    pub show_popup: bool,
    pub selected_profile_index: usize,
    pub widget: Vec<Box<dyn WidgetExt>>,
    pub active_widget: usize,
}

impl Tab {
    pub fn new(name: &str, content: &str) -> Self {
        Self {
            name: name.to_string(),
            show_popup: true,
            selected_profile_index: 0,
            widget: vec![
                Box::new(ParagraphWidget::new(content, true)), Box::new(PopupWidget::new(content, false))
            ],
            active_widget: 0,
        } 
    }

    pub fn handle_input(&mut self, event: KeyEvent) {
        match event.code {
            KeyCode::Char('t') => {
                self.widget[self.active_widget].set_inactive();
                self.active_widget = (self.active_widget + 1) % self.widget.len();
                self.widget[self.active_widget].set_active();
            }
            KeyCode::BackTab => {
                if self.active_widget == 0 {
                    self.active_widget = self.widget.len() - 1;
                } else {
                    self.active_widget -= 1;
                }
            }
            _ => {}
        }
        // Handle input for the active widget
        if let Some(active_widget) = self.widget.get_mut(self.active_widget) {
            if active_widget.is_visible() {
            active_widget.handle_input(event);
            }
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
        let popup_area =  Rect::new(area.x + 5, area.y + 5, area.width - 10, area.height - 10);
        for widget in &self.widget {
            match widget.as_ref() {
                // Render the active widget
                _ if widget.is_visible() => {
                    widget.render(widget_area, buf);
                }
                // Render the popup if show_popup is true
                _ if widget.is_visible() => {
                    widget.render(popup_area, buf);
                }
                _ => {}
            }
        }
    }
}