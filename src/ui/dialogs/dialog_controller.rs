//! Dialog controller trait for widget-tree dialogs.
//!
//! Each dialog implements this trait. The Dialogs container holds all dialogs
//! and routes events/drawing to the active one.

use crate::input::InputEvent;
use crate::screen::Screen;
use crate::state::AppState;

use super::{DialogContext, DialogResult};

/// Controller trait that all dialogs implement.
///
/// Dialogs are stored in the Dialogs struct with concrete types for direct access.
/// This trait provides the common interface for iteration and dispatch.
pub trait DialogController {
    /// Open the dialog, with access to editor and state for initialization
    fn open(&mut self, ctx: &mut DialogContext);

    /// Check if the dialog is currently open
    fn is_open(&self) -> bool;

    /// Close the dialog
    fn close(&mut self);

    /// Update screen size for layout/centering
    fn set_screen_size(&mut self, width: u16, height: u16);

    /// Draw the dialog (read-only access to state for display)
    fn draw(&mut self, screen: &mut Screen, state: &AppState);

    /// Handle an input event, doing any work directly via ctx
    fn handle_event(&mut self, event: &InputEvent, ctx: &mut DialogContext) -> DialogResult;
}
