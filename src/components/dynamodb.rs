use crate::components::aws_base_component::AWSComponentBase;
use crate::components::{AWSComponent, ComponentFocus};
use crate::event_managment::event::{
    ComponentAction, ComponentType, Event, InputBoxEvent, ServiceNavigatorEvent, TabEvent,
    WidgetAction, WidgetEventType, WidgetType,InputBoxType,
};
use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Direction, Layout, Rect},
};
use crate::services::aws::TabClients;
use crate::services::aws::dynamo_client::DynamoDBClient;
use crate::widgets::WidgetExt;
use crate::widgets::popup::PopupContent;
use crate::widgets::service_navigator::NavigatorContent;
use crate::widgets::input_box::InputBoxWidget;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use std::any::Any;
use std::sync::Arc;
use tokio::sync::Mutex;

/// Component for interacting with AWS DynamoDB
pub struct DynamoDB {
    /// Component type identifier
    component_type: ComponentType,
    /// Common AWS component functionality
    base: AWSComponentBase,
    /// Client for DynamoDB API interactions
    dynamodb_client: Option<Arc<Mutex<DynamoDBClient>>>,
    /// AWS service client
    aws_clients: Option<TabClients>,
    /// Input box for sort key
    sort_key_input: InputBoxWidget,
    /// Current focus within this component
    current_sub_focus: ComponentFocus,
}

impl DynamoDB {
    /// Creates a new DynamoDB component with the provided event sender
    pub fn new(event_sender: tokio::sync::mpsc::UnboundedSender<Event>) -> Self {
        Self {
            component_type: ComponentType::DynamoDB,
            base: AWSComponentBase::new(event_sender.clone(), NavigatorContent::Records(vec![])),
            dynamodb_client: None,
            aws_clients: None,
            sort_key_input: InputBoxWidget::new(
                InputBoxType::TimeRange,
                "Sort Key (if applicable)",
                false,
            ),
            current_sub_focus: ComponentFocus::Input,
        }
    }
    /// Updates focus for sort key input and other components
    fn update_sort_key_focus(&mut self, activate: bool) {
        self.sort_key_input.set_active(activate);
        self.base.input.set_active(!activate);
        self.base.navigator.set_active(!activate);
        self.base.results_navigator.set_active(!activate);

        if activate {
            self.current_sub_focus = ComponentFocus::TimeRange;
        }
    }
}

