//! AWS Terminal UI - Main Entry Point
//!
//! This is the main entry point for the AWS TUI application.
//! It initializes the terminal interface, error handling, and runs the application event loop.

use crate::app::App;

/// Application state and lifecycle management
pub mod app;
/// UI components that represent AWS services and data
pub mod components;
/// Event management system for handling user input and component events
pub mod event_managment;
/// AWS service clients and profile management
pub mod services;
/// UI rendering and layout modules
pub mod ui;
/// Reusable UI widgets for building the interface
pub mod widgets;

#[tokio::main]
async fn main() -> color_eyre::Result<()> {
    // Initialize error handling with detailed backtraces
    color_eyre::install()?;
    
    // Initialize the terminal UI with ratatui
    let terminal = ratatui::init();
    
    // Create and run the application with the configured terminal
    let result = App::new().run(terminal).await;
    
    // Restore terminal to original state before exiting
    ratatui::restore();
    
    // Return the final result, which includes any errors that occurred
    result
}