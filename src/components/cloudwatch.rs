use crate::components::aws_base_component::AWSComponentBase;
use crate::components::{AWSComponent, ComponentFocus};
use crate::event_managment::event::{
    ServiceNavigatorEvent, CloudWatchComponentActions, ComponentActions, Event, InputBoxEvent,
    TabEvent, WidgetAction, WidgetEventType, WidgetType,
};
use crate::services::aws::cloudwatch_client::CloudWatchClient;
use crate::widgets::WidgetExt;
use crate::widgets::service_navigator::NavigatorContent;
use crate::widgets::popup::PopupContent;
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
    /// Common AWS component functionality
    base: AWSComponentBase,
    /// Client for CloudWatch API interactions
    cloudwatch_client: Option<Arc<Mutex<CloudWatchClient>>>,
    /// Currently selected CloudWatch log group
    selected_log_group: Option<String>,
}

impl CloudWatch {
    /// Creates a new CloudWatch component with the provided event sender
    pub fn new(event_sender: tokio::sync::mpsc::UnboundedSender<Event>) -> Self {
        Self {
            base: AWSComponentBase::new(event_sender.clone(), NavigatorContent::Records(vec![])),
            cloudwatch_client: None,
            selected_log_group: None,
        }
    }

    /// Assigns a CloudWatch client to this component
    pub fn set_client(&mut self, cloudwatch_client: Arc<Mutex<CloudWatchClient>>) {
        self.cloudwatch_client = Some(cloudwatch_client);
    }

    /// Handles the selection of a log group and fetches its logs
    async fn handle_log_group_selection(&mut self, log_group: String) {
        self.selected_log_group = Some(log_group.clone());
        self.base
            .navigator
            .set_title(format!("Log Group: {}", log_group));

        if let Some(client) = &self.cloudwatch_client {
            let logs = client
                .lock()
                .await
                .list_log_events(&log_group, "")
                .await
                .unwrap_or_else(|_| vec!["No log events found".to_string()]);

            self.base
                .results_navigator
                .set_title(String::from("Log Events"));
            self.base
                .results_navigator
                .set_content(NavigatorContent::Records(logs));
        }
    }

    /// Performs a filtered search on logs from the specified log group
    async fn search_logs(&mut self, log_group: &str, filter_pattern: &str) {
        if let Some(client) = &self.cloudwatch_client {
            let logs = client
                .lock()
                .await
                .list_log_events(log_group, filter_pattern)
                .await
                .unwrap_or_else(|_| vec!["No matching logs found".to_string()]);

            self.base
                .results_navigator
                .set_title(format!("Search Results: {}", filter_pattern));
            self.base
                .results_navigator
                .set_content(NavigatorContent::Records(logs));
        }
    }

    /// Shows detailed view of a log entry in a popup
    async fn view_log_details(&mut self, log_content: &str) {
        self.base
            .details_popup
            .set_content(PopupContent::Details(log_content.to_string()));
        self.base.details_popup.set_visible(true);
        self.base.details_popup.set_active(true);
    }
}

#[async_trait::async_trait]
impl AWSComponent for CloudWatch {
    fn render(&self, area: Rect, buf: &mut Buffer) {
        if !self.base.visible {
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

        // Create a vertical split for the right panel
        let right_vertical_split = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Percentage(20), // Log events list
                Constraint::Percentage(80), // Search/filter input
            ])
            .split(horizontal_split[1]);

        // Render components
        self.base.navigator.render(horizontal_split[0], buf);
        self.base.input.render(right_vertical_split[0], buf);

        self.base
            .results_navigator
            .render(right_vertical_split[1], buf);

