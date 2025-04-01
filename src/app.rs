use crate::event_managment::event::{AppEvent, Event, EventHandler};
use crossterm::event;
use ratatui::{
    DefaultTerminal,
    crossterm::event::{KeyCode, KeyEvent, KeyModifiers},
};
use crate::components::tab::Tab;
use crate::event_managment::event::WidgetEventType;


/// Application.
pub struct App {
    /// Is the application running?
    pub running: bool,
    /// Counter.
    pub counter: u8,
    /// Event handler.
    pub events: EventHandler,
    ///
    pub active_tab: usize, // Track the active tab
    ///
    pub tabs: Vec<Tab>,

}

impl Default for App {
    fn default() -> Self {
        let events = EventHandler::new();
        Self {
            running: true,
            counter: 0,
            tabs: vec![
                Tab::new("Tab 1", "This is Tab 1.", events.sender.clone()),
                Tab::new("Tab 2", "This is Tab 2.", events.sender.clone()),
                Tab::new("Tab 3", "This is Tab 3.", events.sender.clone()),
            ],
            events,
            active_tab: 0,
        }
    }
}

impl App {
    /// Constructs a new instance of [`App`].
    pub fn new() -> Self {
        Self::default()
    }

    /// Run the application's main loop.
    pub async fn run(mut self, mut terminal: DefaultTerminal) -> color_eyre::Result<()> {
        while self.running {
            terminal.draw(|frame| frame.render_widget(&self, frame.area()))?;
            match self.events.next().await? {
                Event::Tick => self.tick(),
                Event::Crossterm(event) => match event {
                    crossterm::event::Event::Key(key_event) => self.handle_key_events(key_event)?,
                    _ => {}
                },
                Event::App(app_event) => match app_event {
                    AppEvent::Increment => self.increment_counter(),
                    AppEvent::Decrement => self.decrement_counter(),
                    AppEvent::NextTab => self.next_tab(),
                    AppEvent::Quit => self.quit(),
                },
                Event::ActiveTabKey(key_event) => {
                    self.route_event(key_event);
                }
                Event::AWSProfileEvent(profile) => self.set_active_tab_name(&profile),
                Event::WidgetEvent(event) => {
                    // Handle widget events here
                    match event {
                        WidgetEventType::S3 => {
                            self.update_sub_widgets(WidgetEventType::S3);
                            // Handle S3 event
                        }
                        WidgetEventType::DynamoDB => {
                            self.update_sub_widgets(WidgetEventType::DynamoDB);
                            // Handle DynamoDB event
                        }
                        _ => {
                            // Handle other events
                        }
                    }
                }
            }
        }
        Ok(())
    }

    /// Handles the key events and updates the state of [`App`].
    pub fn handle_key_events(&mut self, key_event: KeyEvent) -> color_eyre::Result<()> {
        match key_event.code {
            KeyCode::Esc | KeyCode::Char('q') => self.events.send(Event::App(AppEvent::Quit)),
            KeyCode::Char('c' | 'C') if key_event.modifiers == KeyModifiers::CONTROL => {
                self.events.send(Event::App(AppEvent::Quit))
            }
            KeyCode::Right => self.events.send(Event::App(AppEvent::Increment)),
            KeyCode::Left => self.events.send(Event::App(AppEvent::Decrement)),
            KeyCode::Tab => self.events.send(Event::App(AppEvent::NextTab)),
            // Other handlers you could add here.
            _ => {self.events.send(Event::ActiveTabKey(key_event));},
        }
        Ok(())
    }

    /// Handles the tick event of the terminal.
    ///
    /// The tick event is where you can update the state of your application with any logic that
    /// needs to be updated at a fixed frame rate. E.g. polling a server, updating an animation.
    pub fn tick(&self) {}

    /// Set running to false to quit the application.
    pub fn quit(&mut self) {
        self.running = false;
    }

    pub fn increment_counter(&mut self) {
        self.counter = self.counter.saturating_add(1);
    }

    pub fn decrement_counter(&mut self) {
        self.counter = self.counter.saturating_sub(1);
    }

    /// Switch to the next tab.
    pub fn next_tab(&mut self) {
        // self.tabs[self.active_tab].show_popup = false;
        self.active_tab = (self.active_tab + 1) % self.tabs.len();
    }

    /// Route the event to the active tab.
    pub fn route_event(&mut self, key_event: KeyEvent) {
        if let Some(tab) = self.tabs.get_mut(self.active_tab) {
            tab.handle_input(key_event);
        }
    }

    pub fn set_active_tab_name(&mut self, name: &str) {
        if let Some(tab) = self.tabs.get_mut(self.active_tab) {
            tab.set_name(name.to_string());
        }
    }

    pub fn update_sub_widgets(&mut self, event_type: WidgetEventType) {
        if let Some(tab) = self.tabs.get_mut(self.active_tab) {
            tab.update_sub_widgets(event_type);
        }
    }
}
