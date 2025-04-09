use crate::components::ComponentFocus;
use crate::event_managment::event::Event;
use crate::widgets::WidgetExt;
use crate::widgets::aws_service_navigator::{AWSServiceNavigator, NavigatorContent};
use crate::widgets::input_box::InputBoxWidget;
use crate::widgets::popup::PopupWidget;
use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Direction, Layout, Rect},
};

/// Base component providing common functionality for all AWS service components
pub struct AWSComponentBase {
    /// Left navigator widget for service/bucket/table lists
    pub navigator: AWSServiceNavigator,
    /// Input widget for search/filter/query commands
    pub input: InputBoxWidget,
    /// Results area displaying query results or service content
    pub results_navigator: AWSServiceNavigator,
    /// Popup for displaying details and additional information
    pub details_popup: PopupWidget,
    /// Whether the component is currently active
    pub active: bool,
    /// Whether the component is currently visible
    pub visible: bool,
    /// Channel for sending events to the application
    pub event_sender: tokio::sync::mpsc::UnboundedSender<Event>,
    /// Current focus state within the component
    pub current_focus: ComponentFocus,
    /// Currently selected item (bucket, table, log group, etc.)
    pub selected_item: Option<String>,
    /// Current query string being executed
    pub selected_query: Option<String>,
}

impl AWSComponentBase {
    /// Creates a new base component with default widget configuration
    pub fn new(
        event_sender: tokio::sync::mpsc::UnboundedSender<Event>,
        navigator_content: NavigatorContent,
    ) -> Self {
        Self {
            navigator: AWSServiceNavigator::new(
                crate::event_managment::event::WidgetType::AWSServiceNavigator,
                false,
                navigator_content,
            ),
            input: InputBoxWidget::new("Query Input", false),
            results_navigator: AWSServiceNavigator::new(
                crate::event_managment::event::WidgetType::QueryResultsNavigator,
                false,
                NavigatorContent::Records(vec![]),
            ),
            details_popup: PopupWidget::new("Details", false, false),
            active: false,
            visible: true,
            event_sender,
            current_focus: ComponentFocus::Navigation,
            selected_item: None,
            selected_query: None,
        }
    }

    /// Renders the component with standard three-panel layout
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

    /// Updates active states of all widgets based on current focus
    pub fn update_widget_states(&mut self) {
        self.navigator
            .set_active(self.active & (self.current_focus == ComponentFocus::Navigation));
        self.input
            .set_active(self.active & (self.current_focus == ComponentFocus::Input));
        self.results_navigator
            .set_active(self.active & (self.current_focus == ComponentFocus::Results));
    }

    /// Shifts focus to the previous widget in the cyclic order
    pub fn focus_previous(&mut self) -> ComponentFocus {
        self.current_focus = match self.current_focus {
            ComponentFocus::Navigation => ComponentFocus::None,
            ComponentFocus::Input => ComponentFocus::Navigation,
            ComponentFocus::Results => ComponentFocus::Input,
            ComponentFocus::None => ComponentFocus::None,
        };
        self.current_focus
    }

    /// Shifts focus to the next widget in the cyclic order
    pub fn focus_next(&mut self) -> ComponentFocus {
        self.current_focus = match self.current_focus {
            ComponentFocus::Navigation => ComponentFocus::Input,
            ComponentFocus::Input => ComponentFocus::Results,
            ComponentFocus::Results => ComponentFocus::None,
            ComponentFocus::None => ComponentFocus::Navigation,
        };
        self.current_focus
    }

    /// Sets focus to the results area (typically the last widget in focus order)
    pub fn set_focus_to_last(&mut self) -> ComponentFocus {
        self.current_focus = ComponentFocus::Results;
        self.current_focus
    }

    /// Returns contextual help items based on current component state
    pub fn get_help_items(&self) -> Vec<(String, String)> {
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