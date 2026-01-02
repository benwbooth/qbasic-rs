//! Modal dialog system
#![allow(dead_code)]
//!
//! Modal dialogs capture all input events when open. The main loop
//! doesn't need to know what type of dialog is open - it just routes
//! events to the modal and handles the result.

use std::path::PathBuf;
use crate::input::InputEvent;
use crate::screen::Screen;

/// Result of handling an event in a modal dialog
pub enum ModalResult {
    /// Event handled, keep dialog open
    Continue,
    /// Close dialog with no action
    Close,
    /// Close dialog with a typed result
    Action(ModalAction),
}

/// Actions that can be returned by modal dialogs
#[derive(Debug)]
pub enum ModalAction {
    /// File was selected for opening
    FileOpen(PathBuf),
    /// File was selected for saving
    FileSave(PathBuf),
    /// Help topic requested
    Help(String),
    /// Generic confirmation (Yes/No/Cancel dialogs)
    Confirm(bool),
}

/// A modal dialog that captures all events when open
pub trait ModalDialog {
    /// Draw the dialog
    fn draw(&self, screen: &mut Screen);

    /// Handle an input event
    /// Returns what action to take (continue, close, or action)
    fn handle_event(&mut self, event: &InputEvent) -> ModalResult;

    /// Get the dialog title (for debugging/logging)
    fn title(&self) -> &str;
}
