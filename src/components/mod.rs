pub(crate) mod dynamodb;
pub(crate) mod tab;
pub(crate) mod aws_base_component;
pub(crate) mod s3;
use crate::event_managment::event::{ComponentActions, Event, WidgetActions};
use crossterm::event::KeyEvent;
use ratatui::{buffer::Buffer, layout::Rect};
use std::any::Any;
use tokio::sync::mpsc::UnboundedSender;

/// Common trait for all AWS service components
#[async_trait::async_trait]
pub trait AWSComponent: Send {
    /// Render the component to the buffer
    fn render(&self, area: Rect, buf: &mut Buffer);
    
    /// Handle keyboard input
    fn handle_input(&mut self, key_event: KeyEvent);
    
    /// Process component actions
    async fn process_event(&mut self, event: ComponentActions);
    
    /// Set active state
    fn set_active(&mut self, active: bool);
    
    /// Get active state
    fn is_active(&self) -> bool;
    
    /// Set visibility
    fn set_visible(&mut self, visible: bool);
    
    /// Get visibility
    fn is_visible(&self) -> bool;
    
    /// Update component data from the backend
    async fn update(&mut self) -> Result<(), Box<dyn std::error::Error>>;
    
    /// Get current focus state
    fn get_current_focus(&self) -> ComponentFocus;
    
    /// Reset focus to default state
    fn reset_focus(&mut self);
    
    /// Cast to Any for downcasting
    fn as_any_mut(&mut self) -> &mut dyn Any;

    fn set_focus_to_last(&mut self);
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ComponentFocus {
    Navigation,
    Input,
    Results,
    None,
}