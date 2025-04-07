use crate::components::{AWSComponent, ComponentFocus};
use crate::event_managment::event::{ComponentActions, Event, WidgetActions};
use crate::widgets::WidgetExt;
use crate::widgets::aws_service_navigator::{AWSServiceNavigator, NavigatorContent};
use crate::widgets::input_box::InputBoxWidget;
use crate::widgets::popup::{PopupContent, PopupWidget};
use crossterm::event::KeyEvent;
use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Direction, Layout, Rect},
};
use std::any::Any;

pub struct AWSComponentBase {
    pub navigator: AWSServiceNavigator,
    pub input: InputBoxWidget,
    pub results_navigator: AWSServiceNavigator,
    pub details_popup: PopupWidget,
    pub active: bool,
    pub visible: bool,
    pub event_sender: tokio::sync::mpsc::UnboundedSender<Event>,
    pub current_focus: ComponentFocus,
    pub selected_item: Option<String>,
    pub selected_query: Option<String>,
}

impl AWSComponentBase {
    pub fn new(
        event_sender: tokio::sync::mpsc::UnboundedSender<Event>,
        navigator_content: NavigatorContent,
    ) -> Self {
        Self {
            navigator: AWSServiceNavigator::new(
                crate::event_managment::event::WidgetType::AWSServiceNavigator,
                false,
                event_sender.clone(),
                navigator_content,
            ),
            input: InputBoxWidget::new("Query Input", false, event_sender.clone()),
            results_navigator: AWSServiceNavigator::new(
                crate::event_managment::event::WidgetType::QueryResultsNavigator,
                false,
                event_sender.clone(),
                NavigatorContent::Records(vec![]),
            ),
            details_popup: PopupWidget::new("Details", false, false, event_sender.clone()),
            active: false,
            visible: true,
            event_sender,
            current_focus: ComponentFocus::Navigation,
            selected_item: None,
            selected_query: None,
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
                Constraint::Percentage(30), // Left panel - navigator list
                Constraint::Percentage(70), // Right panel
            ])
            .split(area);

        // Create a vertical split for the right panel
        let right_vertical_split = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Percentage(15), // Input box
                Constraint::Percentage(85), // Results
            ])
            .split(horizontal_split[1]);

        // Render components
        self.navigator.render(horizontal_split[0], buf);
        self.input.render(right_vertical_split[0], buf);
        self.results_navigator.render(right_vertical_split[1], buf);

        if self.details_popup.is_visible() {
            self.details_popup.render(area, buf);
        }
    }

    pub fn update_widget_states(&mut self) {
        self.navigator
            .set_active(self.active & (self.current_focus == ComponentFocus::Navigation));
        self.input
            .set_active(self.active & (self.current_focus == ComponentFocus::Input));
        self.results_navigator
            .set_active(self.active & (self.current_focus == ComponentFocus::Results));
    }

    pub fn focus_previous(&mut self) -> ComponentFocus {
        self.current_focus = match self.current_focus {
            ComponentFocus::Navigation => ComponentFocus::None,
            ComponentFocus::Input => ComponentFocus::Navigation,
            ComponentFocus::Results => ComponentFocus::Input,
            ComponentFocus::None => ComponentFocus::None,
        };
        self.current_focus
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

    pub fn set_focus_to_last(&mut self) -> ComponentFocus {
        self.current_focus = ComponentFocus::Results;
        self.current_focus
    }
}
