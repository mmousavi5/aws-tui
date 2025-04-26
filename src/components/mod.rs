pub(crate) mod cloudwatch;
pub(crate) mod dynamodb;
pub(crate) mod s3;
pub(crate) mod tab;
use crate::event_managment::event::ComponentAction;
use crossterm::event::KeyEvent;
use ratatui::{buffer::Buffer, layout::Rect};
use std::any::Any;

/// Common trait for all AWS service components
#[async_trait::async_trait]
pub trait AWSComponent: Send {
    /// Render the component to the buffer
    fn render(&self, area: Rect, buf: &mut Buffer);

    /// Handle keyboard input
    fn handle_input(&mut self, key_event: KeyEvent);

    /// Process component actions
    async fn process_event(&mut self, event: ComponentAction);

    /// Set active state
    fn set_active(&mut self, active: bool);

    /// Get active state
    fn is_active(&self) -> bool;

    /// Set visibility``
    fn set_visible(&mut self, visible: bool);

    /// Get visibility
    fn is_visible(&self) -> bool;

    /// Update component data from the backend
    async fn update(&mut self) -> Result<(), Box<dyn std::error::Error>>;

    /// Reset focus to default state
    fn reset_focus(&mut self);

    /// Cast to Any for downcasting
    fn as_any_mut(&mut self) -> &mut dyn Any;

    /// Restore focus to the last active widget
    fn set_focus_to_last(&mut self);

    /// Get contextual help information for the component
    fn get_help_items(&self) -> Vec<(String, String)>;

    /// Is the component navigable
    fn allows_focus_continuation(&self) -> bool;

    /// Is the component navigable
    fn allows_focus_continuation_backward(&self) -> bool;
}

/// Represents the current input focus within a component
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ComponentFocus {
    /// Focus on the left navigation area (service list/tables/buckets)
    Navigation,
    /// Focus on the input area (search/filter/command box)
    Input,
    /// Focus on the time range input box
    TimeRange,
    /// Focus on the results display area
    Results,
    /// No focus set
    None,
}
