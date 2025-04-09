use color_eyre::eyre::OptionExt;
use futures::{FutureExt, StreamExt};
use ratatui::crossterm::event::Event as CrosstermEvent;
use ratatui::crossterm::event::KeyEvent;
use std::time::Duration;
use tokio::sync::mpsc;

/// The frequency at which tick events are emitted.
const TICK_RATE: f64 = 30.0;

/// Main event enum for the application
#[derive(Clone)]
pub enum Event {
    /// Regular interval event for animations and polling
    Tick,
    /// Terminal events from crossterm
    Crossterm(CrosstermEvent),
    /// Custom application-level events
    App(AppEvent),
    /// Tab-related events
    Tab(TabEvent),
}

/// Events related to tab functionality
#[derive(Clone)]
pub enum TabEvent {
    TabAction(TabAction),
    WidgetActions(WidgetAction),
    ComponentActions(ComponentActions),
}

/// Actions for AWS service components
#[derive(Clone)]
pub enum ComponentActions {
    S3ComponentActions(S3ComponentActions),
    DynamoDBComponentActions(DynamoDBComponentActions),
    CloudWatchComponentActions(CloudWatchComponentActions),
}

/// Actions specific to CloudWatch services
#[derive(Clone)]
pub enum CloudWatchComponentActions {
    SelectLogGroup(String),
    SearchLogs(String),
    ViewLogDetails(String),
    PopupDetails(String),
    NextFocus,
    PreviousFocus,
    WidgetAction(WidgetAction),
}

/// Actions specific to S3 services
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
    WidgetAction(WidgetAction),
}

/// Actions specific to DynamoDB services
#[derive(Clone)]
pub enum DynamoDBComponentActions {
    ArrowUp,
    ArrowDown,
    NextFocus,
    PreviousFocus,
    SetTitle(String),
    SetQuery(String),
    PopupDetails(String),
    WidgetActions(WidgetAction),
}

/// Actions that can be performed on widgets
#[derive(Clone)]
pub enum WidgetAction {
    ServiceNavigatorEvent(ServiceNavigatorEvent, WidgetType),
    InputBoxEvent(InputBoxEvent),
    ParagraphEvent(ParagraphEvent),
    ToggleFocusState,
    PopupAction(PopupAction),
}

/// Actions specific to tab navigation and selection
#[derive(Clone, Debug)]
pub enum TabAction {
    NextFocus,
    PreviousFocus,
    SelectProfile(String),
    SelectService(WidgetEventType),
}

/// Events for popup widgets
#[derive(Clone)]
pub enum PopupAction {
    ItemSelected(String),
    ArrowUp,
    ArrowDown,
    Enter,
    Escape,
    Cancel,
}

/// Events for AWS service navigation
#[derive(Clone)]
pub enum ServiceNavigatorEvent {
    ItemSelected(WidgetEventType),
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

/// Events for input box widgets
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

/// Events for paragraph widgets
#[derive(Clone)]
pub enum ParagraphEvent {
    ArrowUp,
    ArrowDown,
    Enter,
    Escape,
    Cancel,
}

/// Types of widgets that can be interacted with
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum WidgetEventType {
    S3,
    DynamoDB,
    CloudWatch,
    RecordSelected(String),
}

impl WidgetEventType {
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

/// High-level application events
#[derive(Clone, Debug)]
pub enum AppEvent {
    NextTab,
    PreviousTab,
    CreateTab,
    CloseTab,
    Quit,
}

/// Identifiers for different widget types in the application
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

/// Handles event processing and distribution
#[derive(Debug)]
pub struct EventHandler {
    /// Channel for sending events
    pub sender: mpsc::UnboundedSender<Event>,
    /// Channel for receiving events
    receiver: mpsc::UnboundedReceiver<Event>,
}

impl EventHandler {
    /// Creates a new EventHandler with a channel for communication
    /// and spawns a background task to process events
    pub fn new() -> Self {
        let (sender, receiver) = mpsc::unbounded_channel();
        let actor = EventTask::new(sender.clone());
        tokio::spawn(async { actor.run().await });
        Self { sender, receiver }
    }

    /// Waits for and returns the next event from the channel
    /// 
    /// Returns an error if the event source disconnects
    pub async fn next(&mut self) -> color_eyre::Result<Event> {
        self.receiver
            .recv()
            .await
            .ok_or_eyre("Failed to receive event")
    }

    /// Queues an event to be processed in the next iteration of the event loop
    /// 
    /// Useful for internal event generation within the application
    pub fn send(&mut self, event: Event) {
        // Ignore the result as the receiver cannot be dropped while this struct exists
        let _ = self.sender.send(event);
    }
}

/// Handles event generation and dispatching for the application
struct EventTask {
    /// Channel for sending events to the main application
    sender: mpsc::UnboundedSender<Event>,
}

impl EventTask {
    /// Creates a new event task with the provided sender channel
    fn new(sender: mpsc::UnboundedSender<Event>) -> Self {
        Self { sender }
    }

    /// Runs the event thread.
    ///
    /// This function emits tick events at a fixed rate and polls for crossterm events in between.
    async fn run(self) -> color_eyre::Result<()> {
        // Configure the tick rate for UI updates
        let tick_rate = Duration::from_secs_f64(1.0 / TICK_RATE);
        // Create an event stream for terminal input
        let mut reader = crossterm::event::EventStream::new();
        // Set up interval timer for regular tick events
        let mut tick = tokio::time::interval(tick_rate);
        loop {
            let tick_delay = tick.tick();
            let crossterm_event = reader.next().fuse();
            tokio::select! {
              // Exit if the receiver channel is closed
              _ = self.sender.closed() => {
                break;
              }
              // Send a tick event at regular intervals
              _ = tick_delay => {
                self.send(Event::Tick);
              }
              // Process terminal input events
              Some(Ok(evt)) = crossterm_event => {
                self.send(Event::Crossterm(evt));
              }
            };
        }
        Ok(())
    }

    /// Sends an event to the receiver.
    /// 
    /// This is internal to the event task and should not be confused with
    /// the public EventHandler::send method.
    fn send(&self, event: Event) {
        // Ignores the result because shutting down the app drops the receiver, which causes the send
        // operation to fail. This is expected behavior and should not panic.
        let _ = self.sender.send(event);
    }
}