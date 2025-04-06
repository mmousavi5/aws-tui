use crate::components::tab::Tab;
use crate::event_managment::event::TabEvent;
use crate::event_managment::event::{AppEvent, Event, EventHandler};
use ratatui::{
    DefaultTerminal,
    crossterm::event::{KeyCode, KeyEvent, KeyModifiers},
};

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
                Event::App(app_event) => {
                    self.apply_app_state(app_event);
                }
                Event::Tab(tab_event) => {
                    self.apply_tab_state(tab_event).await;
                }
            }
        }
        Ok(())
    }

    /// Handles the key events and updates the state of [`App`].
    pub fn handle_key_events(&mut self, key_event: KeyEvent) -> color_eyre::Result<()> {
        match key_event.code {
            // Mac-style shortcuts (Command/⌘ is mapped to CONTROL in terminal apps)
            KeyCode::Char('w') if key_event.modifiers == KeyModifiers::CONTROL => {
                self.events.send(Event::App(AppEvent::CloseTab)) // ⌘+W to close tab
            }
            KeyCode::Char('t') if key_event.modifiers == KeyModifiers::CONTROL => {
                self.events.send(Event::App(AppEvent::CreateTab)) // ⌘+T for new tab
            }
            KeyCode::Tab if key_event.modifiers == KeyModifiers::CONTROL => {
                self.events.send(Event::App(AppEvent::NextTab)) // ⌘+Tab to switch tabs
            }
            KeyCode::Tab if key_event.modifiers == KeyModifiers::CONTROL | KeyModifiers::SHIFT => {
                self.events.send(Event::App(AppEvent::PreviousTab)) // ⌘+Shift+Tab to switch tabs backwards
            }
            KeyCode::Char('q') if key_event.modifiers == KeyModifiers::CONTROL => {
                self.events.send(Event::App(AppEvent::Quit)) // ⌘+Q to quit
            }
            _ => {
                if let Some(tab) = self.tabs.get_mut(self.active_tab) {
                    tab.handle_input(key_event);
                }
            }
        }
        Ok(())
    }

    pub fn apply_app_state(&mut self, app_state: AppEvent) {
        match app_state {
            AppEvent::NextTab => self.next_tab(),
            AppEvent::CreateTab => {
                self.tabs.push(Tab::new(
                    "New Tab",
                    "This is a new tab.",
                    self.events.sender.clone(),
                ));
            }
            AppEvent::CloseTab => {
                if self.tabs.len() > 1 {
                    self.tabs.remove(self.active_tab);
                    self.active_tab = self.active_tab.saturating_sub(1);
                }
            }
            AppEvent::Quit => self.quit(),
            _ => {}
        }
    }

    pub async fn apply_tab_state(&mut self, tab_event: TabEvent) {
        // match tab_state {
        //     TabEvent::TabActions(TabActions::ProfileSelected(profile)) => self.set_active_tab_name(&profile),
        //     _ => {}
        // }
        if let Some(tab) = self.tabs.get_mut(self.active_tab) {
            tab.process_event(tab_event).await;
        }
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

    /// Switch to the next tab.
    pub fn next_tab(&mut self) {
        // self.tabs[self.active_tab].show_popup = false;
        self.active_tab = (self.active_tab + 1) % self.tabs.len();
    }
}
