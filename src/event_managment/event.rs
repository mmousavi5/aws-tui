use color_eyre::eyre::OptionExt;
use futures::{FutureExt, StreamExt};
use ratatui::crossterm::event::Event as CrosstermEvent;
use ratatui::crossterm::event::KeyEvent;
use std::time::Duration;
use tokio::sync::mpsc;
/// The frequency at which tick events are emitted.
const TICK_FPS: f64 = 30.0;

/// Representation of all possible events.
#[derive(Clone)]
pub enum Event {
    /// An event that is emitted on a regular schedule.
    ///
    /// Use this event to run any code which has to run outside of being a direct response to a user
    /// event. e.g. polling exernal systems, updating animations, or rendering the UI based on a
    /// fixed frame rate.
    Tick,
    /// Crossterm events.
    ///
    /// These events are emitted by the terminal.
    Crossterm(CrosstermEvent),
    /// Application events.
    ///
    /// Use this event to emit custom events that are specific to your application.
    App(AppEvent),
    /// Tab events.
    Tab(TabEvent),
}

#[derive(Clone)]
pub enum TabEvent {
    TabActions(TabActions),
    WidgetActions(WidgetActions),
    ComponentActions(ComponentActions),
}

#[derive(Clone)]
pub enum ComponentActions {
    S3ComponentActions(S3ComponentActions),
    DynamoDBComponentActions(DynamoDBComponentActions),
    CloudWatchComponentActions(CloudWatchComponentActions),
}
#[derive(Clone)]
pub enum CloudWatchComponentActions {
    SelectLogGroup(String),
    SearchLogs(String),
    ViewLogDetails(String),
    PopupDetails(String),
    NextFocus,
    PreviousFocus,
    WidgetActions(WidgetActions),
}

#[derive(Clone)]
pub enum S3ComponentActions {
    ArrowUp,
    ArrowDown,
    NextFocus,
    PreviousFocus,
    SelectBucket(String),
    NavigateFolder(String),
    NavigateUp,
    LoadPath(String, String), // bucket, path
    PopupDetails(String),
    WidgetActions(WidgetActions),
}

#[derive(Clone)]
pub enum DynamoDBComponentActions {
    ArrowUp,
    ArrowDown,
    NextFocus,
    PreviousFocus,
    SetTitle(String),
    SetQuery(String),
    PopupDetails(String),
    WidgetActions(WidgetActions),
}

#[derive(Clone)]
pub enum WidgetActions {
    AWSServiceNavigatorEvent(AWSServiceNavigatorEvent, WidgetType),
    InputBoxEvent(InputBoxEvent),
    ParagraphEvent(ParagraphEvent),
    ToggleFocus,
    PopupEvent(PopupEvent),
}

#[derive(Clone, Debug)]
pub enum TabActions {
    NextFocus,
    PreviousFocus,
    ProfileSelected(String),
    AWSServiceSelected(WidgetEventType),
}

#[derive(Clone)]
pub enum PopupEvent {
    SelectedItem(String),
    ArrowUp,
    ArrowDown,
    Enter,
    Escape,
    Cancel,
}

#[derive(Clone)]
pub enum AWSServiceNavigatorEvent {
    SelectedItem(WidgetEventType),
    ArrowUp,
    ArrowDown,
    PageDown,
    PageUp,
    Home,
    End,
    Enter,
    Escape,
    Cancel,
}

#[derive(Clone)]
pub enum InputBoxEvent {
    ArrowUp,
    ArrowDown,
    Enter,
    Escape,
    Cancel,
    Backspace,
    Delete,
    Left,
    Right,
    Written(String),
    KeyPress(KeyEvent),
}

