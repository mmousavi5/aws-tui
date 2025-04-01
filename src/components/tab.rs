use std::rc::Rc;
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    buffer::Buffer,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Tabs, Widget},
};
use crate::{
    event_managment::event::Event,
    widgets::{
        WidgetExt,
        paragraph::ParagraphWidget,
        popup::PopupWidget,
        aws_service_navigator::AWSServiceNavigator,
    },
};

// Constants
const TAB_HEIGHT: u16 = 3;
const POPUP_PADDING: u16 = 5;

pub struct Tab {
    pub name: String,
    show_popup: bool,
    widgets: Vec<Box<dyn WidgetExt>>,
    active_widget_index: usize,
    event_sender: tokio::sync::mpsc::UnboundedSender<Event>,
}

impl Tab {
    pub fn new(
        name: &str, 
        content: &str, 
        event_sender: tokio::sync::mpsc::UnboundedSender<Event>
    ) -> Self {
        Self {
            name: name.to_string(),
            show_popup: true,
            widgets: vec![
                Box::new(AWSServiceNavigator::new(false)),
                Box::new(ParagraphWidget::new(content, false)),
                Box::new(PopupWidget::new(content, true, event_sender.clone())),
            ],
            active_widget_index: 2,
            event_sender,
        }
    }

    // Public getters and setters
    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn set_name(&mut self, name: String) {
        self.name = name;
    }

    pub fn toggle_popup(&mut self) {
        self.show_popup = !self.show_popup;
    }

    // Public methods
    pub fn handle_input(&mut self, event: KeyEvent) {
        match event.code {
            KeyCode::Char('t') => self.cycle_active_widget_forward(),
            KeyCode::BackTab => self.cycle_active_widget_backward(),
            _ => self.handle_widget_input(event),
        }
    }

    pub fn render(&self, area: Rect, buf: &mut Buffer, tab_titles: Vec<String>, active_tab: usize) {
        self.render_tab_bar(area, buf, tab_titles, active_tab);
        let content_area = self.get_content_area(area);
        
        self.render_widgets(content_area, buf);
    }

    // Private methods
    fn cycle_active_widget_forward(&mut self) {
        self.widgets[self.active_widget_index].set_inactive();
        self.active_widget_index = (self.active_widget_index + 1) % self.widgets.len();
        self.widgets[self.active_widget_index].set_active();
    }

    fn cycle_active_widget_backward(&mut self) {
        self.widgets[self.active_widget_index].set_inactive();
        self.active_widget_index = if self.active_widget_index == 0 {
            self.widgets.len() - 1
        } else {
            self.active_widget_index - 1
        };
        self.widgets[self.active_widget_index].set_active();
    }

    fn handle_widget_input(&mut self, event: KeyEvent) {
        if let Some(active_widget) = self.widgets.get_mut(self.active_widget_index) {
            if active_widget.is_visible() {
                active_widget.handle_input(event);
            }
        }
    }

    fn render_tab_bar(&self, area: Rect, buf: &mut Buffer, tab_titles: Vec<String>, active_tab: usize) {
        let tab_block = Block::bordered()
            .title_alignment(Alignment::Center)
            .border_type(BorderType::Rounded);

        let tab_titles: Vec<Line> = tab_titles.iter()
            .map(|t| Line::from(Span::styled(t, Style::default().fg(Color::Yellow))))
            .collect();

        let tabs = Tabs::new(tab_titles)
            .block(tab_block)
            .highlight_style(Style::default().fg(Color::LightGreen))
            .select(active_tab);

        let tab_area = Rect::new(area.x, area.y, area.width, TAB_HEIGHT);
        tabs.render(tab_area, buf);
    }

    fn get_content_area(&self, area: Rect) -> Rect {
        Rect::new(
            area.x,
            area.y + TAB_HEIGHT,
            area.width,
            area.height - TAB_HEIGHT
        )
    }

    fn create_layout(&self, area: Rect) -> Vec<Rect> {
        Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(20),
                Constraint::Percentage(80),
            ])
            .split(area)
            .to_vec()
    }

    fn render_widgets(&self, area: Rect, buf: &mut Buffer) {
        let popup_area = self.calculate_popup_area(area);
        let layout: Vec<Rect> = self.create_layout(area);

        for (index, widget) in self.widgets.iter().enumerate() {
            if widget.is_visible() {
                match index {
                    0 => widget.render(layout[0], buf),  // AWS Navigator
                    1 => widget.render(layout[1], buf),  // Main content
                    2 if self.show_popup => widget.render(popup_area, buf), // Popup
                    _ => {}
                }
            }
        }
    }

    fn calculate_popup_area(&self, base_area: Rect) -> Rect {
        Rect::new(
            base_area.x + POPUP_PADDING,
            base_area.y + POPUP_PADDING,
            base_area.width - 2 * POPUP_PADDING,
            base_area.height - 2 * POPUP_PADDING
        )
    }
}