//! Widget trait and event system for UI components
#![allow(dead_code)]

use crate::input::InputEvent;
use crate::screen::Screen;
use super::layout::Rect;

/// Result of handling an event
#[derive(Clone, Debug, PartialEq)]
pub enum EventResult {
    /// Event was handled, stop propagation
    Consumed,
    /// Event was not handled, continue propagation
    Ignored,
    /// Event triggered a named action
    Action(String),
}

impl EventResult {
    /// Check if the event was consumed (either Consumed or Action)
    pub fn is_consumed(&self) -> bool {
        !matches!(self, EventResult::Ignored)
    }
}

/// Common interface for all UI widgets
///
/// Widgets encapsulate both rendering and event handling for a UI component.
/// They receive their bounds from the layout system and draw themselves
/// within those bounds.
pub trait Widget {
    /// Draw the widget to the screen within the given bounds
    fn draw(&self, screen: &mut Screen, bounds: Rect);

    /// Handle an input event
    ///
    /// Returns EventResult indicating whether the event was handled.
    /// Mouse events should check if the click is within bounds before handling.
    fn handle_event(&mut self, event: &InputEvent, bounds: Rect) -> EventResult;

    /// Whether this widget can receive keyboard focus
    fn focusable(&self) -> bool {
        false
    }

    /// Whether this widget currently has focus
    fn has_focus(&self) -> bool {
        false
    }

    /// Set focus state
    fn set_focus(&mut self, _focused: bool) {}
}

/// Check if a mouse event is within bounds
pub fn is_mouse_in_bounds(event: &InputEvent, bounds: Rect) -> bool {
    match event {
        InputEvent::MouseClick { row, col }
        | InputEvent::MouseRelease { row, col }
        | InputEvent::MouseDrag { row, col }
        | InputEvent::MouseMove { row, col }
        | InputEvent::ScrollUp { row, col }
        | InputEvent::ScrollDown { row, col }
        | InputEvent::ScrollLeft { row, col }
        | InputEvent::ScrollRight { row, col } => bounds.contains(*row, *col),
        _ => true, // Non-mouse events aren't position-dependent
    }
}

/// Extract mouse position from an event
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
