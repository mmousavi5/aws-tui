use crate::components::{AWSComponent};
use crate::event_managment::event::{
    ComponentAction, ComponentType, Event, InputBoxEvent, ServiceNavigatorEvent, TabEvent,
    WidgetAction, WidgetEventType, WidgetType, InputBoxType,
};
use crate::services::aws::TabClients;
use crate::services::aws::s3_client::S3Client;
use crate::widgets::WidgetExt;
use crate::widgets::popup::{PopupContent, PopupWidget};
use crate::widgets::service_navigator::{NavigatorContent, ServiceNavigator};
use crate::widgets::input_box::InputBoxWidget;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Direction, Layout, Rect},
};
use std::any::Any;
use std::sync::Arc;
use tokio::sync::Mutex;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum S3Focus {
    /// Focus on the left navigation area (service list/tables/buckets)
    Navigation,
    /// Focus on the input area (search/filter/command box)
    Input,
    /// Focus on the results display area
    Results,
    /// No focus set
    None,
}


/// Component for interacting with AWS S3 storage
pub struct S3Component {
    /// Component type identifier
    component_type: ComponentType,
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
    current_focus: S3Focus,
    /// Client for S3 API interactions
    s3_client: Option<Arc<Mutex<S3Client>>>,
    /// Current path within the selected bucket
    current_path: String,
    /// Currently selected S3 bucket name
    selected_bucket: Option<String>,
    /// AWS service client
    aws_clients: Option<TabClients>,
}

impl S3Component {
    /// Creates a new S3 component with the provided event sender
    pub fn new(event_sender: tokio::sync::mpsc::UnboundedSender<Event>) -> Self {
        let popup_content = PopupContent::Profiles(vec!["No content".to_string()]);

        Self {
            component_type: ComponentType::DynamoDB,
            navigator: ServiceNavigator::new(
                crate::event_managment::event::WidgetType::AWSServiceNavigator,
                false,
                NavigatorContent::Records(vec![]),
            ),
            input: InputBoxWidget::new(InputBoxType::Text, "Query Input", false),
            results_navigator: ServiceNavigator::new(
                crate::event_managment::event::WidgetType::QueryResultsNavigator,
                false,
                NavigatorContent::Records(vec![]),
            ),
            details_popup: PopupWidget::new(popup_content, "Details", false, false),
            active: false,
            visible: true,
            event_sender,
            current_focus: S3Focus::Navigation,
            s3_client: None,
            current_path: String::new(),
            selected_bucket: None,
            aws_clients: None,
        }
    }

    /// Updates active states of all widgets based on current focus
    fn update_widget_states(&mut self) {
        self.navigator
            .set_active(self.active & (self.current_focus == S3Focus::Navigation));
        self.input
            .set_active(self.active & (self.current_focus == S3Focus::Input));
        self.results_navigator
            .set_active(self.active & (self.current_focus == S3Focus::Results));
    }

    /// Shifts focus to the previous widget in the cyclic order
    fn focus_previous(&mut self) -> S3Focus {
        self.current_focus = match self.current_focus {
            S3Focus::Navigation => S3Focus::Input,
            S3Focus::Input => S3Focus::Results,
            S3Focus::Results => S3Focus::None,
            S3Focus::None => S3Focus::Navigation,
        };
        self.current_focus
    }

    /// Shifts focus to the next widget in the cyclic order
    fn focus_next(&mut self) -> S3Focus {
        self.current_focus = match self.current_focus {
            S3Focus::Navigation => S3Focus::Results,
            S3Focus::Results => S3Focus::Input,
            S3Focus::Input => S3Focus::None,
            S3Focus::None => S3Focus::Navigation,
        };
        self.current_focus
    }

    /// Handles the selection of a bucket and fetches its contents
    async fn handle_bucket_selection(&mut self, bucket_name: String) {
        self.selected_bucket = Some(bucket_name.clone());
        self.current_path = String::new();
        self.navigator
            .set_title(format!("Bucket: {}", bucket_name));

        if let Some(client) = &self.s3_client {
            let objects = client
                .lock()
                .await
                .list_objects(&bucket_name, "")
                .await
                .unwrap_or_else(|_| vec!["Error listing objects".to_string()]);

            self.results_navigator
                .set_title(String::from("Objects"));
            self.results_navigator
                .set_content(NavigatorContent::Records(objects));
        }
    }

