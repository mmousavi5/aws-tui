use crate::components::{AWSComponent, ComponentFocus};
use crate::event_managment::event::{
    ComponentAction, ComponentType, Event, InputBoxEvent, InputBoxType, ServiceNavigatorEvent,
    TabEvent, WidgetAction, WidgetEventType, WidgetType,
};
use crate::services::aws::TabClients;
use crate::services::aws::cloudwatch_client::CloudWatchClient;
use crate::widgets::WidgetExt;
use crate::widgets::input_box::InputBoxWidget;
use crate::widgets::popup::{PopupContent, PopupWidget};
use crate::widgets::service_navigator::{NavigatorContent, ServiceNavigator};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Direction, Layout, Rect},
};
use std::any::Any;
use std::sync::Arc;
use tokio::sync::Mutex;

/// Component for interacting with AWS CloudWatch logs
pub struct CloudWatch {
    /// Component type identifier
    component_type: ComponentType,
    /// Client for CloudWatch API interactions
    cloudwatch_client: Option<Arc<Mutex<CloudWatchClient>>>,
    /// Currently selected CloudWatch log group
    selected_log_group: Option<String>,
    /// Input box for time range filtering
    time_range_input: InputBoxWidget,
    /// Current time range value
    time_range: Option<String>,
    /// AWS service client
    aws_clients: Option<TabClients>,
    
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

impl CloudWatch {
    /// Creates a new CloudWatch component with the provided event sender
    pub fn new(event_sender: tokio::sync::mpsc::UnboundedSender<Event>) -> Self {
        let popup_content = PopupContent::Profiles(vec!["No content".to_string()]);
        
        Self {
            component_type: ComponentType::CloudWatch,
            cloudwatch_client: None,
            selected_log_group: None,
            time_range_input: InputBoxWidget::new(
                InputBoxType::TimeRange,
                "Time Range (e.g. 1h, 1d, 7d)",
                false,
            ),
            time_range: None,
            aws_clients: None,
            
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

    /// Sets focus to the results area (typically the last widget in focus order)
    fn set_focus_to_last(&mut self) -> ComponentFocus {
        self.current_focus = ComponentFocus::Results;
        self.current_focus
    }

    /// Handles the selection of a log group and fetches its logs
    async fn handle_log_group_selection(&mut self, log_group: String) {
        self.selected_log_group = Some(log_group.clone());
        self.navigator
            .set_title(format!("Log Group: {}", log_group));

        let time_range = self.time_range.clone().unwrap_or_else(|| "5m".to_string());
        let filter_pattern = self.input.get_content().unwrap_or_default();

        // Fetch logs with current filter and time range
        self.fetch_logs(&log_group, &filter_pattern, &time_range, "Log Events")
            .await;
    }

    /// Fetches logs with the specified parameters and updates the UI
    ///
    /// Consolidates the previous separate log fetching methods into one
    /// Fetches logs with the specified parameters and updates the UI
    ///
    /// Uses background task to prevent UI blocking
    async fn fetch_logs(
        &mut self,
        log_group: &str,
        filter_pattern: &str,
        time_range: &str,
        title_prefix: &str,
    ) {
        if let Some(client_ref) = &self.cloudwatch_client {
            // Show loading state immediately
            let title = if filter_pattern.is_empty() {
                format!("{} (Loading...)", title_prefix)
            } else {
                format!("{}: {} (Loading...)", title_prefix, filter_pattern)
            };

            self.event_sender
                .send(Event::Tab(TabEvent::ComponentActions(
                    ComponentAction::WidgetAction(WidgetAction::ServiceNavigatorEvent(
                        ServiceNavigatorEvent::UpdateTitle(title),
                        WidgetType::QueryResultsNavigator,
                    )),
                    self.component_type.clone(),
                )))
                .unwrap_or_default();
            self.event_sender
                .send(Event::Tab(TabEvent::ComponentActions(
                    ComponentAction::WidgetAction(WidgetAction::ServiceNavigatorEvent(
                        ServiceNavigatorEvent::UpdateContent(vec![
                            "Fetching logs, please wait...".to_string(),
                        ], false),
                        WidgetType::QueryResultsNavigator,
                    )),
                    self.component_type.clone(),
                )))
                .unwrap_or_default();

            // Clone what we need for the background task
            let client_clone = Arc::clone(client_ref);
            let log_group = log_group.to_string();
            let filter_pattern = filter_pattern.to_string();
            let time_range = time_range.to_string();
            let event_sender = self.event_sender.clone();
            let title = title_prefix.to_string();
            let component_type = self.component_type.clone();
            // Spawn background task to fetch logs without blocking UI
            let _ = tokio::spawn(async move {
                // Fetch logs in background

                let logs_result = match tokio::time::timeout(
                    std::time::Duration::from_secs(330), // 30-second timeout
                    client_clone.lock().await.list_log_events(
                        &log_group,
                        &filter_pattern,
                        Some(&time_range),
                    ),
                )
                .await
                {
                    Ok(result) => result,
                    Err(_) => Ok(vec!["Request timed out after 30 seconds".to_string()]),
                };
                // Send event with results back to the component
                match logs_result {
                    Ok(logs) => {
                        // Send event with logs
                        event_sender
                            .send(Event::Tab(TabEvent::ComponentActions(
                                ComponentAction::WidgetAction(WidgetAction::ServiceNavigatorEvent(
                                    ServiceNavigatorEvent::UpdateContent(logs, false),
                                    WidgetType::QueryResultsNavigator,
                                )),
                                component_type.clone(),
                            )))
                            .unwrap_or_default();
                        event_sender
                            .send(Event::Tab(TabEvent::ComponentActions(
                                ComponentAction::WidgetAction(WidgetAction::ServiceNavigatorEvent(
                                    ServiceNavigatorEvent::UpdateTitle(title),
                                    WidgetType::QueryResultsNavigator,
                                )),
                                component_type.clone(),
                            )))
                            .unwrap_or_default();
                    }
                    Err(err) => {
                        // Send event with error message
                        event_sender
                            .send(Event::Tab(TabEvent::ComponentActions(
                                ComponentAction::WidgetAction(WidgetAction::ServiceNavigatorEvent(
                                    ServiceNavigatorEvent::UpdateContent(vec![err.to_string()], false),
                                    WidgetType::QueryResultsNavigator,
                                )),
                                component_type.clone(),
                            )))
                            .unwrap_or_default();
                    }
                }
            });
        }
    }

    /// Sets the time range and refreshes the current view
    async fn set_time_range(&mut self, time_range: String) {
        self.time_range = Some(time_range.clone());

        // If a log group is selected, refresh the logs with the new time range
        if let Some(log_group) = &self.selected_log_group {
            let log_group = log_group.clone();
            let filter = self.input.get_content().unwrap_or_default();
            self.fetch_logs(&log_group, &filter, &time_range, "Search Results")
                .await;
        }
    }

    /// Shows detailed view of a log entry in a popup
    async fn view_log_details(&mut self, log_content: &str) {
        self.details_popup
            .set_content(PopupContent::Details(log_content.to_string()));
        self.details_popup.set_visible(true);
        self.details_popup.set_active(true);
    }

    /// Updates focus for the time range input and other components
    fn update_time_range_focus(&mut self, activate: bool) {
        self.time_range_input.set_active(activate);
        self.input.set_active(!activate);
        self.navigator.set_active(!activate);
        self.results_navigator.set_active(!activate);

        if activate {
            self.current_focus = ComponentFocus::TimeRange;
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
                items.push(("Enter".to_string(), "Select log group".to_string()));
                items.push(("Alt+2".to_string(), "Focus results".to_string()));
                items.push(("Alt+3".to_string(), "Focus input".to_string()));
            }
            ComponentFocus::Results => {
                items.push(("Enter".to_string(), "View log details".to_string()));
                items.push(("Alt+1".to_string(), "Focus log groups".to_string()));
                items.push(("Alt+3".to_string(), "Focus input".to_string()));
            }
            ComponentFocus::Input => {
                items.push(("Enter".to_string(), "Search logs".to_string()));
                items.push(("Alt+1".to_string(), "Focus log groups".to_string()));
                items.push(("Alt+2".to_string(), "Focus results".to_string()));
            }
            _ => {}
        }
        items
    }
}

#[async_trait::async_trait]
impl AWSComponent for CloudWatch {
    fn render(&self, area: Rect, buf: &mut Buffer) {
        if !self.visible {
            return;
        }

        // Create a horizontal split for left panel (log groups) and right panel (log events)
        let horizontal_split = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(30), // Left panel - log groups list
                Constraint::Percentage(70), // Right panel - log events and search
            ])
            .split(area);

