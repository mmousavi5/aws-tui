use crate::components::aws_base_component::AWSComponentBase;
use crate::components::{AWSComponent, ComponentFocus};
use crate::event_managment::event::{
    ServiceNavigatorEvent, ComponentActions, Event, InputBoxEvent, S3ComponentActions, TabEvent,
    WidgetAction, WidgetEventType, WidgetType,
};
use crate::services::aws::s3_client::S3Client;
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

/// Component for interacting with AWS S3 storage
pub struct S3Component {
    /// Common AWS component functionality
    base: AWSComponentBase,
    /// Client for S3 API interactions
    s3_client: Option<Arc<Mutex<S3Client>>>,
    /// Current path within the selected bucket
    current_path: String,
    /// Currently selected S3 bucket name
    selected_bucket: Option<String>,
}

impl S3Component {
    /// Creates a new S3 component with the provided event sender
    pub fn new(event_sender: tokio::sync::mpsc::UnboundedSender<Event>) -> Self {
        Self {
            base: AWSComponentBase::new(event_sender.clone(), NavigatorContent::Records(vec![])),
            s3_client: None,
            current_path: String::new(),
            selected_bucket: None,
        }
    }

    /// Assigns an S3 client to this component
    pub fn set_client(&mut self, s3_client: Arc<Mutex<S3Client>>) {
        self.s3_client = Some(s3_client);
    }

    /// Handles the selection of a bucket and fetches its contents
    async fn handle_bucket_selection(&mut self, bucket_name: String) {
        self.selected_bucket = Some(bucket_name.clone());
        self.current_path = String::new();
        self.base
            .navigator
            .set_title(format!("Bucket: {}", bucket_name));

        if let Some(client) = &self.s3_client {
            let objects = client
                .lock()
                .await
                .list_objects(&bucket_name, "")
                .await
                .unwrap_or_else(|_| vec!["Error listing objects".to_string()]);

            self.base
                .results_navigator
                .set_title(String::from("Objects"));
            self.base
                .results_navigator
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

                self.base
                    .results_navigator
                    .set_title(format!("Path: {}", full_path));
                self.base
                    .results_navigator
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
                self.base
                    .event_sender
                    .send(Event::Tab(TabEvent::ComponentActions(
                        ComponentActions::S3ComponentActions(S3ComponentActions::LoadPath(
                            bucket.clone(),
                            self.current_path.clone(),
                        )),
                    )))
                    .unwrap();
            }
        }
    }
}