        if self.base.details_popup.is_visible() {
            self.base.details_popup.render(area, buf);
        }
    }

    /// Handles keyboard input for the CloudWatch component
    fn handle_input(&mut self, key_event: KeyEvent) {
        // Special handling for popup details if visible
        if self.base.details_popup.is_visible() {
            if let Some(signal) = self.base.details_popup.handle_input(key_event) {
                self.base
                    .event_sender
                    .send(Event::Tab(TabEvent::ComponentActions(
                        ComponentActions::CloudWatchComponentActions(
                            CloudWatchComponentActions::WidgetAction(signal),
                        ),
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
                        ComponentActions::CloudWatchComponentActions(
                            CloudWatchComponentActions::NextFocus,
                        ),
                    )))
                    .unwrap();
            }
            KeyCode::BackTab => {
                self.base
                    .event_sender
                    .send(Event::Tab(TabEvent::ComponentActions(
                        ComponentActions::CloudWatchComponentActions(
                            CloudWatchComponentActions::PreviousFocus,
                        ),
                    )))
                    .unwrap();
            }
            // Alt+number shortcuts to switch focus between areas
            KeyCode::Char('1') if key_event.modifiers == KeyModifiers::ALT => {
                self.base.current_focus = ComponentFocus::Navigation;
                self.base.update_widget_states();
            }
            KeyCode::Char('2') if key_event.modifiers == KeyModifiers::ALT => {
                self.base.current_focus = ComponentFocus::Results;
                self.base.update_widget_states();
            }
            KeyCode::Char('3') if key_event.modifiers == KeyModifiers::ALT => {
                self.base.current_focus = ComponentFocus::Input;
                self.base.update_widget_states();
            }
            KeyCode::Esc => {
                if self.base.current_focus != ComponentFocus::Navigation {
                    self.base.current_focus = ComponentFocus::Navigation;
                    self.base.update_widget_states();
                }
            }
            _ => {
                // Forward input to the currently focused widget
                if let Some(signal) = match self.base.current_focus {
                    ComponentFocus::Navigation => self.base.navigator.handle_input(key_event),
                    ComponentFocus::Input => self.base.input.handle_input(key_event),
                    ComponentFocus::Results => self.base.results_navigator.handle_input(key_event),
                    ComponentFocus::None => None,
                } {
                    self.base
                        .event_sender
                        .send(Event::Tab(TabEvent::ComponentActions(
                            ComponentActions::CloudWatchComponentActions(
                                CloudWatchComponentActions::WidgetAction(signal),
                            ),
                        )))
                        .unwrap();
                }
            }
        }
    }

    /// Processes CloudWatch-specific component actions
    async fn process_event(&mut self, event: ComponentActions) {
        match event {
            ComponentActions::CloudWatchComponentActions(cw_event) => match cw_event {
                // Handle selection of a log group from the list
                CloudWatchComponentActions::SelectLogGroup(log_group) => {
                    self.handle_log_group_selection(log_group).await;
                }
                // Handle search/filter request for logs
                CloudWatchComponentActions::SearchLogs(filter) => {
                    if let Some(log_group) = &self.selected_log_group {
                        let log_group_clone = log_group.clone();
                        self.search_logs(&log_group_clone, &filter).await;
                    }
                }
                // Display detailed view of a log entry
                CloudWatchComponentActions::ViewLogDetails(log_content) => {
                    self.view_log_details(&log_content).await;
                }
                // Cycle focus forward through widgets
                CloudWatchComponentActions::NextFocus => {
                    self.base.focus_next();
                    self.base.update_widget_states();
                }
                // Cycle focus backward through widgets
                CloudWatchComponentActions::PreviousFocus => {
                    self.base.focus_previous();
                    self.base.update_widget_states();
                }
                // Show details in popup window
                CloudWatchComponentActions::PopupDetails(details) => {
                    self.base
                        .details_popup
                        .set_content(PopupContent::Details(details.clone()));
                    self.base.details_popup.set_visible(true);
                    self.base.details_popup.set_active(true);
                }
                // Process events from child widgets
                CloudWatchComponentActions::WidgetAction(widget_action) => match widget_action {
                    WidgetAction::ServiceNavigatorEvent(
                        ref _aws_navigator_event,
                        widget_type,
                    ) => {
                        if widget_type == WidgetType::AWSServiceNavigator {
                            if let Some(signal) =
                                self.base.navigator.process_event(widget_action.clone())
                            {
                                match signal {
                                    // User selected a log group from the navigator
                                    WidgetAction::ServiceNavigatorEvent(
                                        ServiceNavigatorEvent::ItemSelected(
                                            WidgetEventType::RecordSelected(log_group),
                                        ),
                                        WidgetType::AWSServiceNavigator,
                                    ) => {
                                        self.base
                                            .event_sender
                                            .send(Event::Tab(TabEvent::ComponentActions(
                                                ComponentActions::CloudWatchComponentActions(
                                                    CloudWatchComponentActions::SelectLogGroup(
                                                        log_group,
                                                    ),
                                                ),
                                            )))
                                            .unwrap();
                                    }
                                    _ => {}
                                }
                            }
                        } else if widget_type == WidgetType::QueryResultsNavigator {
                            if let Some(signal) = self
                                .base
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
                                        self.base
                                            .event_sender
                                            .send(Event::Tab(TabEvent::ComponentActions(
                                                ComponentActions::CloudWatchComponentActions(
                                                    CloudWatchComponentActions::PopupDetails(
                                                        log_content,
                                                    ),
                                                ),
                                            )))
                                            .unwrap();
                                    }
                                    _ => {}
                                }
                            }
                        }
                    }
                    WidgetAction::InputBoxEvent(ref _input_box_event) => {
                        if let Some(signal) = self.base.input.process_event(widget_action) {
                            if let WidgetAction::InputBoxEvent(InputBoxEvent::Written(content)) =
                                signal
                            {
                                // Use input content to filter logs
                                if let Some(_) = &self.selected_log_group {
                                    self.base
                                        .event_sender
                                        .send(Event::Tab(TabEvent::ComponentActions(
                                            ComponentActions::CloudWatchComponentActions(
                                                CloudWatchComponentActions::SearchLogs(content),
                                            ),
                                        )))
                                        .unwrap();
                                }
                            }
                        }
                    }
                    // Close popup when exit event received
                    WidgetAction::PopupAction(_) => {
                        self.base.details_popup.set_visible(false);
                        self.base.details_popup.set_active(false);
                    }
                    _ => {}
                },
            },
            _ => {} // Ignore other component actions that don't belong to CloudWatch
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

    /// Fetches and displays the list of CloudWatch log groups
    async fn update(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(client) = &self.cloudwatch_client {
            let client = client.lock().await;
            let log_groups = client.list_log_groups().await?;
            self.base.navigator.set_title(String::from("Log Groups"));
            self.base
                .navigator
                .set_content(NavigatorContent::Records(log_groups));

            // Reset results area
            self.base
                .results_navigator
                .set_content(NavigatorContent::Records(vec![]));
            self.base
                .results_navigator
                .set_title(String::from("Select a log group"));
        }
        Ok(())
    }

    fn get_current_focus(&self) -> ComponentFocus {
        self.base.current_focus
    }

    /// Resets focus to the navigation pane
    fn reset_focus(&mut self) {
        self.base.current_focus = ComponentFocus::Navigation;
        self.base.update_widget_states();
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }

    /// Restores focus to the last active widget
    fn set_focus_to_last(&mut self) {
        self.base.set_focus_to_last();
        self.base.update_widget_states();
    }

    fn get_help_items(&self) -> Vec<(String, String)> {
        // Return help items based on the base component's state
        self.base.get_help_items()
    }
}