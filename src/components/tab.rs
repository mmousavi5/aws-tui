use crate::services::aws::TabClients;
use crate::{
    components::dynamodb::DynamoDB,
    event_managment::event::{
        AWSServiceNavigatorEvent, ComponentActions, Event, PopupEvent, TabActions, TabEvent,
        WidgetActions, WidgetEventType, WidgetType,DynamoDBComponentActions,S3ComponentActions,CloudWatchComponentActions,
    },
    widgets::{
        WidgetExt,
        aws_service_navigator::{AWSServiceNavigator, NavigatorContent},
        popup::PopupWidget,
    },
    components::ComponentFocus,
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
use crate::components::s3::S3Component;
use crate::components::AWSComponent;
use crate::components::cloudwatch::CloudWatch;

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
    right_widgets: HashMap<WidgetType, Box<dyn AWSComponent>>,
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
        let mut right_widgets: HashMap<WidgetType, Box<dyn AWSComponent>> = HashMap::new();
        right_widgets.insert(
            WidgetType::DynamoDB, 
            Box::new(DynamoDB::new(event_sender.clone()))
        );
        right_widgets.insert(
            WidgetType::S3,
            Box::new(S3Component::new(event_sender.clone()))
        );
        right_widgets.insert(
            WidgetType::CloudWatch,
            Box::new(CloudWatch::new(event_sender.clone()))
        );

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
    pub fn set_active_service(&mut self, service_type: WidgetType) {
        self.active_right_widget = service_type;
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
                // Use Tab for focus switching (standard macOS behavior)
                KeyCode::Tab => {
                    self.event_sender
                        .send(Event::Tab(TabEvent::TabActions(TabActions::NextFocus)))
                        .unwrap();
                }
                KeyCode::BackTab => {
                    // Shift+Tab for reverse focus
                    self.event_sender
                        .send(Event::Tab(TabEvent::TabActions(TabActions::PreviousFocus)))
                        .unwrap();
                }
                _ => {
                    if self.current_focus == TabFocus::Left {
                        if let Some(signal) = self.left_widgets.handle_input(event) {
                            self.event_sender
                                .send(Event::Tab(TabEvent::WidgetActions(signal)))
                                .unwrap();
                        }
                    } else {
                        if let Some(widget) = self.right_widgets.get_mut(&self.active_right_widget)
                        {
                            widget.handle_input(event);
                        }
                    }
                }
            }
        }
    }
    pub async fn process_event(&mut self, tab_event: TabEvent) {
        match tab_event {
            TabEvent::TabActions(tab_action) => {
                self.process_tab_action(tab_action).await;
            }
            TabEvent::WidgetActions(widget_action) => match widget_action {
                WidgetActions::PopupEvent(ref _popup_event) => {
                    if let Some(popup) = self.popup_widget.as_mut() {
                        if self.popup_mod {
                            if let Some(signal) = popup.process_event(widget_action) {
                                match signal {
                                    WidgetActions::PopupEvent(PopupEvent::SelectedItem(
                                        selected,
                                    )) => {
                                        self.event_sender
                                            .send(Event::Tab(TabEvent::TabActions(
                                                TabActions::ProfileSelected(selected),
                                            )))
                                            .unwrap();
                                    }
                                    _ => {}
                                }
                            }
                        }
                    }
                }
                WidgetActions::AWSServiceNavigatorEvent(ref _aws_navigator_event, _) => {
                    if let Some(signal) = self.left_widgets.process_event(widget_action) {
                        match signal {
                            WidgetActions::AWSServiceNavigatorEvent(
                                AWSServiceNavigatorEvent::SelectedItem(selected),
                                _widget_type,
                            ) => {
                                self.event_sender
                                    .send(Event::Tab(TabEvent::TabActions(
                                        TabActions::AWSServiceSelected(selected),
                                    )))
                                    .unwrap();
                            }
                            _ => {}
                        }
                    }
                }
                _ => {}
            },
            TabEvent::ComponentActions(component_action) => {
                // Route component actions to the appropriate component based on type
                match component_action {
                    ComponentActions::S3ComponentActions(_) if self.active_right_widget == WidgetType::S3 => {
                        if let Some(widget) = self.right_widgets.get_mut(&WidgetType::S3) {
                            widget.process_event(component_action).await;
                        }
                    },
                    ComponentActions::DynamoDBComponentActions(_) if self.active_right_widget == WidgetType::DynamoDB => {
                        if let Some(widget) = self.right_widgets.get_mut(&WidgetType::DynamoDB) {
                            widget.process_event(component_action).await;
                        }
                    },
                    ComponentActions::CloudWatchComponentActions(_) if self.active_right_widget == WidgetType::CloudWatch => {
                        if let Some(widget) = self.right_widgets.get_mut(&WidgetType::CloudWatch) {
                            widget.process_event(component_action).await;
                        }
                    },
                    // Handle generic component actions that aren't specific to a component type
                    _ => {
                        if let Some(widget) = self.right_widgets.get_mut(&self.active_right_widget) {
                            widget.process_event(component_action).await;
                        }
                    }
                }
            }
        }
    }
    
    pub async fn process_tab_action(&mut self, tab_action: TabActions) {
        match tab_action {
            TabActions::ProfileSelected(profile) => {
                self.set_name(profile);
            }
            TabActions::AWSServiceSelected(service) => {
                match service {
                    WidgetEventType::DynamoDB => {
                        self.active_right_widget = WidgetType::DynamoDB;
                        if let Some(widget) = self.right_widgets.get_mut(&WidgetType::DynamoDB) {
                            if let Ok(client) = self.aws_clients.get_dynamodb_client().await {
                                let dynamo = widget.as_any_mut().downcast_mut::<DynamoDB>().unwrap();
                                dynamo.set_client(client);
                                widget.update().await.ok();
                            }
                        }
                    }
                    WidgetEventType::S3 => {
                        self.active_right_widget = WidgetType::S3;
                        if let Some(widget) = self.right_widgets.get_mut(&WidgetType::S3) {
                            if let Ok(client) = self.aws_clients.get_s3_client().await {
                                let s3 = widget.as_any_mut().downcast_mut::<S3Component>().unwrap();
                                s3.set_client(client);
                                widget.update().await.ok();
                            }
                        }
                    }
                    WidgetEventType::CloudWatch => {
                        self.active_right_widget = WidgetType::CloudWatch;
                        if let Some(widget) = self.right_widgets.get_mut(&WidgetType::CloudWatch) {
                            if let Ok(client) = self.aws_clients.get_cloudwatch_client().await {
                                let cloudwatch = widget.as_any_mut().downcast_mut::<CloudWatch>().unwrap();
                                cloudwatch.set_client(client);
                                widget.update().await.ok();
                            }
                        }
                    }
                    _ => {}
                }
            }
            TabActions::NextFocus => {
                if self.current_focus == TabFocus::Left {
                    self.current_focus = TabFocus::Right;
                    // Activate the right widget when switching to it
                    if let Some(widget) = self.right_widgets.get_mut(&self.active_right_widget) {
                        widget.set_active(true);
                    }
                } else {
                    if let Some(widget) = self.right_widgets.get_mut(&self.active_right_widget) {
                        if widget.get_current_focus() == ComponentFocus::None {
                            self.current_focus = TabFocus::Left;
                            widget.reset_focus();
                            widget.set_active(false);
                        } else {
                            widget.set_active(true);
                            match self.active_right_widget {
                                WidgetType::S3 => {
                                    self.event_sender
                                        .send(Event::Tab(TabEvent::ComponentActions(
                                            ComponentActions::S3ComponentActions(S3ComponentActions::NextFocus),
                                        )))
                                        .unwrap();
                                },
                                WidgetType::DynamoDB => {
                                    self.event_sender
                                        .send(Event::Tab(TabEvent::ComponentActions(
                                            ComponentActions::DynamoDBComponentActions(DynamoDBComponentActions::NextFocus),
                                        )))
                                        .unwrap();
                                },
                                WidgetType::CloudWatch => {
                                    self.event_sender
                                        .send(Event::Tab(TabEvent::ComponentActions(
                                            ComponentActions::CloudWatchComponentActions(CloudWatchComponentActions::NextFocus),
                                        )))
                                        .unwrap();
                                },
                                _ => {}
                            }
                        }
                    }
                }
            }
            TabActions::PreviousFocus => {
                if self.current_focus == TabFocus::Right {
                    if let Some(widget) = self.right_widgets.get_mut(&self.active_right_widget) {
                        if widget.get_current_focus() != ComponentFocus::Navigation {
                            // Send previous focus to component
                            match self.active_right_widget {
                                WidgetType::S3 => {
                                    self.event_sender
                                        .send(Event::Tab(TabEvent::ComponentActions(
                                            ComponentActions::S3ComponentActions(S3ComponentActions::PreviousFocus),
                                        )))
                                        .unwrap();
                                },
                                WidgetType::DynamoDB => {
                                    self.event_sender
                                        .send(Event::Tab(TabEvent::ComponentActions(
                                            ComponentActions::DynamoDBComponentActions(DynamoDBComponentActions::PreviousFocus),
                                        )))
                                        .unwrap();
                                },

                                WidgetType::CloudWatch => {
                                    self.event_sender
                                        .send(Event::Tab(TabEvent::ComponentActions(
                                            ComponentActions::CloudWatchComponentActions(CloudWatchComponentActions::PreviousFocus),
                                        )))
                                        .unwrap();
                                },
                                _ => {}
                            }
                        } else {
                            // Go back to left component
                            self.current_focus = TabFocus::Left;
                            widget.set_active(false);
                        }
                    }
                } else {
                    // If already at left, cycle to rightmost component's last focus
                    self.current_focus = TabFocus::Right;
                    if let Some(widget) = self.right_widgets.get_mut(&self.active_right_widget) {
                        widget.set_active(true);
                        widget.set_focus_to_last();
                    }
                }
            }
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
}
