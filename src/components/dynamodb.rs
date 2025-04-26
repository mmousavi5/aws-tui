use crate::components::{AWSComponent, ComponentFocus};
use crate::event_managment::event::{
    ComponentAction, ComponentType, Event, InputBoxEvent, ServiceNavigatorEvent, TabEvent,
    WidgetAction, WidgetEventType, WidgetType, InputBoxType,
};
use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Direction, Layout, Rect},
};
use crate::services::aws::TabClients;
use crate::services::aws::dynamo_client::DynamoDBClient;
use crate::widgets::WidgetExt;
use crate::widgets::popup::{PopupContent, PopupWidget};
use crate::widgets::service_navigator::{NavigatorContent, ServiceNavigator};
use crate::widgets::input_box::InputBoxWidget;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use std::any::Any;
use std::sync::Arc;
use tokio::sync::Mutex;

/// Component for interacting with AWS DynamoDB
pub struct DynamoDB {
    /// Component type identifier
    component_type: ComponentType,
    /// Client for DynamoDB API interactions
    dynamodb_client: Option<Arc<Mutex<DynamoDBClient>>>,
    /// AWS service client
    aws_clients: Option<TabClients>,
    /// Input box for sort key
    sort_key_input: InputBoxWidget,
    /// Current focus within this component
    current_sub_focus: ComponentFocus,
    
    /// Left navigator widget for service/bucket/table lists
    navigator: ServiceNavigator,
    /// Input widget for search/filter/query commands
    input: InputBoxWidget,
    /// Results area displaying query results or service content
    results_navigator: ServiceNavigator,
    /// Popup for displaying details and additional information
    details_popup: PopupWidget,
    /// Whether the component is currently active
    active: bool,
    /// Whether the component is currently visible
    visible: bool,
    /// Channel for sending events to the application
    event_sender: tokio::sync::mpsc::UnboundedSender<Event>,
    /// Current focus state within the component
    current_focus: ComponentFocus,
    /// Currently selected item (bucket, table, log group, etc.)
    selected_item: Option<String>,
    /// Current query string being executed
    selected_query: Option<String>,
}

impl DynamoDB {
    /// Creates a new DynamoDB component with the provided event sender
    pub fn new(event_sender: tokio::sync::mpsc::UnboundedSender<Event>) -> Self {
        let popup_content = PopupContent::Profiles(vec!["No content".to_string()]);
        
        Self {
            component_type: ComponentType::DynamoDB,
            dynamodb_client: None,
            aws_clients: None,
            sort_key_input: InputBoxWidget::new(
                InputBoxType::TimeRange,
                "Sort Key (if applicable)",
                false,
            ),
            current_sub_focus: ComponentFocus::Input,
            
            // Fields moved from AWSComponentBase
            navigator: ServiceNavigator::new(
                WidgetType::AWSServiceNavigator,
                false,
                NavigatorContent::Records(vec![]),
            ),
            input: InputBoxWidget::new(InputBoxType::Text, "Query Input", false),
            results_navigator: ServiceNavigator::new(
                WidgetType::QueryResultsNavigator,
                false,
                NavigatorContent::Records(vec![]),
            ),
            details_popup: PopupWidget::new(popup_content, "Details", false, false),
            active: false,
            visible: true,
            event_sender,
            current_focus: ComponentFocus::Navigation,
            selected_item: None,
            selected_query: None,
        }
    }
    
    /// Updates active states of all widgets based on current focus
    fn update_widget_states(&mut self) {
        self.navigator
            .set_active(self.active & (self.current_focus == ComponentFocus::Navigation));
        self.input
            .set_active(self.active & (self.current_focus == ComponentFocus::Input));
        self.results_navigator
            .set_active(self.active & (self.current_focus == ComponentFocus::Results));
    }

    /// Shifts focus to the previous widget in the cyclic order
    fn focus_previous(&mut self) -> ComponentFocus {
        self.current_focus = match self.current_focus {
            ComponentFocus::Navigation => ComponentFocus::None,
            ComponentFocus::Input => ComponentFocus::Navigation,
            ComponentFocus::TimeRange => ComponentFocus::Input,
            ComponentFocus::Results => ComponentFocus::TimeRange,
            ComponentFocus::None => ComponentFocus::Results,
        };
        self.current_focus
    }

    /// Shifts focus to the next widget in the cyclic order
    fn focus_next(&mut self) -> ComponentFocus {
        self.current_focus = match self.current_focus {
            ComponentFocus::Navigation => ComponentFocus::Input,
            ComponentFocus::Input => ComponentFocus::TimeRange,
            ComponentFocus::TimeRange => ComponentFocus::Results,
            ComponentFocus::Results => ComponentFocus::None,
            ComponentFocus::None => ComponentFocus::Navigation,
        };
        self.current_focus
    }
    
    /// Updates focus for sort key input and other components
    fn update_sort_key_focus(&mut self, activate: bool) {
        self.sort_key_input.set_active(activate);
        self.input.set_active(!activate);
        self.navigator.set_active(!activate);
        self.results_navigator.set_active(!activate);

        if activate {
            self.current_sub_focus = ComponentFocus::TimeRange;
        }
    }
    
