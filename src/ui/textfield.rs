//! TextField widget - a single-line text input field
#![allow(dead_code)]

use crate::input::InputEvent;
use crate::screen::Screen;
use crate::terminal::Color;
use super::layout::Rect;
use super::widget::{Widget, EventResult, mouse_position};

/// Colors for the text field
#[derive(Clone, Copy)]
pub struct TextFieldColors {
    pub text_fg: Color,
    pub text_bg: Color,
    pub cursor_fg: Color,
    pub cursor_bg: Color,
    pub selection_fg: Color,
    pub selection_bg: Color,
}

impl Default for TextFieldColors {
    fn default() -> Self {
        Self {
            text_fg: Color::Black,
            text_bg: Color::Cyan,
            cursor_fg: Color::Cyan,
            cursor_bg: Color::Black,
            selection_fg: Color::White,
            selection_bg: Color::Blue,
        }
    }
}

/// A single-line text input widget
pub struct TextField {
    /// Text content
    text: String,
    /// Cursor position (character index)
    cursor_pos: usize,
    /// Horizontal scroll offset
    scroll_offset: usize,
    /// Selection anchor (if any)
    selection_anchor: Option<usize>,
    /// Colors
    colors: TextFieldColors,
    /// Whether widget has focus
    focused: bool,
    /// Action prefix for events
    action_prefix: String,
}

impl TextField {
    pub fn new(action_prefix: impl Into<String>) -> Self {
        Self {
            text: String::new(),
            cursor_pos: 0,
            scroll_offset: 0,
            selection_anchor: None,
            colors: TextFieldColors::default(),
            focused: false,
            action_prefix: action_prefix.into(),
        }
    }

    pub fn with_colors(mut self, colors: TextFieldColors) -> Self {
        self.colors = colors;
        self
    }

    pub fn with_text(mut self, text: impl Into<String>) -> Self {
        self.text = text.into();
        self.cursor_pos = self.text.len();
        self
    }

    /// Get the text content
    pub fn text(&self) -> &str {
        &self.text
    }

    /// Set the text content
    pub fn set_text(&mut self, text: impl Into<String>) {
        self.text = text.into();
        self.cursor_pos = self.cursor_pos.min(self.text.len());
        self.ensure_cursor_visible();
    }

    /// Get cursor position
    pub fn cursor_pos(&self) -> usize {
        self.cursor_pos
    }

    /// Set cursor position
    pub fn set_cursor_pos(&mut self, pos: usize) {
        self.cursor_pos = pos.min(self.text.len());
        self.ensure_cursor_visible();
    }

    /// Get visible width
    fn visible_width(&self, bounds: Rect) -> usize {
        bounds.width as usize
    }

    /// Ensure cursor is visible within the scroll window
    fn ensure_cursor_visible(&mut self) {
        // This needs the actual width, which we won't have until draw
        // So we'll call it with the actual width from handle_event/draw
    }

    fn ensure_cursor_visible_with_width(&mut self, visible_width: usize) {
        if visible_width == 0 {
            return;
        }

        // Leave one character at the end for the cursor when at end of text
        let usable_width = visible_width.saturating_sub(1);

        if self.cursor_pos < self.scroll_offset {
            self.scroll_offset = self.cursor_pos;
        } else if self.cursor_pos > self.scroll_offset + usable_width {
            self.scroll_offset = self.cursor_pos.saturating_sub(usable_width);
        }
    }

    /// Get the selected range (if any)
    fn selection_range(&self) -> Option<(usize, usize)> {
        self.selection_anchor.map(|anchor| {
            if anchor < self.cursor_pos {
                (anchor, self.cursor_pos)
            } else {
                (self.cursor_pos, anchor)
            }
        })
    }

    /// Get selected text
    pub fn selected_text(&self) -> Option<&str> {
        self.selection_range().map(|(start, end)| &self.text[start..end])
    }

    /// Delete selected text
    fn delete_selection(&mut self) -> bool {
        if let Some((start, end)) = self.selection_range() {
            self.text.replace_range(start..end, "");
            self.cursor_pos = start;
            self.selection_anchor = None;
            return true;
        }
        false
    }

    /// Clear selection
    fn clear_selection(&mut self) {
        self.selection_anchor = None;
    }

    /// Start selection from current cursor position
    fn start_selection(&mut self) {
        if self.selection_anchor.is_none() {
            self.selection_anchor = Some(self.cursor_pos);
        }
    }