    /// Navigate into a folder in the current bucket
    async fn navigate_folder(&mut self, path: String) {
        if let Some(bucket) = &self.selected_bucket {
            // Build full path by appending new path segment to current path
            let full_path = if self.current_path.is_empty() {
                path.clone()
            } else {
                format!("{}/{}", self.current_path, path)
            };

            self.current_path = full_path.clone();

            if let Some(client) = &self.s3_client {
                let objects = client
                    .lock()
                    .await
                    .list_objects(bucket, &full_path)
                    .await
                    .unwrap_or_else(|_| vec!["Error listing objects".to_string()]);

                self.results_navigator
                    .set_title(format!("Path: {}", full_path));
                self.results_navigator
                    .set_content(NavigatorContent::Records(objects));
            }
        }
    }

    /// Navigate up one directory level
    fn navigate_up(&mut self) {
        if !self.current_path.is_empty() {
            // Remove the last directory from the path
            if let Some(last_slash) = self.current_path.rfind('/') {
                self.current_path = self.current_path[..last_slash].to_string();
            } else {
                self.current_path = String::new();
            }

            // Send event to update objects list with new path
            if let Some(bucket) = &self.selected_bucket {
                self.event_sender
                    .send(Event::Tab(TabEvent::ComponentActions(
                        ComponentAction::LoadPath(bucket.clone(), self.current_path.clone()),
                        self.component_type.clone(),
                    )))
                    .unwrap();
            }
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
            S3Focus::Navigation => {
                items.push(("Enter".to_string(), "Select bucket".to_string()));
                items.push(("Alt+2".to_string(), "Focus results".to_string()));
                items.push(("Alt+3".to_string(), "Focus input".to_string()));
            }
            S3Focus::Results => {
                items.push(("Enter".to_string(), "View object details".to_string()));
                items.push(("Backspace".to_string(), "Navigate up".to_string()));
                items.push(("Alt+1".to_string(), "Focus buckets".to_string()));
                items.push(("Alt+3".to_string(), "Focus input".to_string()));
            }
            S3Focus::Input => {
                items.push(("Enter".to_string(), "Search objects".to_string()));
                items.push(("Alt+1".to_string(), "Focus buckets".to_string()));
                items.push(("Alt+2".to_string(), "Focus results".to_string()));
            }
            _ => {}
        }
        items
    }
}

