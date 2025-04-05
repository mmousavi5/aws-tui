use crate::services::aws::{TabClients, TabClientsError};
use crate::{
    components::dynamodb::{ComponentFocus, DynamoDB},
    event_managment::event::{
        ComponentActions, Event, TabActions, TabEvent, WidgetActions, WidgetEventType, WidgetType,
    },
    widgets::{
        WidgetExt,
        aws_service_navigator::{AWSServiceNavigator, NavigatorContent},
        input_box::InputBoxWidget,
        popup::PopupWidget,
    },
};
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::widgets::Borders;
use ratatui::{
    buffer::Buffer,
    layout::{Alignment, Constraint, Direction, Layout, Margin, Rect},
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Tabs, Widget},
};
use std::collections::HashMap;

// Constants
const TAB_HEIGHT: u16 = 3;
const POPUP_PADDING: u16 = 5;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TabFocus {
    Left,
    Right,
}

pub struct Tab {
    pub name: String,
    popup_mod: bool,
    popup_widget: Option<Box<dyn WidgetExt>>,
    right_widgets: HashMap<WidgetType, DynamoDB>,
    left_widgets: Box<dyn WidgetExt>,
    active_right_widget: WidgetType,
    event_sender: tokio::sync::mpsc::UnboundedSender<Event>,
    current_focus: TabFocus,
    aws_clients: TabClients,
}

impl Tab {
    pub fn new(
        name: &str,
        content: &str,
        event_sender: tokio::sync::mpsc::UnboundedSender<Event>,
    ) -> Self {
        let mut right_widgets: HashMap<WidgetType, DynamoDB> = HashMap::new();
        right_widgets.insert(WidgetType::DynamoDB, DynamoDB::new(event_sender.clone()));

        Self {
            name: name.to_string(),
            popup_mod: true,
            left_widgets: Box::new(AWSServiceNavigator::new(
                WidgetType::AWSServiceNavigator,
                false,
                event_sender.clone(),
                NavigatorContent::Services(WidgetEventType::VALUES.to_vec()),
            )),
            popup_widget: Some(Box::new(PopupWidget::new(
                content,
                true,
                event_sender.clone(),
            ))),
            right_widgets,
            active_right_widget: WidgetType::DynamoDB,
            event_sender,
            current_focus: TabFocus::Left, // Default to left widget
            aws_clients: TabClients::new(String::new(), String::from("eu-west-1")),
        }
    }

    pub fn handle_input(&mut self, event: KeyEvent) {
        if self.popup_mod {
            if let Some(popup) = self.popup_widget.as_mut() {
                if let Some(signal) = popup.handle_input(event) {
                    self.event_sender
                        .send(Event::Tab(TabEvent::WidgetActions(signal)))
                        .unwrap();
                }
            }
        } else {
            match event.code {
                KeyCode::Char('t') => {
                    self.event_sender
                        .send(Event::Tab(TabEvent::TabActions(TabActions::NextFocus)))
                        .unwrap();
                }
                _ => {
                    if self.current_focus == TabFocus::Left {
                        if let Some(signal) = self.left_widgets.handle_input(event) {
                            self.event_sender
                                .send(Event::Tab(TabEvent::WidgetActions(signal)))
                                .unwrap();
                        }
                        self.left_widgets.handle_input(event);
                    } else {
                        if let Some(widget) = self.right_widgets.get_mut(&self.active_right_widget)
                        {
                            widget.handle_input(event);
                        }
                    }
                } // match event.code {
                  //     KeyCode::Char('t') => {
                  //         if self.current_focus {
                  //             self.left_widgets.set_active(true);
                  //             self.current_focus = !self.current_focus;
                  //         } else {
                  //             self.left_widgets.set_active(false);
                  //             if let Some(widget) = self.right_widgets.get_mut(&self.active_right_widget) {
                  //                 widget.set_active(true);
                  //                 match widget.get_current_focus() {
                  //                     ComponentFocus::None => {
                  //                         self.current_focus = !self.current_focus;
                  //                         widget.reset_focus();
                  //                     }
                  //                     _ => {
                  //                         widget.handle_input(event);
                  //                     }
                  //                 }
                  //             }
                  //             self.current_focus = true;
                  //         }
                  //     }
                  //     _ => {
                  //         if self.current_focus {
                  //         } else if let Some(widget) = self.right_widgets.get_mut(&self.active_right_widget) {
                  //             widget.handle_input(event);
                  //         }
                  //     }
                  // }
            }
        }
    }
    pub async fn process_event(&mut self, tab_event: TabEvent) {
        match tab_event {
            TabEvent::TabActions(tab_acion) => {
                self.process_tab_action(tab_acion).await;
            }
            TabEvent::WidgetActions(widget_action) => match widget_action {
                WidgetActions::PopupEvent(ref _popup_event) => {
                    if let Some(popup) = self.popup_widget.as_mut() {
                        if self.popup_mod {
                            popup.process_event(widget_action);
                        }
                    }
                }
                WidgetActions::AWSServiceNavigatorEvent(ref _aws_navigator_event, _) => {
                    self.left_widgets.process_event(widget_action);
                }
                _ => {}
            },
            TabEvent::ComponentActions(component_action) => {
                if let Some(widget) = self.right_widgets.get_mut(&self.active_right_widget) {
                    widget.process_event(component_action);
                }
            }
            _ => {}
        }
    }