#[async_trait::async_trait]
impl AWSComponent for DynamoDB {
    fn render(&self, area: Rect, buf: &mut Buffer) {
        if !self.base.visible {
            return;
        }

        // Create a horizontal split for left panel (tables) and right panel (query and results)
        let horizontal_split = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(30), // Left panel - tables list
                Constraint::Percentage(70), // Right panel - queries and results
            ])
            .split(area);

        // Create a vertical split for the right panel to separate inputs from results
        let right_vertical_split = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3), // Input row (partition key + sort key)
                Constraint::Min(1),    // Query results
            ])
            .split(horizontal_split[1]);

        // Create a horizontal split for the input area to place partition key and sort key inputs side by side
        let input_row = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(50), // Partition key input (half width)
                Constraint::Percentage(50), // Sort key input (half width)
            ])
            .split(right_vertical_split[0]);

        // Render components
        self.base.navigator.render(horizontal_split[0], buf);

        // Render the partition key input box
        self.base.input.render(input_row[0], buf);

        // Render the sort key input box
        self.sort_key_input.render(input_row[1], buf);

        // Render the results navigator
        self.base.results_navigator.render(right_vertical_split[1], buf);

        // Render popup if visible
        if self.base.details_popup.is_visible() {
            self.base.details_popup.render(area, buf);
        }
    }

    /// Sets focus to the last active widget in the component
    fn set_focus_to_last(&mut self) {
        self.base.set_focus_to_last();
        
        // Special handling for sort key focus
        if self.base.current_focus == ComponentFocus::Input && 
           self.current_sub_focus == ComponentFocus::TimeRange {
            self.update_sort_key_focus(true);
        } else {
            self.update_sort_key_focus(false);
            self.base.update_widget_states();
        }
    }

    /// Handles keyboard input events
    fn handle_input(&mut self, key_event: KeyEvent) {
        // Special handling for popup details if visible
        if self.base.details_popup.is_visible() {
            if let Some(signal) = self.base.details_popup.handle_input(key_event) {
                self.base
                    .event_sender
                    .send(Event::Tab(TabEvent::ComponentActions(
                        ComponentAction::WidgetAction(signal),
                        self.component_type.clone(),
                    )))
                    .unwrap();
                return;
            }
        }

        match key_event.code {
            KeyCode::Tab => {
                self.base
                    .event_sender
                    .send(Event::Tab(TabEvent::ComponentActions(
                        ComponentAction::NextFocus,
                        self.component_type.clone(),
                    )))
                    .unwrap();
            }
            KeyCode::BackTab => {
                self.base
                    .event_sender
                    .send(Event::Tab(TabEvent::ComponentActions(
                        ComponentAction::PreviousFocus,
                        self.component_type.clone(),
                    )))
                    .unwrap();
            }
            // Alt+number shortcuts to switch focus between areas
            KeyCode::Char('1') if key_event.modifiers == KeyModifiers::ALT => {
                self.base.current_focus = ComponentFocus::Navigation;
                self.update_sort_key_focus(false);
                self.base.update_widget_states();
            }
            KeyCode::Char('2') if key_event.modifiers == KeyModifiers::ALT => {
                self.base.current_focus = ComponentFocus::Input;
                self.update_sort_key_focus(false);
                self.base.update_widget_states();
            }
            KeyCode::Char('3') if key_event.modifiers == KeyModifiers::ALT => {
                self.base.current_focus = ComponentFocus::Input;
                self.update_sort_key_focus(true);
            }
            KeyCode::Char('4') if key_event.modifiers == KeyModifiers::ALT => {
                self.base.current_focus = ComponentFocus::Results;
                self.update_sort_key_focus(false);
                self.base.update_widget_states();
            }
            KeyCode::Esc => {
                if self.base.current_focus != ComponentFocus::Navigation {
                    self.base.current_focus = ComponentFocus::Navigation;
                    self.update_sort_key_focus(false);
                    self.base.update_widget_states();
                }
            }
            _ => {
                // Forward input to the currently focused widget
                if let Some(signal) = match self.base.current_focus {
                    ComponentFocus::Navigation => self.base.navigator.handle_input(key_event),
                    ComponentFocus::Input => {
                        if self.current_sub_focus == ComponentFocus::TimeRange {
                            self.sort_key_input.handle_input(key_event)
                        } else {
                            self.base.input.handle_input(key_event)
                        }
                    },
                    ComponentFocus::Results => self.base.results_navigator.handle_input(key_event),
                    ComponentFocus::None => None,
                    _ => None,
                } {
                    self.base
                        .event_sender
                        .send(Event::Tab(TabEvent::ComponentActions(
                            ComponentAction::WidgetAction(signal),
                            self.component_type.clone(),
                        )))
                        .unwrap();
                }
            }
        }
    }
    /// Processes component-specific actions
    async fn process_event(&mut self, event: ComponentAction) {
        match event {
            ComponentAction::Active(aws_profile) => {
                self.aws_clients = Some(TabClients::new(aws_profile, String::from("eu-west-1")));

                // Unwrap the Result and handle errors properly
                if let Some(clients) = &mut self.aws_clients {
                    match clients.get_dynamodb_client().await {
                        Ok(client) => {
                            self.dynamodb_client = Some(client);
                            self.update().await.ok();
                        }
                        Err(err) => {
                            // Handle the error (show error in UI)
                            self.base
                                .results_navigator
                                .set_title(String::from("Error connecting to CloudWatch"));
                            self.base
                                .results_navigator
                                .set_content(NavigatorContent::Records(vec![format!(
                                    "Failed to initialize CloudWatch client: {}",
                                    err
                                )]));
                        }
                    }
                }
            }
            ComponentAction::Focused => {
                self.set_active(true);
            }
            ComponentAction::Unfocused => {
                self.reset_focus();
                // Set the component as inactive
                self.set_active(false);
            }

            // Handle selection of a table
            ComponentAction::SetTitle(title) => {
                self.base.navigator.set_title(title.clone());
                self.base.selected_item = Some(title);
                self.base.focus_next();
                self.base.update_widget_states();
            }
            // Show item details in a popup
            ComponentAction::PopupDetails(title) => {
                self.base
                    .details_popup
                    .set_content(PopupContent::Details(title.clone()));
                self.base.details_popup.set_visible(true);
                self.base.details_popup.set_active(true);
            }
            // Cycle focus through widgets
            ComponentAction::NextFocus => {
                // If we're on sort key focus, we need special handling
                if self.base.current_focus == ComponentFocus::TimeRange {
                    self.update_sort_key_focus(false);
                    self.base.current_focus = ComponentFocus::Results;
                    self.base.update_widget_states();
                } else {
                    let prev_focus = self.base.current_focus;
                    self.base.focus_next();

                    // If we just moved to TimeRange, activate time range input
                    if prev_focus != ComponentFocus::TimeRange
                        && self.base.current_focus == ComponentFocus::TimeRange
                    {
                        self.update_sort_key_focus(true);
                    } else {
                        self.base.update_widget_states();
                    }
                }
            }
            ComponentAction::PreviousFocus => {
                // If we're on sort key focus, go back to primary key
                if self.base.current_focus == ComponentFocus::TimeRange {
                    self.update_sort_key_focus(false);
                    self.base.current_focus = ComponentFocus::Input;
                    self.base.update_widget_states();
                } else {
                    let prev_focus = self.base.current_focus;
                    self.base.focus_previous();
                    
                    // If we just moved to TimeRange, activate time range input
                    if prev_focus != ComponentFocus::TimeRange
                        && self.base.current_focus == ComponentFocus::TimeRange
                    {
                        self.update_sort_key_focus(true);
                    } else {
                        self.base.update_widget_states();
                    }
                }
            }
            ComponentAction::SetQuery(partition_key) => {
                self.base.results_navigator.set_title(partition_key.clone());
                self.base.selected_query = Some(partition_key.clone());

                if let Some(client) = &self.dynamodb_client {
                    if let Some(selected_table) = &self.base.selected_item {
                        // Get the sort key value if available
                        let sort_key = self.sort_key_input.get_content();
                        
                        // Query the selected table with the partition key and sort key
                        let content = client
                            .lock()
                            .await
                            .query_table_composite(
                                selected_table.clone(), 
                                partition_key.clone(),
                                sort_key
                            )
                            .await
                            .unwrap_or_else(|_| vec!["Query error".to_string()]);

                        self.base
                            .results_navigator
                            .set_content(NavigatorContent::Records(content));
                    }
                }
                // Move focus to the results after query
                self.base.current_focus = ComponentFocus::Results;
                self.update_sort_key_focus(false);
                self.base.update_widget_states();
            }
            // Handle widget-specific actions
            ComponentAction::WidgetAction(widget_action) => match widget_action {
                // Process navigator events
                WidgetAction::ServiceNavigatorEvent(ref _aws_navigator_event, widget_type) => {
                    if widget_type == WidgetType::AWSServiceNavigator {
                        if let Some(signal) =
                            self.base.navigator.process_event(widget_action.clone())
                        {
                            match signal {
                                // Handle selection of a table from the navigator
                                WidgetAction::ServiceNavigatorEvent(
                                    ServiceNavigatorEvent::ItemSelected(
                                        WidgetEventType::RecordSelected(title),
                                    ),
                                    WidgetType::AWSServiceNavigator,
                                ) => {
                                    self.base
                                        .event_sender
                                        .send(Event::Tab(TabEvent::ComponentActions(
                                            ComponentAction::SetTitle(title.clone()),
                                            self.component_type.clone(),
                                        )))
                                        .unwrap();
                                }
                                _ => {}
                            }
                        }
                    } else if widget_type == WidgetType::QueryResultsNavigator {
                        // Process events from the query results navigator
                        if let Some(signal) = self
                            .base
                            .results_navigator
                            .process_event(widget_action.clone())
                        {
                            match signal {
                                // Handle selection of a result item to show details
                                WidgetAction::ServiceNavigatorEvent(
                                    ServiceNavigatorEvent::ItemSelected(
                                        WidgetEventType::RecordSelected(title),
                                    ),
                                    WidgetType::QueryResultsNavigator,
                                ) => {
                                    self.base
                                        .event_sender
                                        .send(Event::Tab(TabEvent::ComponentActions(
                                            ComponentAction::PopupDetails(title.clone()),
                                            self.component_type.clone(),
                                        )))
                                        .unwrap();
                                }
                                _ => {}
                            }
                        }
                    }
                }
                // Process input box events
                WidgetAction::InputBoxEvent(ref _input_box_event, ref input_type) => {
                    match input_type {
                        InputBoxType::Text => {
                            if let Some(signal) = self.base.input.process_event(widget_action.clone()) {
                                match signal {
                                    WidgetAction::InputBoxEvent(InputBoxEvent::Written(content), _) => {
                                        // If the Enter key was pressed in the partition key input
                                        self.base
                                            .event_sender
                                            .send(Event::Tab(TabEvent::ComponentActions(
                                                ComponentAction::SetQuery(content),
                                                self.component_type.clone(),
                                            )))
                                            .unwrap();
                                    }
                                    _ => {}
                                }
                            }
                        }
                        InputBoxType::TimeRange => {
                            if let Some(signal) = self.sort_key_input.process_event(widget_action.clone()) {
                                match signal {
                                    WidgetAction::InputBoxEvent(InputBoxEvent::Written(_), _) => {
                                        // When Enter is pressed in the sort key input, 
                                        // execute the query using the partition key
                                        if let Some(partition_key) = self.base.input.get_content() {
                                            self.base
                                                .event_sender
                                                .send(Event::Tab(TabEvent::ComponentActions(
                                                    ComponentAction::SetQuery(partition_key),
                                                    self.component_type.clone(),
                                                )))
                                                .unwrap();
                                        }
                                    }
                                    _ => {}
                                }
                            }
                        }
                        _ => {}
                    }
                }
                // Handle popup close events
                WidgetAction::PopupAction(_) => {
                    self.base.details_popup.set_visible(false);
                    self.base.details_popup.set_active(false);
                }
                _ => {}
            },
            _ => {}
        }
    }

    /// Sets the active state of this component
    fn set_active(&mut self, active: bool) {
        self.base.active = active;
        self.base.update_widget_states();
    }

    fn is_active(&self) -> bool {
        self.base.active
    }

    fn set_visible(&mut self, visible: bool) {
        self.base.visible = visible;
    }

    fn is_visible(&self) -> bool {
        self.base.visible
    }

    /// Refreshes the list of DynamoDB tables from AWS
    async fn update(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(client) = &self.dynamodb_client {
            let client = client.lock().await;
            let tables = client.list_tables().await?;
            self.base
                .navigator
                .set_content(NavigatorContent::Records(tables));
        }
        Ok(())
    }

    fn get_current_focus(&self) -> ComponentFocus {
        self.base.current_focus
    }

    /// Resets focus to the navigation pane
    fn reset_focus(&mut self) {
        self.base.current_focus = ComponentFocus::Navigation;
        self.current_sub_focus = ComponentFocus::Input;
        self.update_sort_key_focus(false);
        self.base.update_widget_states();
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }

    fn get_help_items(&self) -> Vec<(String, String)> {
        let mut help_items = self.base.get_help_items();
        
        // Add sort key specific help
        if self.base.current_focus == ComponentFocus::Input {
            if self.current_sub_focus == ComponentFocus::TimeRange {
                help_items.push(("Alt+2".to_string(), "Partition Key".to_string()));
            } else {
                help_items.push(("Alt+3".to_string(), "Sort Key".to_string()));
            }
        }
        
        help_items
    }
}
