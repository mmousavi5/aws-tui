use crate::widgets::input_box::InputBoxWidget;
use crate::widgets::aws_service_navigator::{AWSServiceNavigator, NavigatorContent};
use ratatui::{
    buffer::Buffer,
    layout::{Layout, Direction, Constraint, Rect},
    widgets::Widget,
};
use crate::event_managment::event::{Event, WidgetActions, ComponentActions, TabEvent, WidgetType};
use crate::widgets::WidgetExt;
use crossterm::event::{KeyCode, KeyEvent};

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
}

impl DynamoDB {
    pub fn new(event_sender: tokio::sync::mpsc::UnboundedSender<Event>) -> Self {
        Self {
            table_list_navigator: AWSServiceNavigator::new(
                WidgetType::AWSServiceNavigator,
                false,
                event_sender.clone(),
                NavigatorContent::Records(vec!["DynamoDB".to_string(), "S3".to_string(), "Lambda".to_string()]),
            ),
            query_input: InputBoxWidget::new("Query Input", false),
            query_results_navigator: AWSServiceNavigator::new(
                WidgetType::QueryResultsNavigator,
                false,
                event_sender.clone(),
                NavigatorContent::Records(vec!["q1".to_string(), "q2".to_string(), "q3".to_string()]),
            ),
            active: false,
            visible: true,
            event_sender,
            current_focus: ComponentFocus::Navigation,
        }
    }

    pub fn render(&self, area: Rect, buf: &mut Buffer) {
        if !self.visible {
            return;
        }

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
        self.query_results_navigator.render(right_vertical_split[1], buf);
    }

    pub fn handle_input(&mut self, key_event: crossterm::event::KeyEvent) {
        match key_event.code {
            KeyCode::Char('t') => {
                self.event_sender
                    .send(Event::Tab(TabEvent::ComponentActions(ComponentActions::NextFocus)))
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
                        .send(Event::Tab(TabEvent::ComponentActions(ComponentActions::WidgetActions(signal))))
                        .unwrap();
                }
            }
        }
    }

    fn update_widget_states(&mut self) {
        self.table_list_navigator.set_active(self.active & (self.current_focus == ComponentFocus::Navigation));
        self.query_input.set_active(self.active & (self.current_focus == ComponentFocus::Input));
        self.query_results_navigator.set_active(self.active &(self.current_focus == ComponentFocus::Results));
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

    pub fn process_event(&mut self, event: ComponentActions) {
        match event {
            ComponentActions::ArrowDown => {
            }
            ComponentActions::ArrowUp => {
            }
            ComponentActions::NextFocus => {
                self.focus_next();
                self.update_widget_states();

            }
            ComponentActions::WidgetActions(widget_action) => {
                match widget_action {
                    WidgetActions::AWSServiceNavigatorEvent(ref _aws_navigator_event, widget_type) => {
                        if widget_type == WidgetType::AWSServiceNavigator {
                            self.table_list_navigator.process_event(widget_action.clone());
                        } else if widget_type == WidgetType::QueryResultsNavigator {
                            self.query_results_navigator.process_event(widget_action.clone());
                        }
                    }
                    WidgetActions::InputBoxEvent(ref _input_box_event) => {
                        self.query_input.process_event(widget_action);
                    }
                    _ => {}
                }
            }
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

    pub fn focus_next(&mut self) -> ComponentFocus {
        self.current_focus = match self.current_focus {
            ComponentFocus::Navigation => ComponentFocus::Input,
            ComponentFocus::Input => ComponentFocus::Results,
            ComponentFocus::Results => ComponentFocus::None,
            ComponentFocus::None => ComponentFocus::Navigation,
        };
        self.current_focus
    }
}