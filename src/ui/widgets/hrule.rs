//! Horizontal rule widget - draws a horizontal separator line

use crate::input::InputEvent;
use crate::screen::Screen;
use crate::ui::layout::{Rect, SizeHint};
use crate::ui::theme::Theme;
use crate::ui::widget::EventResult;
use crate::ui::widget_tree::{TreeWidget, EventPhase};

/// Horizontal rule style
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum HRuleStyle {
    /// Simple line: ───────
    #[default]
    Line,
    /// T-connectors for inside boxes: ├───────┤
    TConnector,
}

/// Horizontal separator line widget
#[derive(Clone, Debug, Default)]
pub struct HRule {
    style: HRuleStyle,
}

impl HRule {
    /// Create a horizontal rule with T-connectors (for inside boxes)
    pub fn t_connector() -> Self {
        Self {
            style: HRuleStyle::TConnector,
        }
    }
}

impl TreeWidget for HRule {
    fn draw(&self, screen: &mut Screen, bounds: Rect, theme: &Theme) {
        let fg = theme.separator_fg;
        let bg = theme.separator_bg;

        if bounds.width < 1 {
            return;
        }

        let row = bounds.y;
        let col = bounds.x;

        match self.style {
            HRuleStyle::Line => {
                for c in 0..bounds.width {
                    screen.set(row, col + c, '─', fg, bg);
                }
            }
            HRuleStyle::TConnector => {
                if bounds.width < 2 {
                    screen.set(row, col, '─', fg, bg);
                    return;
                }
                screen.set(row, col, '├', fg, bg);
                for c in 1..bounds.width - 1 {
                    screen.set(row, col + c, '─', fg, bg);
                }
                screen.set(row, col + bounds.width - 1, '┤', fg, bg);
            }
        }
    }

    fn handle_event(&mut self, _event: &InputEvent, _bounds: Rect, _phase: EventPhase) -> EventResult {
        EventResult::Ignored
    }

    fn size_hint(&self) -> SizeHint {
        SizeHint {
            min_width: 1,
            min_height: 1,
            flex: 0, // HRule doesn't flex vertically
        }
    }

    fn focusable(&self) -> bool {
        false
    }

    fn wants_full_bleed(&self) -> bool {
        // TConnector style needs full width to attach to container borders
        matches!(self.style, HRuleStyle::TConnector)
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}
