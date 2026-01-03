//! Spacer widget - flexible empty space

use crate::input::InputEvent;
use crate::screen::Screen;
use crate::ui::layout::{Rect, SizeHint};
use crate::ui::theme::Theme;
use crate::ui::widget::EventResult;
use crate::ui::widget_tree::{TreeWidget, EventPhase};

/// Flexible spacer widget
///
/// A spacer fills available space and can be used to push widgets apart
/// or create padding. By default, it has flex=1 so it will expand to
/// fill available space in a layout.
#[derive(Clone, Debug)]
pub struct Spacer {
    flex: u16,
    min_size: u16,
}

impl Spacer {
    /// Create a new flexible spacer with flex=1
    pub fn new() -> Self {
        Self {
            flex: 1,
            min_size: 0,
        }
    }

    /// Create a fixed-size spacer (no flex)
    pub fn fixed(size: u16) -> Self {
        Self {
            flex: 0,
            min_size: size,
        }
    }

}

impl Default for Spacer {
    fn default() -> Self {
        Self::new()
    }
}

impl TreeWidget for Spacer {
    fn draw(&self, _screen: &mut Screen, _bounds: Rect, _theme: &Theme) {
        // Spacer doesn't draw anything - it just takes up space
        // The background is already filled by the parent container
    }

    fn handle_event(&mut self, _event: &InputEvent, _bounds: Rect, _phase: EventPhase) -> EventResult {
        EventResult::Ignored
    }

    fn size_hint(&self) -> SizeHint {
        SizeHint {
            min_width: self.min_size,
            min_height: self.min_size,
            flex: self.flex,
        }
    }

    fn focusable(&self) -> bool {
        false
    }

    fn wants_tight_width(&self) -> bool {
        self.flex == 0
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}