    /// Returns contextual help items based on current component state
    fn get_base_help_items(&self) -> Vec<(String, String)> {
        let mut items = vec![];

        // Check if the popup is visible
        if self.details_popup.is_visible() {
            items.push(("Esc".to_string(), "Close details".to_string()));
            items.push(("PgUp/PgDn".to_string(), "Scroll content".to_string()));
            return items;
        }

        // Different help items based on current focus
        match self.current_focus {
            ComponentFocus::Navigation => {
                items.push(("Enter".to_string(), "Select table".to_string()));
                items.push(("Alt+2".to_string(), "Focus query input".to_string()));
                items.push(("Alt+4".to_string(), "Focus results".to_string()));
            }
            ComponentFocus::Results => {
                items.push(("Enter".to_string(), "View item details".to_string()));
                items.push(("Alt+1".to_string(), "Focus tables".to_string()));
                items.push(("Alt+2".to_string(), "Focus query input".to_string()));
            }
            ComponentFocus::Input => {
                items.push(("Enter".to_string(), "Execute query".to_string()));
                items.push(("Alt+1".to_string(), "Focus tables".to_string()));
                items.push(("Alt+4".to_string(), "Focus results".to_string()));
            }
            _ => {}
        }
        items
    }
}

#[async_trait::async_trait]
impl AWSComponent for DynamoDB {
    fn render(&self, area: Rect, buf: &mut Buffer) {
        if !self.visible {
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
        self.navigator.render(horizontal_split[0], buf);

        // Render the partition key input box
        self.input.render(input_row[0], buf);

        // Render the sort key input box
        self.sort_key_input.render(input_row[1], buf);

        // Render the results navigator
        self.results_navigator.render(right_vertical_split[1], buf);

        // Render popup if visible
        if self.details_popup.is_visible() {
            self.details_popup.render(area, buf);
        }
    }

    /// Sets focus to the last active widget in the component
    fn set_focus_to_last(&mut self) {
        self.current_focus = ComponentFocus::Results;
        
        // Special handling for sort key focus
        if self.current_focus == ComponentFocus::TimeRange {
            self.update_sort_key_focus(true);
        } else {
            self.update_sort_key_focus(false);
            self.update_widget_states();
        }
    }

    /// Handles keyboard input events
    fn handle_input(&mut self, key_event: KeyEvent) {
        // Special handling for popup details if visible
        if self.details_popup.is_visible() {
            if let Some(signal) = self.details_popup.handle_input(key_event) {
                self.event_sender
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
                self.event_sender
                    .send(Event::Tab(TabEvent::ComponentActions(
                        ComponentAction::NextFocus,
                        self.component_type.clone(),
                    )))
                    .unwrap();
            }
            KeyCode::BackTab => {
                self.event_sender
                    .send(Event::Tab(TabEvent::ComponentActions(
                        ComponentAction::PreviousFocus,
                        self.component_type.clone(),
                    )))
                    .unwrap();
            }
            // Alt+number shortcuts to switch focus between areas
            KeyCode::Char('1') if key_event.modifiers == KeyModifiers::ALT => {
                self.current_focus = ComponentFocus::Navigation;
                self.update_sort_key_focus(false);
                self.update_widget_states();
            }
            KeyCode::Char('2') if key_event.modifiers == KeyModifiers::ALT => {
                self.current_focus = ComponentFocus::Input;
                self.update_sort_key_focus(false);
                self.update_widget_states();
            }
            KeyCode::Char('3') if key_event.modifiers == KeyModifiers::ALT => {
                self.current_focus = ComponentFocus::Input;
                self.update_sort_key_focus(true);
            }
            KeyCode::Char('4') if key_event.modifiers == KeyModifiers::ALT => {
                self.current_focus = ComponentFocus::Results;
                self.update_sort_key_focus(false);
                self.update_widget_states();
            }
            KeyCode::Esc => {
                if self.current_focus != ComponentFocus::Navigation {
                    self.current_focus = ComponentFocus::Navigation;
                    self.update_sort_key_focus(false);
                    self.update_widget_states();
                }
            }
            _ => {
                // Forward input to the currently focused widget
                if let Some(signal) = match self.current_focus {
                    ComponentFocus::Navigation => self.navigator.handle_input(key_event),
                    ComponentFocus::Input => {
                        if self.current_sub_focus == ComponentFocus::TimeRange {
                            self.sort_key_input.handle_input(key_event)
                        } else {
                            self.input.handle_input(key_event)
                        }
                    },
                    ComponentFocus::Results => self.results_navigator.handle_input(key_event),
                    ComponentFocus::None => None,
                    _ => None,
                } {
                    self.event_sender
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
                            self.results_navigator
                                .set_title(String::from("Error connecting to DynamoDB"));
                            self.results_navigator
                                .set_content(NavigatorContent::Records(vec![format!(
                                    "Failed to initialize DynamoDB client: {}",
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
                self.navigator.set_title(title.clone());
                self.selected_item = Some(title);
                self.focus_next();
                self.update_widget_states();
            }
            // Show item details in a popup
            ComponentAction::PopupDetails(title) => {
                self.details_popup
                    .set_content(PopupContent::Details(title.clone()));
                self.details_popup.set_visible(true);
                self.details_popup.set_active(true);
            }
            // Cycle focus through widgets
            ComponentAction::NextFocus => {
                // If we're on sort key focus, we need special handling
                if self.current_focus == ComponentFocus::TimeRange {
                    self.update_sort_key_focus(false);
                    self.current_focus = ComponentFocus::Results;
                    self.update_widget_states();
                } else {
                    let prev_focus = self.current_focus;
                    self.focus_next();

                    // If we just moved to TimeRange, activate time range input
                    if prev_focus != ComponentFocus::TimeRange
                        && self.current_focus == ComponentFocus::TimeRange
                    {
                        self.update_sort_key_focus(true);
                    } else {
                        self.update_widget_states();
                    }
                }
            }
            ComponentAction::PreviousFocus => {
                // If we're on sort key focus, go back to primary key
                if self.current_focus == ComponentFocus::TimeRange {
                    self.update_sort_key_focus(false);
                    self.current_focus = ComponentFocus::Input;
                    self.update_widget_states();
                } else {
                    let prev_focus = self.current_focus;
                    self.focus_previous();
                    
                    // If we just moved to TimeRange, activate time range input
                    if prev_focus != ComponentFocus::TimeRange
                        && self.current_focus == ComponentFocus::TimeRange
                    {
                        self.update_sort_key_focus(true);
                    } else {
                        self.update_widget_states();
                    }
                }
            }
            ComponentAction::SetQuery(partition_key) => {
                self.results_navigator.set_title(partition_key.clone());
                self.selected_query = Some(partition_key.clone());

                if let Some(client) = &self.dynamodb_client {
                    if let Some(selected_table) = &self.selected_item {
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

                        self.results_navigator
                            .set_content(NavigatorContent::Records(content));
                    }
                }
                // Move focus to the results after query
                self.current_focus = ComponentFocus::Results;
                self.update_sort_key_focus(false);
                self.update_widget_states();
            }
            // Handle widget-specific actions
            ComponentAction::WidgetAction(widget_action) => match widget_action {
                // Process navigator events
                WidgetAction::ServiceNavigatorEvent(ref _aws_navigator_event, widget_type) => {
                    if widget_type == WidgetType::AWSServiceNavigator {
                        if let Some(signal) =
                            self.navigator.process_event(widget_action.clone())
                        {
                            match signal {
                                // Handle selection of a table from the navigator
                                WidgetAction::ServiceNavigatorEvent(
                                    ServiceNavigatorEvent::ItemSelected(
                                        WidgetEventType::RecordSelected(title),
                                    ),
                                    WidgetType::AWSServiceNavigator,
                                ) => {
                                    self.event_sender
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
                                    self.event_sender
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
                            if let Some(signal) = self.input.process_event(widget_action.clone()) {
                                match signal {
                                    WidgetAction::InputBoxEvent(InputBoxEvent::Written(content), _) => {
                                        // If the Enter key was pressed in the partition key input
                                        self.event_sender
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
                                        if let Some(partition_key) = self.input.get_content() {
                                            self.event_sender
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
                    self.details_popup.set_visible(false);
                    self.details_popup.set_active(false);
                }
                _ => {}
            },
            _ => {}
        }
    }

    /// Sets the active state of this component
    fn set_active(&mut self, active: bool) {
        self.active = active;
        self.update_widget_states();
    }

    fn is_active(&self) -> bool {
        self.active
    }

    fn set_visible(&mut self, visible: bool) {
        self.visible = visible;
    }

    fn is_visible(&self) -> bool {
        self.visible
    }

    /// Refreshes the list of DynamoDB tables from AWS
    async fn update(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(client) = &self.dynamodb_client {
            let client = client.lock().await;
            let tables = client.list_tables().await?;
            self.navigator
                .set_content(NavigatorContent::Records(tables));
        }
        Ok(())
    }

    fn get_current_focus(&self) -> ComponentFocus {
        self.current_focus
    }

    /// Resets focus to the navigation pane
    fn reset_focus(&mut self) {
        self.current_focus = ComponentFocus::Navigation;
        self.current_sub_focus = ComponentFocus::Input;
        self.update_sort_key_focus(false);
        self.update_widget_states();
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }

    fn get_help_items(&self) -> Vec<(String, String)> {
        let mut help_items = self.get_base_help_items();
        
        // Add sort key specific help
        if self.current_focus == ComponentFocus::Input {
            if self.current_sub_focus == ComponentFocus::TimeRange {
                help_items.push(("Alt+2".to_string(), "Partition Key".to_string()));
            } else {
                help_items.push(("Alt+3".to_string(), "Sort Key".to_string()));
            }
        }
        
        help_items
    }
}
