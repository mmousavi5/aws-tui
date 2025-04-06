use crate::event_managment::event::{
    ComponentActions, Event, InputBoxEvent, TabEvent, WidgetActions, WidgetType,
};
use crate::services::aws::s3_client::S3Client;
use crate::services::aws::dynamo_client::DynamoDBClient;
use crate::widgets::WidgetExt;
use crate::widgets::aws_service_navigator::{AWSServiceNavigator, NavigatorContent};
use crate::widgets::input_box::InputBoxWidget;
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Direction, Layout, Rect},
    widgets::Widget,
};
use crate::event_managment::event::{AWSServiceNavigatorEvent, WidgetEventType, PopupEvent};
use std::sync::Arc;
use tokio::sync::Mutex;
use crate::widgets::popup::PopupWidget;


const TAB_HEIGHT: u16 = 3;
const POPUP_PADDING: u16 = 5;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ComponentFocus {
    Navigation,
    Input,
    Results,
    None,
}

pub struct DynamoDB {
    table_list_navigator: AWSServiceNavigator,
    query_input: InputBoxWidget,
    query_results_navigator: AWSServiceNavigator,
    active: bool,
    visible: bool,
    event_sender: tokio::sync::mpsc::UnboundedSender<Event>,
    current_focus: ComponentFocus,
    dynamodb_client: Option<Arc<Mutex<DynamoDBClient>>>,
    selected_table: Option<String>,
    selected_query: Option<String>,
    details_popup: PopupWidget,
}