#[async_trait::async_trait]
impl AWSComponent for S3Component {
    fn render(&self, area: Rect, buf: &mut Buffer) {
        if !self.visible {
            return;
        }

        // Create a horizontal split for left panel (buckets) and right panel (objects)
        let horizontal_split = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(30), // Left panel - buckets list
                Constraint::Percentage(70), // Right panel - objects and details
            ])
            .split(area);

        // Create a vertical split for the right panel
        let right_vertical_split = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Percentage(20), // Objects list
                Constraint::Percentage(80), // Object details/metadata
            ])
            .split(horizontal_split[1]);

        // Render components
        self.navigator.render(horizontal_split[0], buf);
        self.results_navigator
            .render(right_vertical_split[1], buf);
        self.input.render(right_vertical_split[0], buf);

        if self.details_popup.is_visible() {
            self.details_popup.render(area, buf);
        }
    }

    /// Handles keyboard input for the S3 component
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
            KeyCode::Backspace => {
                // Navigate up one directory level
                if self.current_focus == S3Focus::Results {
                    self.event_sender
                        .send(Event::Tab(TabEvent::ComponentActions(
                            ComponentAction::NavigateUp,
                            self.component_type.clone(),
                        )))
                        .unwrap();
                }
            }
            // Alt+number shortcuts to switch focus between areas
            KeyCode::Char('1') if key_event.modifiers == KeyModifiers::ALT => {
                self.current_focus = S3Focus::Navigation;
                self.update_widget_states();
            }
            KeyCode::Char('2') if key_event.modifiers == KeyModifiers::ALT => {
                self.current_focus = S3Focus::Results;
                self.update_widget_states();
            }
            KeyCode::Char('3') if key_event.modifiers == KeyModifiers::ALT => {
                self.current_focus = S3Focus::Input;
                self.update_widget_states();
            }
            KeyCode::Esc => {
                if self.current_focus != S3Focus::Navigation {
                    self.current_focus = S3Focus::Navigation;
                    self.update_widget_states();
                }
            }
            _ => {
                // Forward input to the currently focused widget
                if let Some(signal) = match self.current_focus {
                    S3Focus::Navigation => self.navigator.handle_input(key_event),
                    S3Focus::Input => self.input.handle_input(key_event),
                    S3Focus::Results => self.results_navigator.handle_input(key_event),
                    S3Focus::None => None,
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

    /// Processes S3-specific component actions
    async fn process_event(&mut self, event: ComponentAction) {
        match event {
            s3_event => match s3_event {
                // Handle bucket selection
                ComponentAction::Active(aws_profile) => {
                    self.aws_clients =
                        Some(TabClients::new(aws_profile, String::from("eu-west-1")));

                    // Unwrap the Result and handle errors properly
                    if let Some(clients) = &mut self.aws_clients {
                        match clients.get_s3_client().await {
                            Ok(client) => {
                                self.s3_client = Some(client);
                                self.update().await.ok();
                            }
                            Err(err) => {
                                // Handle the error (show error in UI)
                                self.results_navigator
                                    .set_title(String::from("Error connecting to CloudWatch"));
                                self.results_navigator
                                    .set_content(NavigatorContent::Records(vec![format!(
                                        "Failed to initialize CloudWatch client: {}",
                                        err
                                    )]));
                            }
                        }
                    }
                }

                ComponentAction::Focused => {
                    // Set the component as inactive
                    self.set_active(false);
                }
                ComponentAction::Unfocused => {
                    self.reset_focus();
                    // Set the component as inactive
                    self.set_active(false);
                }
                ComponentAction::FocusedToLast => {
                    // Set the component as inactive
                }

                ComponentAction::SelectBucket(bucket) => {
                    self.handle_bucket_selection(bucket).await;
                }
                // Navigate into a folder
                ComponentAction::NavigateFolder(path) => {
                    self.navigate_folder(path).await;
                }
                // Navigate up to parent directory
                ComponentAction::NavigateUp => {
                    self.navigate_up();
                }
                // Load contents at a specific path
                ComponentAction::LoadPath(bucket, path) => {
                    if let Some(client) = &self.s3_client {
                        let objects = client
                            .lock()
                            .await
                            .list_objects(&bucket, &path)
                            .await
                            .unwrap_or_else(|_| vec!["Error listing objects".to_string()]);

                        self.results_navigator.set_title(format!(
                            "Path: {}",
                            if path.is_empty() { "/" } else { &path }
                        ));
                        self.results_navigator
                            .set_content(NavigatorContent::Records(objects));
                    }
                }
                // Display object details in popup
                ComponentAction::PopupDetails(key) => {
                    if let (Some(client), Some(bucket)) = (&self.s3_client, &self.selected_bucket) {
                        // Build full object key with current path
                        let full_key = if self.current_path.is_empty() {
                            key.clone()
                        } else {
                            format!("{}/{}", self.current_path, key)
                        };

                        match client
                            .lock()
                            .await
                            .get_object_details(bucket, &full_key)
                            .await
                        {
                            Ok(details) => {
                                self.details_popup
                                    .set_content(PopupContent::Details(details));
                                self.details_popup.set_visible(true);
                                self.details_popup.set_active(true);
                            }
                            Err(_) => {
                                self.details_popup.set_content(PopupContent::Details(
                                    "Error fetching object details".to_string(),
                                ));
                                self.details_popup.set_visible(true);
                                self.details_popup.set_active(true);
                            }
                        }
                    }
                }
                // Cycle focus forward through widgets
                ComponentAction::NextFocus => {
                    self.focus_next();
                    self.update_widget_states();
                }
                // Cycle focus backward through widgets
                ComponentAction::PreviousFocus => {
                    self.focus_previous();
                    self.update_widget_states();
                }
                // Process events from child widgets
                ComponentAction::WidgetAction(widget_action) => match widget_action {
                    WidgetAction::ServiceNavigatorEvent(ref _aws_navigator_event, widget_type) => {
                        if widget_type == WidgetType::AWSServiceNavigator {
                            if let Some(signal) =
                                self.navigator.process_event(widget_action.clone())
                            {
                                match signal {
                                    // User selected a bucket from the navigator
                                    WidgetAction::ServiceNavigatorEvent(
                                        ServiceNavigatorEvent::ItemSelected(
                                            WidgetEventType::RecordSelected(bucket),
                                        ),
                                        WidgetType::AWSServiceNavigator,
                                    ) => {
                                        self.event_sender
                                            .send(Event::Tab(TabEvent::ComponentActions(
                                                ComponentAction::SelectBucket(bucket),
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
                                    // User selected an object or folder from the results
                                    WidgetAction::ServiceNavigatorEvent(
                                        ServiceNavigatorEvent::ItemSelected(
                                            WidgetEventType::RecordSelected(path),
                                        ),
                                        WidgetType::QueryResultsNavigator,
                                    ) => {
                                        // Check if it's a folder (ends with /) or a file
                                        if path.ends_with('/') {
                                            let folder_name =
                                                path.trim_end_matches('/').to_string();
                                            self.event_sender
                                                .send(Event::Tab(TabEvent::ComponentActions(
                                                    ComponentAction::NavigateFolder(folder_name),
                                                    self.component_type.clone(),
                                                )))
                                                .unwrap();
                                        } else {
                                            // Show object details in popup
                                            self.event_sender
                                                .send(Event::Tab(TabEvent::ComponentActions(
                                                    ComponentAction::PopupDetails(path),
                                                    self.component_type.clone(),
                                                )))
                                                .unwrap();
                                        }
                                    }
                                    _ => {}
                                }
                            }
                        }
                    }
                    WidgetAction::InputBoxEvent(ref _input_box_event, _) => {
                        if let Some(signal) = self.input.process_event(widget_action) {
                            if let WidgetAction::InputBoxEvent(InputBoxEvent::Written(content), _) =
                                signal
                            {
                                // Handle search input when a bucket is selected
                                if let Some(bucket) = &self.selected_bucket {
                                    let search_path = if self.current_path.is_empty() {
                                        content.clone()
                                    } else {
                                        format!("{}/{}", self.current_path, content)
                                    };

                                    self.event_sender
                                        .send(Event::Tab(TabEvent::ComponentActions(
                                            ComponentAction::LoadPath(bucket.clone(), search_path),
                                            self.component_type.clone(),
                                        )))
                                        .unwrap();
                                }
                            }
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

    
    fn allows_focus_continuation(&self) -> bool {
        if self.current_focus == S3Focus::None {
            return  true;
        }
        false
    }

    fn allows_focus_continuation_backward(&self) -> bool {
        if self.current_focus != S3Focus::Navigation {
            return  true;
        }
        false
    }

    /// Fetches and displays the list of S3 buckets
    async fn update(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(client) = &self.s3_client {
            let client = client.lock().await;
            let buckets = client.list_buckets().await?;
            self.navigator
                .set_content(NavigatorContent::Records(buckets));

            // Reset results area
            self.results_navigator
                .set_content(NavigatorContent::Records(vec![]));
            self.results_navigator
                .set_title(String::from("Select a bucket"));
        }
        Ok(())
    }

    fn reset_focus(&mut self) {
        self.current_focus = S3Focus::Navigation;
        self.update_widget_states();
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }

    fn set_focus_to_last(&mut self) {
        self.current_focus = S3Focus::Results;
        self.update_widget_states();
    }

    fn get_help_items(&self) -> Vec<(String, String)> {
        // Return help items based on the component's state
        self.get_base_help_items()
    }
}
