//! QBasic Immediate window - a scratchpad for executing BASIC statements
//!
//! Works like real QBasic: a small editor where pressing Enter executes
//! the current line instead of inserting a newline.
#![allow(dead_code)]

use crate::screen::Screen;
use crate::terminal::Color;
use crate::state::AppState;
use super::layout::{Rect, LayoutItem};
use super::window_chrome;

/// Maximum number of lines in the immediate window
const MAX_LINES: usize = 10;

/// The immediate window - a scratchpad editor where Enter executes the current line
pub struct ImmediateWindow {
    /// Lines of code (up to MAX_LINES)
    lines: Vec<String>,
    /// Current cursor line (0-based)
    cursor_line: usize,
    /// Current cursor column (0-based, in characters)
    cursor_col: usize,
    /// Scroll offset for horizontal scrolling
    scroll_x: usize,
    /// Scroll offset for vertical scrolling (when more than visible lines)
    scroll_y: usize,
}

impl ImmediateWindow {
    pub fn new() -> Self {
        Self {
            lines: vec![String::new()], // Start with one empty line
            cursor_line: 0,
            cursor_col: 0,
            scroll_x: 0,
            scroll_y: 0,
        }
    }

    /// Get the current line
    fn current_line(&self) -> &str {
        self.lines.get(self.cursor_line).map(|s| s.as_str()).unwrap_or("")
    }

    /// Get the current line mutably
    fn current_line_mut(&mut self) -> &mut String {
        // Ensure line exists
        while self.lines.len() <= self.cursor_line {
            self.lines.push(String::new());
        }
        &mut self.lines[self.cursor_line]
    }

    /// Ensure cursor is within valid bounds
    fn clamp_cursor(&mut self) {
        if self.lines.is_empty() {
            self.lines.push(String::new());
        }
        self.cursor_line = self.cursor_line.min(self.lines.len() - 1);
        let line_len = self.current_line().chars().count();
        self.cursor_col = self.cursor_col.min(line_len);
    }

    /// Ensure cursor is visible within the viewport
    fn ensure_visible(&mut self, visible_height: usize, visible_width: usize) {
        // Vertical scrolling
        if self.cursor_line < self.scroll_y {
            self.scroll_y = self.cursor_line;
        } else if self.cursor_line >= self.scroll_y + visible_height {
            self.scroll_y = self.cursor_line - visible_height + 1;
        }

        // Horizontal scrolling
        if self.cursor_col < self.scroll_x {
            self.scroll_x = self.cursor_col;
        } else if self.cursor_col >= self.scroll_x + visible_width {
            self.scroll_x = self.cursor_col - visible_width + 1;
        }
    }

    /// Draw the immediate window
    pub fn draw(&self, screen: &mut Screen, state: &AppState, bounds: Rect, has_focus: bool) {
        let row = bounds.y + 1; // 1-based row
        let col = bounds.x + 1;
        let width = bounds.width;
        let height = bounds.height;

        // Draw border with title
        let border_fg = if has_focus { Color::White } else { Color::LightGray };
        screen.draw_box(row, col, width, height, border_fg, Color::Blue);

        // Title (centered)
        let title = " Immediate ";
        let title_x = col + (width.saturating_sub(title.len() as u16)) / 2;
        screen.write_str(row, title_x, title, border_fg, Color::Blue);

        // Maximize/Minimize button at right of title bar
        window_chrome::draw_maximize_button(screen, row, col, width, state.immediate_maximized, border_fg, Color::Blue);

        // Content area
        let content_row = row + 1;
        let content_col = col + 1;
        let content_width = width.saturating_sub(2) as usize;
        let content_height = height.saturating_sub(2) as usize;

        // Clear content area
        for r in 0..content_height {
            for c in 0..content_width {
                screen.set(content_row + r as u16, content_col + c as u16, ' ', Color::Yellow, Color::Blue);
            }
        }

        // Draw lines
        for i in 0..content_height {
            let line_idx = self.scroll_y + i;
            if line_idx >= self.lines.len() {
                break;
            }

            let line = &self.lines[line_idx];
            let display_row = content_row + i as u16;

            // Get visible portion of line
            let chars: Vec<char> = line.chars().collect();
            for (j, &ch) in chars.iter().skip(self.scroll_x).take(content_width).enumerate() {
                screen.set(display_row, content_col + j as u16, ch, Color::Yellow, Color::Blue);
            }
        }

        // Set cursor position if focused
        if has_focus {
            let cursor_screen_row = content_row + (self.cursor_line - self.scroll_y) as u16;
            let cursor_screen_col = content_col + (self.cursor_col - self.scroll_x) as u16;

            // Only show cursor if within visible area
            if self.cursor_line >= self.scroll_y
                && self.cursor_line < self.scroll_y + content_height
                && self.cursor_col >= self.scroll_x
                && self.cursor_col < self.scroll_x + content_width
            {
                screen.set_cursor(cursor_screen_row, cursor_screen_col);
                screen.set_cursor_style(crate::terminal::CursorStyle::BlinkingUnderline);
                screen.set_cursor_visible(true);
            }
        }
    }

