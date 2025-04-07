use crate::components::{AWSComponent, ComponentFocus};
use crate::components::aws_base_component::AWSComponentBase;
use crate::event_managment::event::{AWSServiceNavigatorEvent, ComponentActions, Event, InputBoxEvent, PopupEvent, S3ComponentActions, TabEvent, WidgetActions, WidgetEventType, WidgetType};
use crate::services::aws::s3_client::S3Client;
use crate::widgets::aws_service_navigator::NavigatorContent;
use crate::widgets::popup::PopupContent;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::{buffer::Buffer, layout::Rect};
use std::any::Any;
use std::sync::Arc;
use tokio::sync::Mutex;
use crate::widgets::WidgetExt;

pub struct S3Component {
    base: AWSComponentBase,
    s3_client: Option<Arc<Mutex<S3Client>>>,
}

impl S3Component {
    pub fn new(event_sender: tokio::sync::mpsc::UnboundedSender<Event>) -> Self {
        Self {
            base: AWSComponentBase::new(
                event_sender.clone(), 
                NavigatorContent::Records(vec![]),
            ),
            s3_client: None,
        }
    }

    pub fn set_client(&mut self, s3_client: Arc<Mutex<S3Client>>) {
        self.s3_client = Some(s3_client);
    }
}

#[async_trait::async_trait]
impl AWSComponent for S3Component {
    fn render(&self, area: Rect, buf: &mut Buffer) {
        self.base.render(area, buf);
    }

    fn handle_input(&mut self, key_event: KeyEvent) {
        if self.base.details_popup.is_visible() {
            if let Some(signal) = self.base.details_popup.handle_input(key_event) {
                self.base.event_sender
                    .send(Event::Tab(TabEvent::ComponentActions(
                        ComponentActions::S3ComponentActions(S3ComponentActions::WidgetActions(signal))
                    )))
                    .unwrap();
                return;
            }
        }

        match key_event.code {
            KeyCode::Tab => {
                self.base.event_sender
                    .send(Event::Tab(TabEvent::ComponentActions(
                        ComponentActions::S3ComponentActions(S3ComponentActions::NextFocus),
                    )))
                    .unwrap();
            }
            KeyCode::BackTab => {
                self.base.event_sender
                    .send(Event::Tab(TabEvent::ComponentActions(
                        ComponentActions::S3ComponentActions(S3ComponentActions::PreviousFocus),
                    )))
                    .unwrap();
            }
            KeyCode::Char('1') if key_event.modifiers == KeyModifiers::ALT => {
                self.base.current_focus = ComponentFocus::Navigation;
                self.base.update_widget_states();
            }
            KeyCode::Char('2') if key_event.modifiers == KeyModifiers::ALT => {
                self.base.current_focus = ComponentFocus::Input;
                self.base.update_widget_states();
            }
            KeyCode::Char('3') if key_event.modifiers == KeyModifiers::ALT => {
                self.base.current_focus = ComponentFocus::Results;
                self.base.update_widget_states();
            }
            KeyCode::Esc => {
                if self.base.current_focus != ComponentFocus::Navigation {
                    self.base.current_focus = ComponentFocus::Navigation;
                    self.base.update_widget_states();
                }
            }
            _ => {
                if let Some(signal) = match self.base.current_focus {
                    ComponentFocus::Navigation => self.base.navigator.handle_input(key_event),
                    ComponentFocus::Input => self.base.input.handle_input(key_event),
                    ComponentFocus::Results => self.base.results_navigator.handle_input(key_event),
                    ComponentFocus::None => None,
                } {
                    self.base.event_sender
                        .send(Event::Tab(TabEvent::ComponentActions(
                            ComponentActions::S3ComponentActions(S3ComponentActions::WidgetActions(signal)),
                        )))
                        .unwrap();
                }
            }
        }
    }

