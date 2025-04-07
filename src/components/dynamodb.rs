use crate::components::aws_base_component::AWSComponentBase;
use crate::components::{AWSComponent, ComponentFocus};
use crate::event_managment::event::{
    AWSServiceNavigatorEvent, ComponentActions, DynamoDBComponentActions, Event, InputBoxEvent,
    TabEvent, WidgetActions, WidgetEventType, WidgetType,
};
use crate::services::aws::dynamo_client::DynamoDBClient;
use crate::widgets::WidgetExt;
use crate::widgets::aws_service_navigator::NavigatorContent;
use crate::widgets::popup::PopupContent;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::{buffer::Buffer, layout::Rect};
use std::any::Any;
use std::sync::Arc;
use tokio::sync::Mutex;

pub struct DynamoDB {
    base: AWSComponentBase,
    dynamodb_client: Option<Arc<Mutex<DynamoDBClient>>>,
}

impl DynamoDB {
    pub fn new(event_sender: tokio::sync::mpsc::UnboundedSender<Event>) -> Self {
        Self {
            base: AWSComponentBase::new(event_sender.clone(), NavigatorContent::Records(vec![])),
            dynamodb_client: None,
        }
    }

    pub fn set_client(&mut self, dynamodb_client: Arc<Mutex<DynamoDBClient>>) {
        self.dynamodb_client = Some(dynamodb_client);
    }
}

#[async_trait::async_trait]
impl AWSComponent for DynamoDB {
    fn render(&self, area: Rect, buf: &mut Buffer) {
        self.base.render(area, buf);
    }

    fn set_focus_to_last(&mut self) {
        self.base.set_focus_to_last();
        self.base.update_widget_states();
    }

    fn handle_input(&mut self, key_event: KeyEvent) {
        if self.base.details_popup.is_visible() {
            if let Some(signal) = self.base.details_popup.handle_input(key_event) {
                self.base
                    .event_sender
                    .send(Event::Tab(TabEvent::ComponentActions(
                        ComponentActions::DynamoDBComponentActions(
                            DynamoDBComponentActions::WidgetActions(signal),
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
                        ComponentActions::DynamoDBComponentActions(
                            DynamoDBComponentActions::NextFocus,
                        ),
                    )))
                    .unwrap();
            }
            KeyCode::BackTab => {
                self.base
                    .event_sender
                    .send(Event::Tab(TabEvent::ComponentActions(
                        ComponentActions::DynamoDBComponentActions(
                            DynamoDBComponentActions::PreviousFocus,
                        ),
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
                    self.base
                        .event_sender
                        .send(Event::Tab(TabEvent::ComponentActions(
                            ComponentActions::DynamoDBComponentActions(
                                DynamoDBComponentActions::WidgetActions(signal),
                            ),
                        )))
                        .unwrap();
                }
            }
        }
    }

    async fn process_event(&mut self, event: ComponentActions) {
        match event {
            ComponentActions::DynamoDBComponentActions(DynamoDBComponentActions::SetTitle(
                title,
            )) => {
                self.base.navigator.set_title(title.clone());
                self.base.selected_item = Some(title);
            }
            ComponentActions::DynamoDBComponentActions(DynamoDBComponentActions::SetQuery(
                query,
            )) => {
                self.base.results_navigator.set_title(query.clone());
                self.base.selected_query = Some(query.clone());

                if let Some(client) = &self.dynamodb_client {
                    if let Some(selected_table) = &self.base.selected_item {
                        let content = client
                            .lock()
                            .await
                            .query_table(selected_table.clone(), query.clone())
                            .await
                            .unwrap_or_else(|_| vec!["Query error".to_string()]);

                        self.base
                            .results_navigator
                            .set_content(NavigatorContent::Records(content));
                    }
                }
            }
            ComponentActions::DynamoDBComponentActions(DynamoDBComponentActions::PopupDetails(
                title,
            )) => {
                self.base
                    .details_popup
                    .set_profile_list(PopupContent::Details(title.clone()));
                self.base.details_popup.set_visible(true);
                self.base.details_popup.set_active(true);
            }
            ComponentActions::DynamoDBComponentActions(DynamoDBComponentActions::NextFocus) => {
                self.base.focus_next();
                self.base.update_widget_states();
            }
            ComponentActions::DynamoDBComponentActions(
                DynamoDBComponentActions::WidgetActions(widget_action),
            ) => match widget_action {
                WidgetActions::AWSServiceNavigatorEvent(ref _aws_navigator_event, widget_type) => {
                    if widget_type == WidgetType::AWSServiceNavigator {
                        if let Some(signal) =
                            self.base.navigator.process_event(widget_action.clone())
                        {
                            match signal {
                                WidgetActions::AWSServiceNavigatorEvent(
                                    AWSServiceNavigatorEvent::SelectedItem(
                                        WidgetEventType::RecordSelected(title),
                                    ),
                                    WidgetType::AWSServiceNavigator,
                                ) => {
                                    self.base
                                        .event_sender
                                        .send(Event::Tab(TabEvent::ComponentActions(
                                            ComponentActions::DynamoDBComponentActions(
                                                DynamoDBComponentActions::SetTitle(title.clone()),
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
                                WidgetActions::AWSServiceNavigatorEvent(
                                    AWSServiceNavigatorEvent::SelectedItem(
                                        WidgetEventType::RecordSelected(title),
                                    ),
                                    WidgetType::QueryResultsNavigator,
                                ) => {
                                    self.base
                                        .event_sender
                                        .send(Event::Tab(TabEvent::ComponentActions(
                                            ComponentActions::DynamoDBComponentActions(
                                                DynamoDBComponentActions::PopupDetails(
                                                    title.clone(),
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
                WidgetActions::InputBoxEvent(ref _input_box_event) => {
                    if let Some(signal) = self.base.input.process_event(widget_action) {
                        match signal {
                            WidgetActions::InputBoxEvent(InputBoxEvent::Written(content)) => {
                                self.base
                                    .event_sender
                                    .send(Event::Tab(TabEvent::ComponentActions(
                                        ComponentActions::DynamoDBComponentActions(
                                            DynamoDBComponentActions::SetQuery(content),
                                        ),
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

    fn reset_focus(&mut self) {
        self.base.current_focus = ComponentFocus::Navigation;
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}
