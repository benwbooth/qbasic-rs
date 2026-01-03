//! Dialog context - provides dialogs with mutable access to app resources.
//!
//! Instead of dialogs returning actions for the app to execute, dialogs
//! receive a DialogContext and do their work directly.

use crate::state::AppState;
use crate::ui::Editor;

/// Context passed to dialogs for handle_event.
/// Provides mutable access to editor and app state.
pub struct DialogContext<'a> {
    pub editor: &'a mut Editor,
    pub state: &'a mut AppState,
}

/// Result from dialog event handling.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum DialogResult {
    /// Dialog is still open, no action needed
    #[default]
    Open,
    /// Dialog closed normally
    Closed,
}
