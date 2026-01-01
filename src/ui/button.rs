//! Button widget - a clickable button

use crate::input::InputEvent;
use crate::screen::Screen;
use crate::terminal::Color;
use super::layout::Rect;
use super::widget::{Widget, EventResult, mouse_position};

/// Colors for the button
#[derive(Clone, Copy)]
pub struct ButtonColors {
    pub text_fg: Color,
    pub text_bg: Color,
    pub focused_fg: Color,
    pub focused_bg: Color,
    pub bracket_fg: Color,
    pub bracket_bg: Color,
}

impl Default for ButtonColors {
    fn default() -> Self {
        Self {
            text_fg: Color::Black,
            text_bg: Color::LightGray,
            focused_fg: Color::White,
            focused_bg: Color::Black,
            bracket_fg: Color::Black,
            bracket_bg: Color::LightGray,
        }
    }
}

/// A clickable button widget
pub struct Button {
    /// Button label
    label: String,
    /// Colors
    colors: ButtonColors,
    /// Whether widget has focus
    focused: bool,
    /// Action name for clicks
    action_name: String,
    /// Whether to show angle brackets around label (QBasic style)
    show_brackets: bool,
}

impl Button {
    pub fn new(label: impl Into<String>, action_name: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            colors: ButtonColors::default(),
            focused: false,
            action_name: action_name.into(),
            show_brackets: true,
        }
    }

    pub fn with_colors(mut self, colors: ButtonColors) -> Self {
        self.colors = colors;
        self
    }

    pub fn with_brackets(mut self, show_brackets: bool) -> Self {
        self.show_brackets = show_brackets;
        self
    }

    /// Get the label
    pub fn label(&self) -> &str {
        &self.label
    }

    /// Set the label
    pub fn set_label(&mut self, label: impl Into<String>) {
        self.label = label.into();
    }

    /// Calculate display width
    pub fn display_width(&self) -> usize {
        if self.show_brackets {
            // < label >
            self.label.len() + 4
        } else {
            self.label.len()
        }
    }
}

impl Widget for Button {
    fn draw(&self, screen: &mut Screen, bounds: Rect) {
        if bounds.width == 0 || bounds.height == 0 {
            return;
        }

        let (text_fg, text_bg) = if self.focused {
            (self.colors.focused_fg, self.colors.focused_bg)
        } else {
            (self.colors.text_fg, self.colors.text_bg)
        };

        let mut col = bounds.x;
        let row = bounds.y;

        if self.show_brackets {
            // Draw opening bracket
            if col < bounds.x + bounds.width {
                screen.set(row, col, '<', self.colors.bracket_fg, self.colors.bracket_bg);
                col += 1;
            }
            if col < bounds.x + bounds.width {
                screen.set(row, col, ' ', text_fg, text_bg);
                col += 1;
            }
        }

        // Draw label
        for ch in self.label.chars() {
            if col >= bounds.x + bounds.width {
                break;
            }
            screen.set(row, col, ch, text_fg, text_bg);
            col += 1;
        }

        if self.show_brackets {
            if col < bounds.x + bounds.width {
                screen.set(row, col, ' ', text_fg, text_bg);
                col += 1;
            }
            // Draw closing bracket
            if col < bounds.x + bounds.width {
                screen.set(row, col, '>', self.colors.bracket_fg, self.colors.bracket_bg);
            }
        }
    }

    fn handle_event(&mut self, event: &InputEvent, bounds: Rect) -> EventResult {
        // Handle keyboard when focused
        if self.focused {
            match event {
                InputEvent::Enter | InputEvent::Char(' ') => {
                    return EventResult::Action(self.action_name.clone());
                }
                _ => {}
            }
        }

        // Handle mouse
        let (row, col) = match mouse_position(event) {
            Some(pos) => pos,
            None => return EventResult::Ignored,
        };

        if !bounds.contains(row, col) {
            return EventResult::Ignored;
        }

        if matches!(event, InputEvent::MouseClick { .. }) {
            return EventResult::Action(self.action_name.clone());
        }

        EventResult::Ignored
    }

    fn focusable(&self) -> bool {
        true
    }

    fn has_focus(&self) -> bool {
        self.focused
    }

    fn set_focus(&mut self, focused: bool) {
        self.focused = focused;
    }
}
