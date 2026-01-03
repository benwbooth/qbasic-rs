//! Main widget trait for top-level UI components (MenuBar, Editor, etc.)

use crate::input::InputEvent;
use crate::screen::Screen;
use crate::state::{AppState, Focus};
use super::layout::Rect;

/// Actions that main widgets can trigger
#[derive(Clone, Debug)]
pub enum WidgetAction {
    /// Event was consumed, no further action needed
    Consumed,
    /// Event was ignored, try next widget
    Ignored,
    /// Change focus to a specific component
    SetFocus(Focus),
    /// Execute a menu action
    MenuAction(usize, usize),
    /// Execute a command in immediate window
    ExecuteCommand(String),
    /// Toggle a boolean state
    Toggle(&'static str),
    /// Start dragging/resizing
    StartDrag(&'static str),
}

impl WidgetAction {
    pub fn is_consumed(&self) -> bool {
        !matches!(self, WidgetAction::Ignored)
    }
}

/// Trait for main UI components that participate in the widget tree
pub trait MainWidget {
    /// Unique identifier for this widget
    fn id(&self) -> &'static str;

    /// Draw the widget
    fn draw(&mut self, screen: &mut Screen, state: &AppState, bounds: Rect);

    /// Handle an input event
    ///
    /// Returns a WidgetAction indicating what happened.
    /// Widgets should check if the event is within their bounds for mouse events.
    fn handle_event(&mut self, event: &InputEvent, state: &mut AppState, bounds: Rect) -> WidgetAction;

    /// Handle scroll wheel events
    fn handle_scroll(&mut self, event: &InputEvent, bounds: Rect) -> WidgetAction {
        let _ = (event, bounds);
        WidgetAction::Ignored
    }

    /// Whether this widget can receive focus
    fn focusable(&self) -> bool {
        false
    }

    /// The focus type this widget corresponds to (if any)
    fn focus_type(&self) -> Option<Focus> {
        None
    }
}

/// Check if a mouse event is within bounds
pub fn event_in_bounds(event: &InputEvent, bounds: Rect) -> bool {
    match event {
        InputEvent::MouseClick { row, col }
        | InputEvent::MouseRelease { row, col }
        | InputEvent::MouseDrag { row, col }
        | InputEvent::MouseMove { row, col }
        | InputEvent::ScrollUp { row, col }
        | InputEvent::ScrollDown { row, col }
        | InputEvent::ScrollLeft { row, col }
        | InputEvent::ScrollRight { row, col } => {
            // Convert to 0-based for comparison with bounds
            let row = row.saturating_sub(1);
            let col = col.saturating_sub(1);
            bounds.contains(row, col)
        }
        _ => true, // Non-mouse events aren't position-dependent
    }
}

/// Check if event is a scroll event
pub fn is_scroll_event(event: &InputEvent) -> bool {
    matches!(
        event,
        InputEvent::ScrollUp { .. }
            | InputEvent::ScrollDown { .. }
            | InputEvent::ScrollLeft { .. }
            | InputEvent::ScrollRight { .. }
    )
}

/// Get mouse position from event
pub fn mouse_position(event: &InputEvent) -> Option<(u16, u16)> {
    match event {
        InputEvent::MouseClick { row, col }
        | InputEvent::MouseRelease { row, col }
        | InputEvent::MouseDrag { row, col }
        | InputEvent::MouseMove { row, col }
        | InputEvent::ScrollUp { row, col }
        | InputEvent::ScrollDown { row, col }
        | InputEvent::ScrollLeft { row, col }
        | InputEvent::ScrollRight { row, col } => Some((*row, *col)),
        _ => None,
    }
}