impl DynamoDB {
    pub fn new(
        event_sender: tokio::sync::mpsc::UnboundedSender<Event>,
    ) -> Self {
        Self {
            table_list_navigator: AWSServiceNavigator::new(
                WidgetType::AWSServiceNavigator,
                false,
                event_sender.clone(),
                NavigatorContent::Records(vec![
                    "DynamoDB".to_string(),
                    "S3".to_string(),
                    "Lambda".to_string(),
                ]),
            ),
            query_input: InputBoxWidget::new("Query Input", false, event_sender.clone()),
            query_results_navigator: AWSServiceNavigator::new(
                WidgetType::QueryResultsNavigator,
                false,
                event_sender.clone(),
                NavigatorContent::Records(vec![
                    "q1".to_string(),
                    "q2".to_string(),
                    "q3".to_string(),
                ]),
            ),
            details_popup: PopupWidget::new("Details", false, false, event_sender.clone()),
            active: false,
            visible: true,
            event_sender,
            current_focus: ComponentFocus::Navigation,
            dynamodb_client: None,
            selected_table: None,
            selected_query: None,
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

    pub fn render(&self, area: Rect, buf: &mut Buffer) {
        if !self.visible {
            return;
        }
        let popup_area = self.calculate_popup_area(area);

        // Create a horizontal split for left and right panels
        let horizontal_split = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(30), // Left panel - table list
                Constraint::Percentage(70), // Right panel
            ])
            .split(area);

        // Create a vertical split for the right panel
        let right_vertical_split = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Percentage(15), // Query input box
                Constraint::Percentage(85), // Query results
            ])
            .split(horizontal_split[1]);

        // Render table list navigator (left panel)
        self.table_list_navigator.render(horizontal_split[0], buf);

        // Render query input box (top of right panel)
        self.query_input.render(right_vertical_split[0], buf);

        // Render query results navigator (bottom of right panel)
        self.query_results_navigator
            .render(right_vertical_split[1], buf);

        if self.details_popup.is_visible() {
            self.details_popup.render(popup_area, buf);
        }

    }

    pub fn handle_input(&mut self, key_event: crossterm::event::KeyEvent) {
        if self.details_popup.is_visible() {
            if let Some(signal) = self.details_popup.handle_input(key_event) {
                self.event_sender
                    .send(Event::Tab(TabEvent::ComponentActions(
                        ComponentActions::WidgetActions(signal),
                    )))
                    .unwrap();
            }
        }
        match key_event.code {
            KeyCode::Char('t') => {
                self.event_sender
                    .send(Event::Tab(TabEvent::ComponentActions(
                        ComponentActions::NextFocus,
                    )))
                    .unwrap();
            }
            _ => {
                // Handle other inputs based on current focus
                if let Some(signal) = match self.current_focus {
                    ComponentFocus::Navigation => self.table_list_navigator.handle_input(key_event),
                    ComponentFocus::Input => self.query_input.handle_input(key_event),
                    ComponentFocus::Results => self.query_results_navigator.handle_input(key_event),
                    ComponentFocus::None => None,
                } {
                    self.event_sender
                        .send(Event::Tab(TabEvent::ComponentActions(
                            ComponentActions::WidgetActions(signal),
                        )))
                        .unwrap();
                }
            }
        }
    }

    fn update_widget_states(&mut self) {
        self.table_list_navigator
            .set_active(self.active & (self.current_focus == ComponentFocus::Navigation));
        self.query_input
            .set_active(self.active & (self.current_focus == ComponentFocus::Input));
        self.query_results_navigator
            .set_active(self.active & (self.current_focus == ComponentFocus::Results));
    }

    fn is_visible(&self) -> bool {
        self.visible
    }

    pub fn set_active(&mut self, active: bool) {
        self.active = active;
    }

    fn set_inactive(&mut self) {
        self.active = false;
    }

    fn set_visible(&mut self, visible: bool) {
        self.visible = visible;
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }

    pub async fn process_event(&mut self, event: ComponentActions) {
        match event {
            ComponentActions::ArrowDown => {}
            ComponentActions::ArrowUp => {}
            ComponentActions::SetTitle(title) => {
                self.table_list_navigator.set_title(title.clone());
                self.selected_table = Some(title);
            }
            ComponentActions::SetQuery(query) => {
                self.query_results_navigator.set_title(query.clone());
                self.selected_query = Some(query.clone());
                let content = self.dynamodb_client
                    .as_ref()
                    .unwrap()
                    .lock()
                    .await
                    .query_table(self.selected_table.clone().unwrap(), query.clone())
                    .await
                    .unwrap();
                self.query_results_navigator
                    .set_content(NavigatorContent::Records(content));
            }
            ComponentActions::PopupDetails(title) => {
                self.details_popup.set_profile_list(vec![title.clone()]);
                self.details_popup.set_visible(true);
                self.details_popup.set_active(true);
            }
            ComponentActions::NextFocus => {
                self.focus_next();
                self.update_widget_states();
            }
            ComponentActions::WidgetActions(widget_action) => match widget_action {
                WidgetActions::AWSServiceNavigatorEvent(ref _aws_navigator_event, widget_type) => {
                    if widget_type == WidgetType::AWSServiceNavigator {
                        
                        if let Some(signal) = self.table_list_navigator
                            .process_event(widget_action.clone()) {
                                match signal {
                                    WidgetActions::AWSServiceNavigatorEvent(
                                        AWSServiceNavigatorEvent::SelectedItem(WidgetEventType::RecordSelected(
                                            title,
                                        )),
                                        WidgetType::AWSServiceNavigator,
                                    ) => {
                                        self.event_sender
                                            .send(Event::Tab(TabEvent::ComponentActions(
                                                ComponentActions::SetTitle(
                                                    title.clone(),
                                                ),
                                            )))
                                            .unwrap();
                                    }
                                    _ => {}
                                    
                                }
                            }


                    } else if widget_type == WidgetType::QueryResultsNavigator {
                        if let Some(signal) = self.query_results_navigator
                            .process_event(widget_action.clone()){
                                match signal {
                                    WidgetActions::AWSServiceNavigatorEvent(
                                        AWSServiceNavigatorEvent::SelectedItem(WidgetEventType::RecordSelected(
                                            title,
                                        )),
                                        WidgetType::QueryResultsNavigator,
                                    ) => {
                                        self.event_sender
                                            .send(Event::Tab(TabEvent::ComponentActions(
                                                ComponentActions::PopupDetails(
                                                    title.clone(),
                                                ),
                                            )))
                                            .unwrap();
                                    }
                                    _ => {}
                                }

                            }
                    }
                }
                WidgetActions::InputBoxEvent(InputBoxEvent::Written(content)) => {
                    let content_vec: Vec<String> =
                        content.lines().map(|line| line.to_string()).collect();
                    self.query_results_navigator
                        .set_content(NavigatorContent::Records(content_vec));
                }
                WidgetActions::PopupEvent(_) => {
                    self.details_popup.set_visible(false);
                    self.details_popup.set_active(false);
                }
                WidgetActions::InputBoxEvent(ref _input_box_event) => {
                    if let Some(signal) = self.query_input.process_event(widget_action){
                        match signal {
                            WidgetActions::InputBoxEvent(InputBoxEvent::Written(content)) => {
                                self.event_sender
                                    .send(Event::Tab(TabEvent::ComponentActions(
                                        ComponentActions::SetQuery(content),
                                    )))
                                    .unwrap();
                                }
                            _ => {}
                        }
                    }
                }
                _ => {}
            },
            // Add specific DynamoDB event handling here
            _ => {}
        }
    }

    pub fn get_current_focus(&self) -> ComponentFocus {
        self.current_focus
    }

    pub fn reset_focus(&mut self) {
        self.current_focus = ComponentFocus::Navigation;
    }

    pub fn set_client(&mut self, dynamodb_client: Arc<Mutex<DynamoDBClient>>) {
        self.dynamodb_client = Some(dynamodb_client);
    }

    pub fn focus_next(&mut self) -> ComponentFocus {
        self.current_focus = match self.current_focus {
            ComponentFocus::Navigation => ComponentFocus::Input,
            ComponentFocus::Input => ComponentFocus::Results,
            ComponentFocus::Results => ComponentFocus::None,
            ComponentFocus::None => ComponentFocus::Navigation,
        };
        self.current_focus
    }
    pub async fn update(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        // Update the DynamoDB tables list if client is available
        if let Some(client) = &self.dynamodb_client {
            let client = client.lock().await;
            let tables = client.list_tables().await?;
            self.table_list_navigator.set_content(NavigatorContent::Records(tables));
        }
        Ok(())
    }
}