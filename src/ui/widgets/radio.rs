//! Radio button widget for widget-tree UIs

use crate::input::InputEvent;
use crate::screen::Screen;
use crate::ui::layout::{Rect, SizeHint};
use crate::ui::theme::Theme;
use crate::ui::widget::EventResult;
use crate::ui::widget_tree::{EventPhase, TreeWidget};

/// A radio button widget
#[derive(Clone, Debug)]
pub struct RadioButton {
    label: String,
    selected: bool,
    focused: bool,
    action_name: String,
    min_width: Option<u16>,
}

impl RadioButton {
    pub fn new(label: impl Into<String>, action_name: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            selected: false,
            focused: false,
            action_name: action_name.into(),
            min_width: None,
        }
    }

    pub fn set_selected(&mut self, selected: bool) {
        self.selected = selected;
    }

    pub fn selected(&self) -> bool {
        self.selected
    }
}

impl TreeWidget for RadioButton {
    fn draw(&self, screen: &mut Screen, bounds: Rect, theme: &Theme) {
        if bounds.width < 4 || bounds.height < 1 {
            return;
        }

        let (fg, bg) = if self.focused {
            (theme.checkbox_focused_fg, theme.checkbox_focused_bg)
        } else {
            (theme.checkbox_fg, theme.checkbox_bg)
        };

        let mark = if self.selected { 'o' } else { ' ' };
        let text = format!("({}) {}", mark, self.label);
        for (i, ch) in text.chars().take(bounds.width as usize).enumerate() {
            screen.set(bounds.y, bounds.x + i as u16, ch, fg, bg);
        }
    }

    fn handle_event(&mut self, event: &InputEvent, bounds: Rect, phase: EventPhase) -> EventResult {
        if phase != EventPhase::Target {
            return EventResult::Ignored;
        }

        if self.focused {
            if matches!(event, InputEvent::Enter | InputEvent::Char(' ')) {
                return EventResult::Action(self.action_name.clone());
            }
        }

        if let InputEvent::MouseClick { row, col } = event {
            if bounds.contains(*row, *col) {
                return EventResult::Action(self.action_name.clone());
            }
        }

        EventResult::Ignored
    }

    fn size_hint(&self) -> SizeHint {
        let base = (self.label.chars().count() as u16).saturating_add(4);
        let min_width = self.min_width.unwrap_or(base).max(base);
        SizeHint {
            min_width,
            min_height: 1,
            flex: 0,
        }
    }

    fn focusable(&self) -> bool {
        true
    }

    fn set_focus(&mut self, focused: bool) {
        self.focused = focused;
    }

    fn wants_tight_width(&self) -> bool {
        true
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}
