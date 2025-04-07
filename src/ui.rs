use crate::app::App;
use ratatui::{buffer::Buffer, layout::Rect, widgets::Widget};

impl Widget for &App {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let all_tabs_names = self
            .tabs
            .iter()
            .map(|t| t.name.to_string())
            .collect::<Vec<String>>();
        // Render Tabs
        if let Some(active_tab) = self.tabs.get(self.active_tab) {
            active_tab.render(area, buf, all_tabs_names, self.active_tab);
        }
    }
}