    /// Handle input for the immediate window
    /// Returns Some(command) if a line should be executed
    pub fn handle_input(&mut self, event: &crate::input::InputEvent, bounds: Rect) -> Option<String> {
        use crate::input::InputEvent;

        let content_height = bounds.height.saturating_sub(4) as usize; // Account for borders
        let content_width = bounds.width.saturating_sub(4) as usize;

        match event {
            InputEvent::Char(c) => {
                // Insert character at cursor
                let cursor_col = self.cursor_col;
                let line = self.current_line_mut();
                let byte_pos: usize = line.chars().take(cursor_col).map(|c| c.len_utf8()).sum();
                line.insert(byte_pos, *c);
                self.cursor_col += 1;
                self.ensure_visible(content_height, content_width);
                None
            }
            InputEvent::Enter => {
                // Execute the current line
                let line = self.current_line().trim().to_string();
                if !line.is_empty() {
                    // Move to next line or create one
                    if self.cursor_line + 1 >= self.lines.len() {
                        self.lines.push(String::new());
                    }

                    // Enforce MAX_LINES limit - remove oldest line if needed
                    while self.lines.len() > MAX_LINES {
                        self.lines.remove(0);
                        if self.cursor_line > 0 {
                            self.cursor_line -= 1;
                        }
                    }

                    self.cursor_line = self.cursor_line.min(self.lines.len() - 1);
                    self.cursor_col = 0;
                    self.ensure_visible(content_height, content_width);
                    Some(line)
                } else {
                    // Empty line - just move to next line
                    if self.cursor_line + 1 < self.lines.len() {
                        self.cursor_line += 1;
                        self.cursor_col = 0;
                    }
                    self.ensure_visible(content_height, content_width);
                    None
                }
            }
            InputEvent::Backspace => {
                if self.cursor_col > 0 {
                    let cursor_col = self.cursor_col;
                    let line = self.current_line_mut();
                    let byte_pos: usize = line.chars().take(cursor_col - 1).map(|c| c.len_utf8()).sum();
                    let char_len = line.chars().nth(cursor_col - 1).map(|c| c.len_utf8()).unwrap_or(0);
                    line.drain(byte_pos..byte_pos + char_len);
                    self.cursor_col -= 1;
                } else if self.cursor_line > 0 {
                    // At start of line - merge with previous line
                    let current = self.lines.remove(self.cursor_line);
                    self.cursor_line -= 1;
                    self.cursor_col = self.lines[self.cursor_line].chars().count();
                    self.lines[self.cursor_line].push_str(&current);
                }
                self.ensure_visible(content_height, content_width);
                None
            }
            InputEvent::Delete => {
                let cursor_col = self.cursor_col;
                let cursor_line = self.cursor_line;
                let line = self.current_line_mut();
                let char_count = line.chars().count();
                if cursor_col < char_count {
                    let byte_pos: usize = line.chars().take(cursor_col).map(|c| c.len_utf8()).sum();
                    let char_len = line.chars().nth(cursor_col).map(|c| c.len_utf8()).unwrap_or(0);
                    line.drain(byte_pos..byte_pos + char_len);
                } else if cursor_line + 1 < self.lines.len() {
                    // At end of line - merge with next line
                    let next = self.lines.remove(cursor_line + 1);
                    self.lines[cursor_line].push_str(&next);
                }
                None
            }
            InputEvent::CursorLeft => {
                if self.cursor_col > 0 {
                    self.cursor_col -= 1;
                } else if self.cursor_line > 0 {
                    self.cursor_line -= 1;
                    self.cursor_col = self.current_line().chars().count();
                }
                self.ensure_visible(content_height, content_width);
                None
            }
            InputEvent::CursorRight => {
                let line_len = self.current_line().chars().count();
                if self.cursor_col < line_len {
                    self.cursor_col += 1;
                } else if self.cursor_line + 1 < self.lines.len() {
                    self.cursor_line += 1;
                    self.cursor_col = 0;
                }
                self.ensure_visible(content_height, content_width);
                None
            }
            InputEvent::CursorUp => {
                if self.cursor_line > 0 {
                    self.cursor_line -= 1;
                    self.clamp_cursor();
                }
                self.ensure_visible(content_height, content_width);
                None
            }
            InputEvent::CursorDown => {
                if self.cursor_line + 1 < self.lines.len() {
                    self.cursor_line += 1;
                    self.clamp_cursor();
                }
                self.ensure_visible(content_height, content_width);
                None
            }
            InputEvent::Home => {
                self.cursor_col = 0;
                self.ensure_visible(content_height, content_width);
                None
            }
            InputEvent::End => {
                self.cursor_col = self.current_line().chars().count();
                self.ensure_visible(content_height, content_width);
                None
            }
            _ => None,
        }
    }