    /// Select all text
    pub fn select_all(&mut self) {
        self.selection_anchor = Some(0);
        self.cursor_pos = self.text.len();
    }

    /// Insert character at cursor
    fn insert_char(&mut self, ch: char) {
        self.delete_selection();
        self.text.insert(self.cursor_pos, ch);
        self.cursor_pos += 1;
    }

    /// Delete character before cursor (backspace)
    fn backspace(&mut self) {
        if self.delete_selection() {
            return;
        }
        if self.cursor_pos > 0 {
            self.cursor_pos -= 1;
            self.text.remove(self.cursor_pos);
        }
    }

    /// Delete character at cursor (delete)
    fn delete(&mut self) {
        if self.delete_selection() {
            return;
        }
        if self.cursor_pos < self.text.len() {
            self.text.remove(self.cursor_pos);
        }
    }

    /// Move cursor left
    fn move_left(&mut self, keep_selection: bool) {
        if keep_selection {
            self.start_selection();
        } else {
            // If there's a selection, move to its start
            if let Some((start, _)) = self.selection_range() {
                self.cursor_pos = start;
                self.clear_selection();
                return;
            }
            self.clear_selection();
        }
        if self.cursor_pos > 0 {
            self.cursor_pos -= 1;
        }
    }

    /// Move cursor right
    fn move_right(&mut self, keep_selection: bool) {
        if keep_selection {
            self.start_selection();
        } else {
            // If there's a selection, move to its end
            if let Some((_, end)) = self.selection_range() {
                self.cursor_pos = end;
                self.clear_selection();
                return;
            }
            self.clear_selection();
        }
        if self.cursor_pos < self.text.len() {
            self.cursor_pos += 1;
        }
    }

    /// Move cursor to start
    fn move_home(&mut self, keep_selection: bool) {
        if keep_selection {
            self.start_selection();
        } else {
            self.clear_selection();
        }
        self.cursor_pos = 0;
    }

    /// Move cursor to end
    fn move_end(&mut self, keep_selection: bool) {
        if keep_selection {
            self.start_selection();
        } else {
            self.clear_selection();
        }
        self.cursor_pos = self.text.len();
    }

    /// Move cursor to next word boundary
    fn move_word_right(&mut self, keep_selection: bool) {
        if keep_selection {
            self.start_selection();
        } else {
            self.clear_selection();
        }

        // Skip current word/whitespace
        while self.cursor_pos < self.text.len() {
            let ch = self.text.chars().nth(self.cursor_pos).unwrap_or(' ');
            if ch.is_alphanumeric() || ch == '_' {
                self.cursor_pos += 1;
            } else {
                break;
            }
        }
        // Skip whitespace
        while self.cursor_pos < self.text.len() {
            let ch = self.text.chars().nth(self.cursor_pos).unwrap_or(' ');
            if ch.is_alphanumeric() || ch == '_' {
                break;
            }
            self.cursor_pos += 1;
        }
    }

    /// Move cursor to previous word boundary
    fn move_word_left(&mut self, keep_selection: bool) {
        if keep_selection {
            self.start_selection();
        } else {
            self.clear_selection();
        }

        if self.cursor_pos > 0 {
            self.cursor_pos -= 1;
        }

        // Skip whitespace
        while self.cursor_pos > 0 {
            let ch = self.text.chars().nth(self.cursor_pos).unwrap_or(' ');
            if ch.is_alphanumeric() || ch == '_' {
                break;
            }
            self.cursor_pos -= 1;
        }
        // Skip word
        while self.cursor_pos > 0 {
            let ch = self.text.chars().nth(self.cursor_pos - 1).unwrap_or(' ');
            if ch.is_alphanumeric() || ch == '_' {
                self.cursor_pos -= 1;
            } else {
                break;
            }
        }
    }
}

