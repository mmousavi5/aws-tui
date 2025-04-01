use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    buffer::Buffer,
    layout::{Alignment, Constraint, Direction, Layout, Rect, Margin},
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Tabs, Widget},
};
use crate::{
    event_managment::event::{Event, WidgetEventType},
    widgets::{
        WidgetExt,
        paragraph::ParagraphWidget,
        popup::PopupWidget,
        aws_service_navigator::{AWSServiceNavigator, WidgetType, NavigatorContent},
    },
};
use std::collections::HashMap;
use ratatui::widgets::Borders;

// Constants
const TAB_HEIGHT: u16 = 3;
const POPUP_PADDING: u16 = 5;

pub struct Tab {
    pub name: String,
    popup_mod: bool,
    popup_widget: Option<Box<dyn WidgetExt>>,
    right_widgets: Vec<Box<dyn WidgetExt>>,
    left_widgets: Box<dyn WidgetExt>,
    sub_widgets: HashMap<WidgetEventType, Box<dyn WidgetExt>>,
    active_right_widget_index: usize,
    event_sender: tokio::sync::mpsc::UnboundedSender<Event>,
    toggle_focus: bool, // True means right panel is focused, false means left panel is focused
}

impl Tab {
    pub fn new(
        name: &str, 
        content: &str, 
        event_sender: tokio::sync::mpsc::UnboundedSender<Event>
    ) -> Self {
        Self {
            name: name.to_string(),
            popup_mod: true,
            left_widgets:Box::new(AWSServiceNavigator::new(
                WidgetType::AWSServiceNavigator,
                false,
                event_sender.clone(),
                NavigatorContent::Services(WidgetEventType::VALUES.to_vec()),
                )),
            popup_widget: Some(Box::new(PopupWidget::new(content, true, event_sender.clone()))),
            right_widgets: vec![
                Box::new(ParagraphWidget::new(content, false)),
            ],
            sub_widgets: HashMap::new(),  // Initialize empty HashMap
            active_right_widget_index: 0,
            event_sender,
            toggle_focus: false, // Default to left panel focused
        }
    }

    // Public getters and setters
    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn set_name(&mut self, name: String) {
        self.name = name;
        self.popup_mod = false;
    }

    // Public methods
    pub fn handle_input(&mut self, event: KeyEvent) {
        if self.popup_mod {
            if let Some(popup) = self.popup_widget.as_mut() {
                popup.handle_input(event);
            }
        } else {
            match event.code {
                KeyCode::Char('t') => {
                    self.toggle_focus = !self.toggle_focus;
                }
                _ => {
                    if self.toggle_focus {
                        self.left_widgets.handle_input(event);
                    } else {
                        self.right_widgets[self.active_right_widget_index].handle_input(event);
                    }
                }
            }
        }
    }

    pub fn render(&self, area: Rect, buf: &mut Buffer, tab_titles: Vec<String>, active_tab: usize) {
        self.render_tab_bar(area, buf, tab_titles, active_tab);
        let content_area = self.get_content_area(area);
        
        self.render_widgets(content_area, buf);
    }

    // Private methods
    fn cycle_active_widget_forward(&mut self) {
        self.toggle_focus = !self.toggle_focus;
        if self.toggle_focus {
            self.left_widgets.set_active();
        } else {
            self.left_widgets.set_inactive();
        }
        // self.widgets[self.active_widget_index].set_inactive();
        // let mut next_index = (self.active_widget_index + 1) % self.widgets.len();

        // while !self.widgets[next_index].is_visible() {
        //     next_index = (next_index + 1) % self.widgets.len();
        //     if next_index == self.active_widget_index {
        //         // If we loop back to the starting index, break to avoid infinite loop
        //         break;
        //     }
        // }

        // self.active_widget_index = next_index;
        // self.widgets[self.active_widget_index].set_active();
    }

    fn handle_widget_input(&mut self, event: KeyEvent) {
        if self.toggle_focus {
            self.left_widgets.handle_input(event);
        } else {
            self.right_widgets[self.active_right_widget_index].handle_input(event);
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
    
        // Create blocks with borders for each layout section
        let left_block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Plain)
            .border_style(Style::default().fg(if self.toggle_focus { Color::Red } else { Color::DarkGray }));
    
        let right_block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Plain)
            .border_style(Style::default().fg(if !self.toggle_focus { Color::Red } else { Color::DarkGray }));
    
        // Calculate inner areas for the actual widgets
        let left_inner = layout[0].inner(Margin::new(1, 1));
        let right_inner = layout[1].inner(Margin::new(1, 1));
    
        // First render the base widgets
        left_block.render(layout[0], buf);
        right_block.render(layout[1], buf);
        self.left_widgets.render(left_inner, buf);
        
        for widget in self.right_widgets.iter() {
            widget.render(right_inner, buf);
        }
        
        for (_, widget) in self.sub_widgets.iter() {
            if widget.is_visible() {
                widget.render(right_inner, buf);
            }
        }
    
        // Render popup last so it appears on top
        if self.popup_mod {
            self.popup_widget.as_ref().map(|popup| {
                popup.render(popup_area, buf);
            });
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

    pub fn update_sub_widgets(&mut self, event: WidgetEventType) {
        for (_, widget) in self.sub_widgets.iter_mut() {
            widget.set_inactive();
            widget.set_visible(false);
        }
        
        if let Some(widget) = self.sub_widgets.get_mut(&event) {
            widget.set_active();
            widget.set_visible(true);
        } else {
            let new_widget = match event {
                WidgetEventType::S3 => Box::new(AWSServiceNavigator::new(
                    WidgetType::AWSService,
                    false,
                    self.event_sender.clone(),
                    NavigatorContent::Records(vec![
                        "s3://my-bucket/images/photo1.jpg".to_string(),
                        "s3://my-bucket/documents/report.pdf".to_string(),
                        "s3://my-bucket/data/backup.zip".to_string(),
                        "s3://my-bucket/config/settings.json".to_string(),
                        "s3://my-bucket/logs/app.log".to_string(),
                    ])
                )),
                WidgetEventType::DynamoDB => Box::new(AWSServiceNavigator::new(
                    WidgetType::AWSService,
                    false,
                    self.event_sender.clone(),
                    NavigatorContent::Records(vec![
                        "Users | Partition Key: user_id | Items: 1500".to_string(),
                        "Orders | Partition Key: order_id | Items: 2300".to_string(),
                        "Products | Partition Key: product_id | Items: 850".to_string(),
                        "Customers | Partition Key: customer_id | Items: 1200".to_string(),
                        "Inventory | Partition Key: sku | Items: 750".to_string(),
                    ])
                )),
                WidgetEventType::RecordSelected(_) => return, // Skip creating new widget for RecordSelected events
            };
            self.right_widgets[0].set_visible(false);
            self.sub_widgets.insert(event, new_widget);
        }
    }
}