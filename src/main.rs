use crate::app::App;

pub mod app;
pub mod event_managment;
pub mod ui;
pub mod components;
pub mod widgets;

#[tokio::main]
async fn main() -> color_eyre::Result<()> {
    color_eyre::install()?;
    let terminal = ratatui::init();
    let result = App::new().run(terminal).await;
    ratatui::restore();
    result
}