#[async_trait::async_trait]
impl AWSComponent for S3Component {
    fn render(&self, area: Rect, buf: &mut Buffer) {
        if !self.base.visible {
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
        self.base.navigator.render(horizontal_split[0], buf);
        self.base
            .results_navigator
            .render(right_vertical_split[1], buf);
        self.base.input.render(right_vertical_split[0], buf);

        if self.base.details_popup.is_visible() {
            self.base.details_popup.render(area, buf);
        }
    }

    /// Handles keyboard input for the S3 component
    fn handle_input(&mut self, key_event: KeyEvent) {
        // Special handling for popup details if visible
        if self.base.details_popup.is_visible() {
            if let Some(signal) = self.base.details_popup.handle_input(key_event) {
                self.base
                    .event_sender
                    .send(Event::Tab(TabEvent::ComponentActions(
                        ComponentActions::S3ComponentActions(S3ComponentActions::WidgetAction(
                            signal,
                        )),
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
                        ComponentActions::S3ComponentActions(S3ComponentActions::NextFocus),
                    )))
                    .unwrap();
            }
            KeyCode::BackTab => {
                self.base
                    .event_sender
                    .send(Event::Tab(TabEvent::ComponentActions(
                        ComponentActions::S3ComponentActions(S3ComponentActions::PreviousFocus),
                    )))
                    .unwrap();
            }
            KeyCode::Backspace => {
                // Navigate up one directory level
                if self.base.current_focus == ComponentFocus::Results {
                    self.base
                        .event_sender
                        .send(Event::Tab(TabEvent::ComponentActions(
                            ComponentActions::S3ComponentActions(S3ComponentActions::NavigateUp),
                        )))
                        .unwrap();
                }
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
                            ComponentActions::S3ComponentActions(
                                S3ComponentActions::WidgetAction(signal),
                            ),
                        )))
                        .unwrap();
                }
            }
        }
    }

    /// Processes S3-specific component actions
    async fn process_event(&mut self, event: ComponentActions) {
        match event {
            ComponentActions::S3ComponentActions(s3_event) => match s3_event {
                // Handle bucket selection
                S3ComponentActions::SelectBucket(bucket) => {
                    self.handle_bucket_selection(bucket).await;
                }
                // Navigate into a folder
                S3ComponentActions::NavigateFolder(path) => {
                    self.navigate_folder(path).await;
                }
                // Navigate up to parent directory
                S3ComponentActions::NavigateUp => {
                    self.navigate_up();
                }
                // Load contents at a specific path
                S3ComponentActions::LoadPath(bucket, path) => {
                    if let Some(client) = &self.s3_client {
                        let objects = client
                            .lock()
                            .await
                            .list_objects(&bucket, &path)
                            .await
                            .unwrap_or_else(|_| vec!["Error listing objects".to_string()]);

                        self.base.results_navigator.set_title(format!(
                            "Path: {}",
                            if path.is_empty() { "/" } else { &path }
                        ));
                        self.base
                            .results_navigator
                            .set_content(NavigatorContent::Records(objects));
                    }
                }
                // Display object details in popup
                S3ComponentActions::PopupDetails(key) => {
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
                                self.base
                                    .details_popup
                                    .set_content(PopupContent::Details(details));
                                self.base.details_popup.set_visible(true);
                                self.base.details_popup.set_active(true);
                            }
                            Err(_) => {
                                self.base
                                    .details_popup
                                    .set_content(PopupContent::Details(
                                        "Error fetching object details".to_string(),
                                    ));
                                self.base.details_popup.set_visible(true);
                                self.base.details_popup.set_active(true);
                            }
                        }
                    }
                }
                // Cycle focus forward through widgets
                S3ComponentActions::NextFocus => {
                    self.base.focus_next();
                    self.base.update_widget_states();
                }
                // Cycle focus backward through widgets
                S3ComponentActions::PreviousFocus => {
                    self.base.focus_previous();
                    self.base.update_widget_states();
                }
                // Process events from child widgets
                S3ComponentActions::WidgetAction(widget_action) => match widget_action {
                    WidgetAction::ServiceNavigatorEvent(
                        ref _aws_navigator_event,
                        widget_type,
                    ) => {
                        if widget_type == WidgetType::AWSServiceNavigator {
                            if let Some(signal) =
                                self.base.navigator.process_event(widget_action.clone())
                            {
                                match signal {
                                    // User selected a bucket from the navigator
                                    WidgetAction::ServiceNavigatorEvent(
                                        ServiceNavigatorEvent::ItemSelected(
                                            WidgetEventType::RecordSelected(bucket),
                                        ),
                                        WidgetType::AWSServiceNavigator,
                                    ) => {
                                        self.base
                                            .event_sender
                                            .send(Event::Tab(TabEvent::ComponentActions(
                                                ComponentActions::S3ComponentActions(
                                                    S3ComponentActions::SelectBucket(bucket),
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
                                            self.base
                                                .event_sender
                                                .send(Event::Tab(TabEvent::ComponentActions(
                                                    ComponentActions::S3ComponentActions(
                                                        S3ComponentActions::NavigateFolder(
                                                            folder_name,
                                                        ),
                                                    ),
                                                )))
                                                .unwrap();
                                        } else {
                                            // Show object details in popup
                                            self.base
                                                .event_sender
                                                .send(Event::Tab(TabEvent::ComponentActions(
                                                    ComponentActions::S3ComponentActions(
                                                        S3ComponentActions::PopupDetails(path),
                                                    ),
                                                )))
                                                .unwrap();
                                        }
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
                                // Handle search input when a bucket is selected
                                if let Some(bucket) = &self.selected_bucket {
                                    let search_path = if self.current_path.is_empty() {
                                        content.clone()
                                    } else {
                                        format!("{}/{}", self.current_path, content)
                                    };

                                    self.base
                                        .event_sender
                                        .send(Event::Tab(TabEvent::ComponentActions(
                                            ComponentActions::S3ComponentActions(
                                                S3ComponentActions::LoadPath(
                                                    bucket.clone(),
                                                    search_path,
                                                ),
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
                _ => {}
            },
            _ => {} // Ignore other component actions that don't belong to S3
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

    /// Fetches and displays the list of S3 buckets
    async fn update(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(client) = &self.s3_client {
            let client = client.lock().await;
            let buckets = client.list_buckets().await?;
            self.base
                .navigator
                .set_content(NavigatorContent::Records(buckets));

            // Reset results area
            self.base
                .results_navigator
                .set_content(NavigatorContent::Records(vec![]));
            self.base
                .results_navigator
                .set_title(String::from("Select a bucket"));
        }
        Ok(())
    }

    fn get_current_focus(&self) -> ComponentFocus {
        self.base.current_focus
    }

    fn reset_focus(&mut self) {
        self.base.current_focus = ComponentFocus::Navigation;
        self.base.update_widget_states();
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }

    fn set_focus_to_last(&mut self) {
        self.base.set_focus_to_last();
        self.base.update_widget_states();
    }
    
    fn get_help_items(&self) -> Vec<(String, String)> {
        // Return help items based on the base component's state
        self.base.get_help_items()
    }
}