impl Widget for TextField {
    fn draw(&self, screen: &mut Screen, bounds: Rect) {
        if bounds.width == 0 || bounds.height == 0 {
            return;
        }

        let visible_width = bounds.width as usize;
        let visible_text: String = self.text
            .chars()
            .skip(self.scroll_offset)
            .take(visible_width)
            .collect();

        let selection_range = self.selection_range();

        // Draw each character
        for i in 0..visible_width {
            let text_index = self.scroll_offset + i;
            let ch = visible_text.chars().nth(i).unwrap_or(' ');

            // Determine colors
            let (fg, bg) = if self.focused && text_index == self.cursor_pos {
                // Cursor position
                (self.colors.cursor_fg, self.colors.cursor_bg)
            } else if let Some((sel_start, sel_end)) = selection_range {
                if text_index >= sel_start && text_index < sel_end {
                    // Selected
                    (self.colors.selection_fg, self.colors.selection_bg)
                } else {
                    // Normal
                    (self.colors.text_fg, self.colors.text_bg)
                }
            } else {
                // Normal
                (self.colors.text_fg, self.colors.text_bg)
            };

            screen.set(bounds.y, bounds.x + i as u16, ch, fg, bg);
        }
    }

    fn handle_event(&mut self, event: &InputEvent, bounds: Rect) -> EventResult {
        let visible_width = self.visible_width(bounds);
        self.ensure_cursor_visible_with_width(visible_width);

        // Handle keyboard events if focused
        if self.focused {
            match event {
                InputEvent::Char(ch) => {
                    self.insert_char(*ch);
                    self.ensure_cursor_visible_with_width(visible_width);
                    return EventResult::Action(format!("{}_change", self.action_prefix));
                }
                InputEvent::Backspace => {
                    self.backspace();
                    self.ensure_cursor_visible_with_width(visible_width);
                    return EventResult::Action(format!("{}_change", self.action_prefix));
                }
                InputEvent::Delete => {
                    self.delete();
                    return EventResult::Action(format!("{}_change", self.action_prefix));
                }
                InputEvent::CursorLeft => {
                    self.move_left(false);
                    self.ensure_cursor_visible_with_width(visible_width);
                    return EventResult::Consumed;
                }
                InputEvent::CursorRight => {
                    self.move_right(false);
                    self.ensure_cursor_visible_with_width(visible_width);
                    return EventResult::Consumed;
                }
                InputEvent::ShiftLeft => {
                    self.move_left(true);
                    self.ensure_cursor_visible_with_width(visible_width);
                    return EventResult::Consumed;
                }
                InputEvent::ShiftRight => {
                    self.move_right(true);
                    self.ensure_cursor_visible_with_width(visible_width);
                    return EventResult::Consumed;
                }
                InputEvent::Home => {
                    self.move_home(false);
                    self.ensure_cursor_visible_with_width(visible_width);
                    return EventResult::Consumed;
                }
                InputEvent::End => {
                    self.move_end(false);
                    self.ensure_cursor_visible_with_width(visible_width);
                    return EventResult::Consumed;
                }
                InputEvent::ShiftHome => {
                    self.move_home(true);
                    self.ensure_cursor_visible_with_width(visible_width);
                    return EventResult::Consumed;
                }
                InputEvent::ShiftEnd => {
                    self.move_end(true);
                    self.ensure_cursor_visible_with_width(visible_width);
                    return EventResult::Consumed;
                }
                InputEvent::CtrlLeft => {
                    self.move_word_left(false);
                    self.ensure_cursor_visible_with_width(visible_width);
                    return EventResult::Consumed;
                }
                InputEvent::CtrlRight => {
                    self.move_word_right(false);
                    self.ensure_cursor_visible_with_width(visible_width);
                    return EventResult::Consumed;
                }
                InputEvent::CtrlShiftLeft => {
                    self.move_word_left(true);
                    self.ensure_cursor_visible_with_width(visible_width);
                    return EventResult::Consumed;
                }
                InputEvent::CtrlShiftRight => {
                    self.move_word_right(true);
                    self.ensure_cursor_visible_with_width(visible_width);
                    return EventResult::Consumed;
                }
                InputEvent::CtrlA => {
                    self.select_all();
                    return EventResult::Consumed;
                }
                InputEvent::Enter => {
                    return EventResult::Action(format!("{}_submit", self.action_prefix));
                }
                _ => {}
            }
        }

        // Handle mouse events
        let (row, col) = match mouse_position(event) {
            Some(pos) => pos,
            None => return EventResult::Ignored,
        };

        if !bounds.contains(row, col) {
            return EventResult::Ignored;
        }

        match event {
            InputEvent::MouseClick { .. } => {
                // Calculate character position from click
                let click_offset = (col - bounds.x) as usize;
                let char_pos = self.scroll_offset + click_offset;
                self.cursor_pos = char_pos.min(self.text.len());
                self.clear_selection();
                return EventResult::Action(format!("{}_focus", self.action_prefix));
            }
            _ => {}
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