        // Create a vertical split for the right panel to separate inputs from results
        let right_vertical_split = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3), // Input row (search + time range)
                Constraint::Min(1),    // Log events results
            ])
            .split(horizontal_split[1]);

        // Create a horizontal split for the input area to place search and time range side by side
        let input_row = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(75), // Search filter input (3/4 width)
                Constraint::Percentage(25), // Time range input (1/4 width)
            ])
            .split(right_vertical_split[0]);

        // Render components
        self.navigator.render(horizontal_split[0], buf);

        // Render the search input box
        self.input.render(input_row[0], buf);

        // Render the time range input box
        self.time_range_input.render(input_row[1], buf);

        // Render the results navigator
        self.results_navigator.render(right_vertical_split[1], buf);

        // Render popup if visible
        if self.details_popup.is_visible() {
            self.details_popup.render(area, buf);
        }
    }

    /// Handles keyboard input for the CloudWatch component
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
                self.update_widget_states();
            }
            KeyCode::Char('2') if key_event.modifiers == KeyModifiers::ALT => {
                self.current_focus = ComponentFocus::Input;
                self.update_widget_states();
            }
            KeyCode::Char('3') if key_event.modifiers == KeyModifiers::ALT => {
                self.update_time_range_focus(true);
            }
            KeyCode::Char('4') if key_event.modifiers == KeyModifiers::ALT => {
                self.current_focus = ComponentFocus::Results;
                self.update_widget_states();
            }
            KeyCode::Esc => {
                if self.current_focus != ComponentFocus::Navigation {
                    self.current_focus = ComponentFocus::Navigation;
                    self.update_widget_states();
                }
            }
            _ => {
                // Forward input to the currently focused widget
                if let Some(signal) = match self.current_focus {
                    ComponentFocus::Navigation => self.navigator.handle_input(key_event),
                    ComponentFocus::Input => self.input.handle_input(key_event),
                    ComponentFocus::TimeRange => self.time_range_input.handle_input(key_event),
                    ComponentFocus::Results => self.results_navigator.handle_input(key_event),
                    ComponentFocus::None => None,
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

    /// Processes CloudWatch-specific component actions
    async fn process_event(&mut self, event: ComponentAction) {
        match event {
            cw_event => match cw_event {
                ComponentAction::Active(aws_profile) => {
                    self.aws_clients =
                        Some(TabClients::new(aws_profile, String::from("eu-west-1")));

                    // Unwrap the Result and handle errors properly
                    if let Some(clients) = &mut self.aws_clients {
                        match clients.get_cloudwatch_client().await {
                            Ok(client) => {
                                self.cloudwatch_client = Some(client);
                                self.update().await.ok();
                            }
                            Err(err) => {
                                // Handle the error (show error in UI)
                                self.results_navigator
                                    .set_title(String::from("Error connecting to CloudWatch"));
                                self.results_navigator.set_content(NavigatorContent::Records(vec![format!(
                                    "Failed to initialize CloudWatch client: {}",
                                    err
                                )]));
                            }
                        }
                    }
                }
                ComponentAction::Focused => {
                    // Set the component as inactive
                    self.set_active(true);
                }
                ComponentAction::Unfocused => {
                    if self.get_current_focus() == ComponentFocus::None {
                        self.reset_focus();
                    }
                    // Set the component as inactive
                    self.set_active(false);
                }
                ComponentAction::FocusedToLast => {
                    // Set the component as inactive
                }

                // Handle selection of a log group from the list
                ComponentAction::SelectLogGroup(log_group) => {
                    self.handle_log_group_selection(log_group).await;
                }
                // Handle search/filter request for logs
                ComponentAction::SearchLogs(filter) => {
                    if let Some(log_group) = &self.selected_log_group {
                        let log_group = log_group.clone();
                        let time_range =
                            self.time_range.clone().unwrap_or_else(|| "5m".to_string());
                        self.fetch_logs(&log_group, &filter, &time_range, "Search Results")
                            .await;
                    }
                }
                // Handle time range setting
                ComponentAction::SetTimeRange(time_range) => {
                    self.set_time_range(time_range).await;
                }
                // Display detailed view of a log entry
                ComponentAction::ViewLogDetails(log_content) => {
                    self.view_log_details(&log_content).await;
                }
                // Cycle focus forward through widgets
                ComponentAction::NextFocus => {
                    // If we're on TimeRange focus, we need special handling
                    if self.current_focus == ComponentFocus::TimeRange {
                        self.update_time_range_focus(false);
                        self.current_focus = ComponentFocus::Results;
                        self.update_widget_states();
                    } else {
                        let prev_focus = self.current_focus;
                        self.focus_next();

                        // If we just moved to TimeRange, activate time range input
                        if prev_focus != ComponentFocus::TimeRange
                            && self.current_focus == ComponentFocus::TimeRange
                        {
                            self.update_time_range_focus(true);
                        } else {
                            self.update_widget_states();
                        }
                    }
                }
                // Cycle focus backward through widgets
                ComponentAction::PreviousFocus => {
                    // If we're on TimeRange focus, we need special handling
                    if self.current_focus == ComponentFocus::TimeRange {
                        self.update_time_range_focus(false);
                        self.current_focus = ComponentFocus::Input;
                        self.update_widget_states();
                    } else {
                        let prev_focus = self.current_focus;
                        self.focus_previous();

                        // If we just moved to TimeRange, activate time range input
                        if prev_focus != ComponentFocus::TimeRange
                            && self.current_focus == ComponentFocus::TimeRange
                        {
                            self.update_time_range_focus(true);
                        } else {
                            self.update_widget_states();
                        }
                    }
                }
                // Show details in popup window
                ComponentAction::PopupDetails(details) => {
                    self.details_popup
                        .set_content(PopupContent::Details(details.clone()));
                    self.details_popup.set_visible(true);
                    self.details_popup.set_active(true);
                }
                // Process events from child widgets
                ComponentAction::WidgetAction(widget_action) => match widget_action {
                    WidgetAction::ServiceNavigatorEvent(ref _aws_navigator_event, widget_type) => {
                        if widget_type == WidgetType::AWSServiceNavigator {
                            if let Some(signal) =
                                self.navigator.process_event(widget_action.clone())
                            {
                                match signal {
                                    // User selected a log group from the navigator
                                    WidgetAction::ServiceNavigatorEvent(
                                        ServiceNavigatorEvent::ItemSelected(
                                            WidgetEventType::RecordSelected(log_group),
                                        ),
                                        WidgetType::AWSServiceNavigator,
                                    ) => {
                                        self.event_sender
                                            .send(Event::Tab(TabEvent::ComponentActions(
                                                ComponentAction::SelectLogGroup(log_group),
                                                self.component_type.clone(),
                                            )))
                                            .unwrap();
                                    }
                                    _ => {}
                                }
                            }
                        } else if widget_type == WidgetType::QueryResultsNavigator {
                            if let Some(signal) = self
                                .results_navigator
                                .process_event(widget_action.clone())
                            {
                                match signal {
                                    // User selected a log entry to view details
                                    WidgetAction::ServiceNavigatorEvent(
                                        ServiceNavigatorEvent::ItemSelected(
                                            WidgetEventType::RecordSelected(log_content),
                                        ),
                                        WidgetType::QueryResultsNavigator,
                                    ) => {
                                        // Show log details in popup
                                        self.event_sender
                                            .send(Event::Tab(TabEvent::ComponentActions(
                                                ComponentAction::PopupDetails(log_content),
                                                self.component_type.clone(),
                                            )))
                                            .unwrap();
                                    }
                                    _ => {}
                                }
                            }
                        }
                    }
                    WidgetAction::InputBoxEvent(ref _input_box_event, ref input_type) => {
                        match input_type {
                            InputBoxType::Text => {
                                if let Some(signal) =
                                    self.input.process_event(widget_action.clone())
                                {
                                    if let WidgetAction::InputBoxEvent(
                                        InputBoxEvent::Written(content),
                                        _,
                                    ) = signal
                                    {
                                        // Use input content to filter logs
                                        if self.selected_log_group.is_some() {
                                            self.event_sender
                                                .send(Event::Tab(TabEvent::ComponentActions(
                                                    ComponentAction::SearchLogs(content),
                                                    self.component_type.clone(),
                                                )))
                                                .unwrap();
                                        }
                                    }
                                }
                            }
                            // Check if it's from time range input
                            InputBoxType::TimeRange => {
                                if let Some(signal) =
                                    self.time_range_input.process_event(widget_action.clone())
                                {
                                    if let WidgetAction::InputBoxEvent(
                                        InputBoxEvent::Written(content),
                                        _,
                                    ) = signal
                                    {
                                        // Set the time range and refresh logs
                                        self.event_sender
                                            .send(Event::Tab(TabEvent::ComponentActions(
                                                ComponentAction::SetTimeRange(content),
                                                self.component_type.clone(),
                                            )))
                                            .unwrap();
                                    }
                                }
                            }
                            _ => {}
                        }
                    }
                    // Close popup when exit event received
                    WidgetAction::PopupAction(_) => {
                        self.details_popup.set_visible(false);
                        self.details_popup.set_active(false);
                    }
                    _ => {}
                },
                _ => {}
            },
        }
    }

    /// Sets the active state of this component
    fn set_active(&mut self, active: bool) {
        self.active = active;
        self.time_range_input.set_active(false); // Always reset time range input active state
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

    /// Fetches and displays the list of CloudWatch log groups
    async fn update(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(client) = &self.cloudwatch_client {
            // Show loading state immediately
            self.navigator.set_title(String::from("Log Groups (Loading...)"));
            self.navigator.set_content(NavigatorContent::Records(vec![
                "Fetching log groups, please wait...".to_string()
            ]));
            
            // Reset results area
            self.results_navigator.set_content(NavigatorContent::Records(vec![]));
            self.results_navigator.set_title(String::from("Select a log group"));
            
            // Clone what we need for the background task
            let client_clone = Arc::clone(client);
            let event_sender = self.event_sender.clone();
            let component_type = self.component_type.clone();
            
            // Spawn background task to fetch log groups without blocking UI
            let _ = tokio::spawn(async move {
                // Fetch log groups in background
                let log_groups_result = match tokio::time::timeout(
                    std::time::Duration::from_secs(30), // 30-second timeout
                    client_clone.lock().await.list_log_groups(),
                ).await {
                    Ok(result) => result,
                    Err(_) => Ok(vec!["Request timed out after 30 seconds".to_string()]),
                };
                
                // Send event with results back to the component
                match log_groups_result {
                    Ok(log_groups) => {
                        // Send event to update navigator with log groups
                        event_sender
                            .send(Event::Tab(TabEvent::ComponentActions(
                                ComponentAction::WidgetAction(WidgetAction::ServiceNavigatorEvent(
                                    ServiceNavigatorEvent::UpdateContent(log_groups, false),
                                    WidgetType::AWSServiceNavigator,
                                )),
                                component_type.clone(),
                            )))
                            .unwrap_or_default();
                        
                        // Update navigator title
                        event_sender
                            .send(Event::Tab(TabEvent::ComponentActions(
                                ComponentAction::WidgetAction(WidgetAction::ServiceNavigatorEvent(
                                    ServiceNavigatorEvent::UpdateTitle(String::from("Log Groups")),
                                    WidgetType::AWSServiceNavigator,
                                )),
                                component_type.clone(),
                            )))
                            .unwrap_or_default();
                    },
                    Err(err) => {
                        // Send event with error message
                        event_sender
                            .send(Event::Tab(TabEvent::ComponentActions(
                                ComponentAction::WidgetAction(WidgetAction::ServiceNavigatorEvent(
                                    ServiceNavigatorEvent::UpdateContent(vec![format!(
                                        "Error fetching log groups: {}", err
                                    )], false),
                                    WidgetType::AWSServiceNavigator,
                                )),
                                component_type.clone(),
                            )))
                            .unwrap_or_default();
                        
                        // Update navigator title to reflect error
                        event_sender
                            .send(Event::Tab(TabEvent::ComponentActions(
                                ComponentAction::WidgetAction(WidgetAction::ServiceNavigatorEvent(
                                    ServiceNavigatorEvent::UpdateTitle(String::from("Log Groups (Error)")),
                                    WidgetType::AWSServiceNavigator,
                                )),
                                component_type,
                            )))
                            .unwrap_or_default();
                    },
                }
            });
        }
        Ok(())
    }

    fn get_current_focus(&self) -> ComponentFocus {
        self.current_focus
    }

    /// Resets focus to the navigation pane
    fn reset_focus(&mut self) {
        self.current_focus = ComponentFocus::Navigation;
        self.update_time_range_focus(false);
        self.update_widget_states();
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }

    /// Restores focus to the last active widget
    fn set_focus_to_last(&mut self) {
        self.set_focus_to_last();

        // Special handling for TimeRange focus
        if self.current_focus == ComponentFocus::TimeRange {
            self.update_time_range_focus(true);
        } else {
            self.update_widget_states();
        }
    }

    fn get_help_items(&self) -> Vec<(String, String)> {
        let mut help_items = vec![];

        // Add time range specific help when time range input is focused
        if self.current_focus == ComponentFocus::TimeRange {
            help_items.push(("Enter".to_string(), "Apply time range".to_string()));
            help_items.push(("Time formats".to_string(), "15m, 1h, 1d, 7d".to_string()));
            help_items.push(("Esc".to_string(), "Return to navigation".to_string()));
        } else {
            // Return default help items based on the base component's state
            help_items = self.get_base_help_items();

            // Add time range navigation helper
            help_items.push(("Alt+3".to_string(), "Time range".to_string()));
        }

        help_items
    }
}