    /// Clear all lines
    pub fn clear(&mut self) {
        self.lines = vec![String::new()];
        self.cursor_line = 0;
        self.cursor_col = 0;
        self.scroll_x = 0;
        self.scroll_y = 0;
    }
}

impl Default for ImmediateWindow {
    fn default() -> Self {
        Self::new()
    }
}

/// Result of handling an immediate window click
#[derive(Clone, Debug, PartialEq)]
pub enum ImmediateClickResult {
    /// No action taken
    None,
    /// Toggle maximize/minimize
    ToggleMaximize,
    /// Start resize dragging (only when not maximized)
    StartResize,
    /// Focus the window
    Focus,
}

impl ImmediateWindow {
    /// Build a layout for the title bar (first row of the window)
    pub fn title_bar_layout(&self, bounds: Rect) -> LayoutItem {
        LayoutItem::hstack(vec![
            LayoutItem::leaf("left_border").fixed_width(1),
            LayoutItem::leaf("title").width(super::layout::Size::Flex(1)),
            LayoutItem::leaf("toggle_button").fixed_width(3),
            LayoutItem::leaf("right_border").fixed_width(2),
        ])
        .fixed_height(1)
        .fixed_width(bounds.width)
    }

    /// Handle a click on the immediate window
    pub fn handle_click(&mut self, row: u16, col: u16, bounds: Rect, is_maximized: bool) -> ImmediateClickResult {
        let title_row = bounds.y + 1;
        let window_col = bounds.x + 1;
        let content_row_start = title_row + 1;
        let content_col_start = window_col + 1;
        let content_width = bounds.width.saturating_sub(2) as usize;
        let content_height = bounds.height.saturating_sub(2) as usize;

        // Check if click is on the title bar row
        if row == title_row {
            if window_chrome::is_maximize_button_click(row, col, title_row, window_col, bounds.width) {
                return ImmediateClickResult::ToggleMaximize;
            }
            if window_chrome::is_title_bar_click(row, col, title_row, window_col, bounds.width) && !is_maximized {
                return ImmediateClickResult::StartResize;
            }
        }

        // Check if click is in the content area - position cursor there
        if row >= content_row_start
            && row < content_row_start + content_height as u16
            && col >= content_col_start
            && col < content_col_start + content_width as u16
        {
            let click_line = self.scroll_y + (row - content_row_start) as usize;
            let click_col = self.scroll_x + (col - content_col_start) as usize;

            if click_line < self.lines.len() {
                self.cursor_line = click_line;
                let line_len = self.lines[self.cursor_line].chars().count();
                self.cursor_col = click_col.min(line_len);
            }
        }

        ImmediateClickResult::Focus
    }
}

// Implement MainWidget trait
use super::main_widget::{MainWidget, WidgetAction, event_in_bounds};
use crate::state::Focus;

impl MainWidget for ImmediateWindow {
    fn id(&self) -> &'static str {
        "immediate"
    }

    fn draw(&mut self, screen: &mut Screen, state: &AppState, bounds: Rect) {
        let has_focus = state.focus == Focus::Immediate;
        ImmediateWindow::draw(self, screen, state, bounds, has_focus);
    }

    fn handle_event(&mut self, event: &crate::input::InputEvent, state: &mut AppState, bounds: Rect) -> WidgetAction {
        use crate::input::InputEvent;

        match event {
            InputEvent::MouseClick { row, col } => {
                if !event_in_bounds(event, bounds) {
                    return WidgetAction::Ignored;
                }
                match self.handle_click(*row, *col, bounds, state.immediate_maximized) {
                    ImmediateClickResult::ToggleMaximize => WidgetAction::Toggle("immediate_maximized"),
                    ImmediateClickResult::StartResize => WidgetAction::StartDrag("immediate_resize"),
                    ImmediateClickResult::Focus => WidgetAction::SetFocus(Focus::Immediate),
                    ImmediateClickResult::None => WidgetAction::Consumed,
                }
            }
            _ if state.focus != Focus::Immediate => WidgetAction::Ignored,
            InputEvent::Escape => {
                WidgetAction::SetFocus(Focus::Editor)
            }
            _ => {
                if let Some(cmd) = self.handle_input(event, bounds) {
                    WidgetAction::ExecuteCommand(cmd)
                } else {
                    WidgetAction::Consumed
                }
            }
        }
    }

    fn handle_scroll(&mut self, event: &crate::input::InputEvent, bounds: Rect) -> WidgetAction {
        if event_in_bounds(event, bounds) {
            return WidgetAction::Consumed;
        }
        WidgetAction::Ignored
    }

    fn focusable(&self) -> bool {
        true
    }

    fn focus_type(&self) -> Option<Focus> {
        Some(Focus::Immediate)
    }
}