    pub async fn process_tab_action(&mut self, tab_action: TabActions) {
        match tab_action {
            TabActions::ProfileSelected(profile) => {
                self.set_name(profile);
            }
            TabActions::AWSServiceSelected(service) => {
                if let Some(widget) = self.right_widgets.get_mut(&self.active_right_widget) {
                    widget.set_s3_client(self.aws_clients.get_s3_client().await.unwrap());
                    widget.update().await;    
                }
            }
            TabActions::NextFocus => {
                if self.current_focus == TabFocus::Left {
                    self.current_focus = TabFocus::Right;
                } else {
                    if let Some(widget) = self.right_widgets.get_mut(&self.active_right_widget) {
                        widget.get_current_focus();
                        if widget.get_current_focus() == ComponentFocus::None {
                            self.current_focus = TabFocus::Left;
                            widget.reset_focus();
                        } else {
                            widget.set_active(true);
                            self.current_focus = TabFocus::Right;
                            self.event_sender
                                .send(Event::Tab(TabEvent::ComponentActions(
                                    ComponentActions::NextFocus,
                                )))
                                .unwrap();
                        }
                    }
                }
            }
            _ => {}
        }
    }

    // Public getters and setters
    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn set_name(&mut self, name: String) {
        self.name = name;
        self.popup_mod = false;
        self.aws_clients.set_profile(self.name.clone());
    }

    pub fn render(&self, area: Rect, buf: &mut Buffer, tab_titles: Vec<String>, active_tab: usize) {
        self.render_tab_bar(area, buf, tab_titles, active_tab);
        let content_area = self.get_content_area(area);

        self.render_widgets(content_area, buf);
    }

    fn render_tab_bar(
        &self,
        area: Rect,
        buf: &mut Buffer,
        tab_titles: Vec<String>,
        active_tab: usize,
    ) {
        let tab_block = Block::bordered()
            .title_alignment(Alignment::Center)
            .border_type(BorderType::Rounded);

        let tab_titles: Vec<Line> = tab_titles
            .iter()
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
            area.height - TAB_HEIGHT,
        )
    }

    fn create_layout(&self, area: Rect) -> Vec<Rect> {
        Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(20), Constraint::Percentage(80)])
            .split(area)
            .to_vec()
    }

    // Update the render_widgets method
    fn render_widgets(&self, area: Rect, buf: &mut Buffer) {
        let popup_area = self.calculate_popup_area(area);
        let layout: Vec<Rect> = self.create_layout(area);

        let left_block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Plain)
            .border_style(
                Style::default().fg(if self.current_focus == TabFocus::Left {
                    Color::Red
                } else {
                    Color::DarkGray
                }),
            );

        let right_block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Plain)
            .border_style(
                Style::default().fg(if self.current_focus == TabFocus::Right {
                    Color::Red
                } else {
                    Color::DarkGray
                }),
            );

        let left_inner = layout[0].inner(Margin::new(1, 1));
        let right_inner = layout[1].inner(Margin::new(1, 1));

        left_block.render(layout[0], buf);
        right_block.render(layout[1], buf);
        self.left_widgets.render(left_inner, buf);

        if let Some(widget) = self.right_widgets.get(&self.active_right_widget) {
            widget.render(right_inner, buf);
        }

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
            base_area.height - 2 * POPUP_PADDING,
        )
    }

    // pub async fn update_sub_widgets(&mut self, event: WidgetEventType) -> Result<(), TabClientsError> {
    //     match event {
    //         WidgetEventType::S3 => {
    //             let buckets = match self.aws_clients.list_s3_buckets().await {
    //                 Ok(buckets) if !buckets.is_empty() => buckets,
    //                 Ok(_) => vec!["No buckets found".to_string()],
    //                 Err(e) => vec![format!("Error listing buckets: {}", e)],
    //             };

    //             if let Some(existing_widget) = self.right_widgets.get_mut(&WidgetType::S3) {
    //                 if let Some(navigator) = existing_widget.as_any_mut().downcast_mut::<AWSServiceNavigator>() {
    //                     navigator.set_content(NavigatorContent::Records(buckets));
    //                 }
    //             } else {
    //                 self.right_widgets.insert(
    //                     WidgetType::S3,
    //                     Box::new(AWSServiceNavigator::new(
    //                         WidgetType::AWSService,
    //                         true,
    //                         self.event_sender.clone(),
    //                         NavigatorContent::Records(buckets)
    //                     ))
    //                 );
    //             }
    //             self.active_right_widget = WidgetType::S3;
    //         },
    //         WidgetEventType::DynamoDB => {
    //             let tables = match self.aws_clients.list_dynamodb_tables().await {
    //                 Ok(tables) if !tables.is_empty() => tables,
    //                 Ok(_) => vec!["No tables found".to_string()],
    //                 Err(e) => vec![format!("Error listing tables: {}", e)],
    //             };

    //             if let Some(dynamodb) = self.right_widgets.get_mut(&WidgetType::DynamoDB) {
    //                 if let Some(dynamodb_widget) = dynamodb.as_any_mut().downcast_mut::<DynamoDB>() {
    //                     // Update the tables in DynamoDB component
    //                     // You'll need to implement set_tables method in DynamoDB
    //                     dynamodb_widget.set_tables(tables);
    //                 }
    //             } else {
    //                 self.right_widgets.insert(
    //                     WidgetType::DynamoDB,
    //                     Box::new(DynamoDB::new(self.event_sender.clone()))
    //                 );
    //             }
    //             self.active_right_widget = WidgetType::DynamoDB;
    //         },
    //         WidgetEventType::RecordSelected(_) => return Ok(()),
    //     }
    //     self.right_widgets.remove(&WidgetType::Default);
    //     Ok(())
    // }
}