#[derive(Clone)]
pub enum ParagraphEvent {
    ArrowUp,
    ArrowDown,
    Enter,
    Escape,
    Cancel,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum WidgetEventType {
    /// AWS profile event.
    S3,
    /// Active tab event.
    DynamoDB,
    CloudWatch,
    RecordSelected(String), // Add this new variant
}

impl WidgetEventType {
    // Update the VALUES array to include CloudWatch
    pub const VALUES: [Self; 3] = [Self::S3, Self::DynamoDB, Self::CloudWatch];
}

impl std::fmt::Display for WidgetEventType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            WidgetEventType::S3 => write!(f, "S3"),
            WidgetEventType::DynamoDB => write!(f, "DynamoDB"),
            WidgetEventType::CloudWatch => write!(f, "CloudWatch"),
            WidgetEventType::RecordSelected(record) => write!(f, "{}", record),
        }
    }
}
/// Application events.
///
/// You can extend this enum with your own custom events.
#[derive(Clone, Debug)]
pub enum AppEvent {
    /// Switch to the next tab.
    NextTab,
    /// Switch to the previous tab.
    PreviousTab,
    /// Create a new tab.
    CreateTab,
    /// Close the current tab.
    CloseTab,
    /// Quit the application.
    Quit,
}

#[derive(Hash, Eq, PartialEq, Clone, Copy)]
pub enum WidgetType {
    Default,
    AWSServiceNavigator,
    AWSService,
    S3,
    DynamoDB,
    CloudWatch,
    InputBox,
    QueryResultsNavigator,
}

/// Terminal event handler.
#[derive(Debug)]
pub struct EventHandler {
    /// Event sender channel.
    pub sender: mpsc::UnboundedSender<Event>,
    /// Event receiver channel.
    receiver: mpsc::UnboundedReceiver<Event>,
}

impl EventHandler {
    /// Constructs a new instance of [`EventHandler`] and spawns a new thread to handle events.
    pub fn new() -> Self {
        let (sender, receiver) = mpsc::unbounded_channel();
        let actor = EventTask::new(sender.clone());
        tokio::spawn(async { actor.run().await });
        Self { sender, receiver }
    }

    /// Receives an event from the sender.
    ///
    /// This function blocks until an event is received.
    ///
    /// # Errors
    ///
    /// This function returns an error if the sender channel is disconnected. This can happen if an
    /// error occurs in the event thread. In practice, this should not happen unless there is a
    /// problem with the underlying terminal.
    pub async fn next(&mut self) -> color_eyre::Result<Event> {
        self.receiver
            .recv()
            .await
            .ok_or_eyre("Failed to receive event")
    }

    /// Queue an app event to be sent to the event receiver.
    ///
    /// This is useful for sending events to the event handler which will be processed by the next
    /// iteration of the application's event loop.
    pub fn send(&mut self, event: Event) {
        // Ignore the result as the reciever cannot be dropped while this struct still has a
        // reference to it
        let _ = self.sender.send(event);
    }
}

/// A thread that handles reading crossterm events and emitting tick events on a regular schedule.
struct EventTask {
    /// Event sender channel.
    sender: mpsc::UnboundedSender<Event>,
}

impl EventTask {
    /// Constructs a new instance of [`EventThread`].
    fn new(sender: mpsc::UnboundedSender<Event>) -> Self {
        Self { sender }
    }

    /// Runs the event thread.
    ///
    /// This function emits tick events at a fixed rate and polls for crossterm events in between.
    async fn run(self) -> color_eyre::Result<()> {
        let tick_rate = Duration::from_secs_f64(TICK_FPS);
        let mut reader = crossterm::event::EventStream::new();
        let mut tick = tokio::time::interval(tick_rate);
        loop {
            let tick_delay = tick.tick();
            let crossterm_event = reader.next().fuse();
            tokio::select! {
              _ = self.sender.closed() => {
                break;
              }
              _ = tick_delay => {
                self.send(Event::Tick);
              }
              Some(Ok(evt)) = crossterm_event => {
                self.send(Event::Crossterm(evt));
              }
            };
        }
        Ok(())
    }

    /// Sends an event to the receiver.
    fn send(&self, event: Event) {
        // Ignores the result because shutting down the app drops the receiver, which causes the send
        // operation to fail. This is expected behavior and should not panic.
        let _ = self.sender.send(event);
    }
}
