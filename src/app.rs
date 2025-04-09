//! Application module
//!
//! Provides the main application state and event loop functionality.
//! Manages tabs, event handling, and the core application lifecycle.

use crate::components::tab::Tab;
use crate::event_managment::event::TabEvent;
use crate::event_managment::event::{AppEvent, Event, EventHandler};
use ratatui::{
    DefaultTerminal,
    crossterm::event::{KeyCode, KeyEvent, KeyModifiers},
};

/// Main application state container
///
/// Manages application lifecycle, tab collection and event flow
pub struct App {
    /// Whether the application is still running
    pub running: bool,
    /// Generic counter for application state
    pub counter: u8,
    /// Event handler for processing UI and system events
    pub events: EventHandler,
    /// Index of the currently active tab
    pub active_tab: usize,
    /// Collection of all tabs in the application
    pub tabs: Vec<Tab>,
}

impl Default for App {
    /// Creates a default application state with initial tabs
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
    /// Constructs a new instance of [`App`]
    pub fn new() -> Self {
        Self::default()
    }

    /// Run the application's main event loop
    ///
    /// Processes events and updates the terminal UI until the application exits
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

    /// Processes keyboard events and routes them to appropriate handlers
    ///
    /// Handles global shortcuts and routes other keypresses to the active tab
    pub fn handle_key_events(&mut self, key_event: KeyEvent) -> color_eyre::Result<()> {
        match key_event.code {
            // Mac-style shortcuts (Command/⌘ is mapped to CONTROL in terminal apps)
            KeyCode::Char('w') if key_event.modifiers == KeyModifiers::CONTROL => {
                self.events.send(Event::App(AppEvent::CloseTab)) // ⌘+W to close tab
            }
            KeyCode::Char('t') if key_event.modifiers == KeyModifiers::CONTROL => {
                self.events.send(Event::App(AppEvent::CreateTab)) // ⌘+T for new tab
            }
            KeyCode::Char('l') if key_event.modifiers == KeyModifiers::CONTROL => {
                self.events.send(Event::App(AppEvent::NextTab)) // ⌘+Tab to switch tabs
            }
            KeyCode::Char('j') if key_event.modifiers == KeyModifiers::CONTROL => {
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

    /// Updates application state based on application events
    ///
    /// Handles tab switching, creation, closure and application exit
    pub fn apply_app_state(&mut self, app_state: AppEvent) {
        match app_state {
            AppEvent::NextTab => self.next_tab(),
            AppEvent::PreviousTab => self.previous_tab(),
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

    /// Routes tab events to the currently active tab
    ///
    /// Used for handling events targeted at specific tabs like profile selection
    pub async fn apply_tab_state(&mut self, tab_event: TabEvent) {
        // match tab_state {
        //     TabEvent::TabActions(TabActions::ProfileSelected(profile)) => self.set_active_tab_name(&profile),
        //     _ => {}
        // }
        if let Some(tab) = self.tabs.get_mut(self.active_tab) {
            tab.process_event(tab_event).await;
        }
    }

    /// Handles the tick event of the terminal
    ///
    /// Called at a fixed frame rate to update animations or poll external systems
    pub fn tick(&self) {}

    /// Terminates the application by setting running to false
    pub fn quit(&mut self) {
        self.running = false;
    }

    /// Cycles to the next tab in the tab collection
    pub fn next_tab(&mut self) {
        // self.tabs[self.active_tab].show_popup = false;
        self.active_tab = (self.active_tab + 1) % self.tabs.len();
    }
    pub fn previous_tab(&mut self) {
        // self.tabs[self.active_tab].show_popup = false;
        if self.active_tab == 0 {
            self.active_tab = self.tabs.len() - 1;
        } else {
            self.active_tab -= 1;
        }
    }
}
