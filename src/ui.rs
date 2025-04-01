use ratatui::{
    buffer::Buffer,
    layout::{Alignment, Rect},
    widgets::{Block, BorderType, Widget},
};
use crate::app::App;

impl Widget for &App {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let block = Block::bordered()
            .title("rat-tab-test")
            .title_alignment(Alignment::Center)
            .border_type(BorderType::Rounded);

        let all_tabs_names = self.tabs.iter().map(|t| t.name.to_string()).collect::<Vec<String>>();
        // Render Tabs
        if let Some(active_tab) = self.tabs.get(self.active_tab) {
            active_tab.render(area, buf, all_tabs_names, self.active_tab);
        }
    }
}