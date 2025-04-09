//! UI rendering module
//!
//! Implements ratatui Widget trait for the App struct,
//! enabling the application to be rendered to the terminal.

use crate::app::App;
use ratatui::{buffer::Buffer, layout::Rect, widgets::Widget};

/// Implements the Widget trait for App references
///
/// This enables the App to be directly rendered as a Widget,
/// creating a clean interface for the main rendering loop
impl Widget for &App {
    /// Renders the entire application UI
    ///
    /// Collects tab names and delegates rendering to the active tab
    fn render(self, area: Rect, buf: &mut Buffer) {
        // Collect the names of all tabs for the tab bar
        let all_tabs_names = self
            .tabs
            .iter()
            .map(|t| t.name.to_string())
            .collect::<Vec<String>>();
            
        // Render the currently active tab with the full area
        if let Some(active_tab) = self.tabs.get(self.active_tab) {
            active_tab.render(area, buf, all_tabs_names, self.active_tab);
        }
    }
}