    async fn process_event(&mut self, event: ComponentActions) {
        match event {
            ComponentActions::S3ComponentActions(s3_event) => match s3_event {
                S3ComponentActions::SetTitle(title) => {
                    self.base.navigator.set_title(title.clone());
                    self.base.selected_item = Some(title);
                }
                S3ComponentActions::SetQuery(query) => {
                    self.base.results_navigator.set_title(query.clone());
                    self.base.selected_query = Some(query.clone());
                    
                    if let Some(client) = &self.s3_client {
                        if let Some(selected_bucket) = &self.base.selected_item {
                            let content = client
                                .lock()
                                .await
                                .list_objects(selected_bucket, &query)
                                .await
                                .unwrap_or_else(|_| vec!["Query error".to_string()]);
                                
                            self.base.results_navigator.set_content(NavigatorContent::Records(content));
                        }
                    }
                }
                S3ComponentActions::PopupDetails(title) => {
                    self.base.details_popup.set_profile_list(PopupContent::Details(title.clone()));
                    self.base.details_popup.set_visible(true);
                    self.base.details_popup.set_active(true);
                }
                S3ComponentActions::NextFocus => {
                    self.base.focus_next();
                    self.base.update_widget_states();
                }
                S3ComponentActions::PreviousFocus => {
                    self.base.focus_previous();
                    self.base.update_widget_states();
                }
                S3ComponentActions::WidgetActions(widget_action) => match widget_action {
                    WidgetActions::AWSServiceNavigatorEvent(ref _aws_navigator_event, widget_type) => {
                        if widget_type == WidgetType::AWSServiceNavigator {
                            if let Some(signal) = self.base.navigator.process_event(widget_action.clone()) {
                                match signal {
                                    WidgetActions::AWSServiceNavigatorEvent(
                                        AWSServiceNavigatorEvent::SelectedItem(
                                            WidgetEventType::RecordSelected(title),
                                        ),
                                        WidgetType::AWSServiceNavigator,
                                    ) => {
                                        self.base.event_sender
                                            .send(Event::Tab(TabEvent::ComponentActions(
                                                ComponentActions::S3ComponentActions(S3ComponentActions::SetTitle(title.clone())),
                                            )))
                                            .unwrap();
                                    }
                                    _ => {}
                                }
                            }
                        } else if widget_type == WidgetType::QueryResultsNavigator {
                            if let Some(signal) = self.base.results_navigator.process_event(widget_action.clone()) {
                                match signal {
                                    WidgetActions::AWSServiceNavigatorEvent(
                                        AWSServiceNavigatorEvent::SelectedItem(
                                            WidgetEventType::RecordSelected(title),
                                        ),
                                        WidgetType::QueryResultsNavigator,
                                    ) => {
                                        self.base.event_sender
                                            .send(Event::Tab(TabEvent::ComponentActions(
                                                ComponentActions::S3ComponentActions(S3ComponentActions::PopupDetails(title.clone())),
                                            )))
                                            .unwrap();
                                    }
                                    _ => {}
                                }
                            }
                        }
                    }
                    WidgetActions::InputBoxEvent(ref _input_box_event) => {
                        if let Some(signal) = self.base.input.process_event(widget_action) {
                            match signal {
                                WidgetActions::InputBoxEvent(InputBoxEvent::Written(content)) => {
                                    self.base.event_sender
                                        .send(Event::Tab(TabEvent::ComponentActions(
                                            ComponentActions::S3ComponentActions(S3ComponentActions::SetQuery(content)),
                                        )))
                                        .unwrap();
                                }
                                _ => {}
                            }
                        }
                    }
                    WidgetActions::PopupEvent(_) => {
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

    async fn update(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(client) = &self.s3_client {
            let client = client.lock().await;
            let buckets = client.list_buckets().await?;
            self.base.navigator.set_content(NavigatorContent::Records(buckets));
        }
        Ok(())
    }

    fn get_current_focus(&self) -> ComponentFocus {
        self.base.current_focus
    }

    fn reset_focus(&mut self) {
        self.base.current_focus = ComponentFocus::Navigation;
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
    fn set_focus_to_last(&mut self) {
        self.base.set_focus_to_last();
        self.base.update_widget_states();
    }

}