//! QBasic Immediate window for direct BASIC expression evaluation
#![allow(dead_code)]

use crate::screen::Screen;
use crate::terminal::Color;
use crate::state::AppState;
use super::layout::{Rect, LayoutItem};
use super::window_chrome;

/// The immediate window at the bottom
pub struct ImmediateWindow {
    /// Command history
    pub history: Vec<String>,
    /// Current input line
    pub input: String,
    /// Cursor position in input
    pub cursor: usize,
    /// History navigation position
    pub history_pos: Option<usize>,
    /// Output lines
    pub output: Vec<String>,
    /// Maximum output lines to keep
    pub max_output: usize,
}

impl ImmediateWindow {
    pub fn new() -> Self {
        Self {
            history: Vec::new(),
            input: String::new(),
            cursor: 0,
            history_pos: None,
            output: Vec::new(),
            max_output: 100,
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

        // Maximize/Minimize button at right of title bar (shared code)
        window_chrome::draw_maximize_button(screen, row, col, width, state.immediate_maximized, border_fg, Color::Blue);

        // Content area
        let content_row = row + 1;
        let content_width = width.saturating_sub(2);
        let content_height = height.saturating_sub(2);

        // Clear content area
        for r in 0..content_height {
            for c in 0..content_width {
                screen.set(content_row + r, col + 1 + c, ' ', Color::Yellow, Color::Blue);
            }
        }

        // Show recent output (all but last line)
        let output_lines = content_height.saturating_sub(1) as usize;
        let output_start = self.output.len().saturating_sub(output_lines);
        for (i, line) in self.output.iter().skip(output_start).take(output_lines).enumerate() {
            let display = if line.len() > content_width as usize - 1 {
                &line[..content_width as usize - 1]
            } else {
                line.as_str()
            };
            screen.write_str(content_row + i as u16, col + 1, display, Color::Yellow, Color::Blue);
        }

        // Draw input line at bottom of content area
        let input_row = content_row + content_height - 1;
        let prompt = "? ";
        screen.write_str(input_row, col + 1, prompt, Color::White, Color::Blue);

        // Input text
        let input_start = col + 1 + prompt.len() as u16;
        let max_input_width = content_width - prompt.len() as u16;

        // Handle scrolling for long input
        let display_start = if self.cursor > max_input_width as usize - 1 {
            self.cursor - (max_input_width as usize - 1)
        } else {
            0
        };

        let display_text: String = self.input.chars().skip(display_start).take(max_input_width as usize).collect();
        screen.write_str(input_row, input_start, &display_text, Color::Yellow, Color::Blue);

        // Cursor position
        if has_focus {
            let cursor_x = input_start + (self.cursor - display_start) as u16;
            screen.set_cursor(input_row, cursor_x);
            screen.set_cursor_style(crate::terminal::CursorStyle::BlinkingUnderline);
            screen.set_cursor_visible(true);
        }
    }

    /// Handle input for the immediate window
    pub fn handle_input(&mut self, event: &crate::input::InputEvent) -> Option<String> {
        use crate::input::InputEvent;

        match event {
            InputEvent::Char(c) => {
                self.input.insert(self.cursor, *c);
                self.cursor += 1;
                self.history_pos = None;
                None
            }
            InputEvent::Enter => {
                if !self.input.is_empty() {
                    let cmd = self.input.clone();
                    self.history.push(cmd.clone());
                    self.input.clear();
                    self.cursor = 0;
                    self.history_pos = None;
                    Some(cmd)
                } else {
                    None
                }
            }
            InputEvent::Backspace => {
                if self.cursor > 0 {
                    self.cursor -= 1;
                    self.input.remove(self.cursor);
                }
                None
            }
            InputEvent::Delete => {
                if self.cursor < self.input.len() {
                    self.input.remove(self.cursor);
                }
                None
            }
            InputEvent::CursorLeft => {
                if self.cursor > 0 {
                    self.cursor -= 1;
                }
                None
            }
            InputEvent::CursorRight => {
                if self.cursor < self.input.len() {
                    self.cursor += 1;
                }
                None
            }
            InputEvent::Home => {
                self.cursor = 0;
                None
            }
            InputEvent::End => {
                self.cursor = self.input.len();
                None
            }
            InputEvent::CursorUp => {
                // History navigation
                if !self.history.is_empty() {
                    match self.history_pos {
                        None => {
                            self.history_pos = Some(self.history.len() - 1);
                        }
                        Some(0) => {}
                        Some(pos) => {
                            self.history_pos = Some(pos - 1);
                        }
                    }
                    if let Some(pos) = self.history_pos {
                        self.input = self.history[pos].clone();
                        self.cursor = self.input.len();
                    }
                }
                None
            }
            InputEvent::CursorDown => {
                // History navigation
                if let Some(pos) = self.history_pos {
                    if pos + 1 < self.history.len() {
                        self.history_pos = Some(pos + 1);
                        self.input = self.history[pos + 1].clone();
                        self.cursor = self.input.len();
                    } else {
                        self.history_pos = None;
                        self.input.clear();
                        self.cursor = 0;
                    }
                }
                None
            }
            _ => None,
        }
    }

    /// Add output line
    pub fn add_output(&mut self, line: &str) {
        self.output.push(line.to_string());
        while self.output.len() > self.max_output {
            self.output.remove(0);
        }
    }

    /// Clear output
    #[allow(dead_code)]
    pub fn clear(&mut self) {
        self.output.clear();
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
    /// Creates regions for the toggle button and border
    pub fn title_bar_layout(&self, bounds: Rect) -> LayoutItem {
        // Title bar is the first row inside the window
        // Layout: [border][...title...][button][border] (matches window_chrome layout)
        LayoutItem::hstack(vec![
            LayoutItem::leaf("left_border").fixed_width(1),
            LayoutItem::leaf("title").width(super::layout::Size::Flex(1)),
            LayoutItem::leaf("toggle_button").fixed_width(3), // [↑] or [↕]
            LayoutItem::leaf("right_border").fixed_width(2),  // 2 chars space to corner
        ])
        .fixed_height(1)
        .fixed_width(bounds.width)
    }

    /// Handle a click on the immediate window
    /// bounds is the immediate window rect from the main layout (0-based)
    /// row, col are 1-based screen coordinates
    pub fn handle_click(&self, row: u16, col: u16, bounds: Rect, is_maximized: bool) -> ImmediateClickResult {
        // Convert bounds to 1-based
        let title_row = bounds.y + 1;
        let window_col = bounds.x + 1;

        // Check if click is on the title bar row
        if row == title_row {
            // Use shared hit testing for maximize button
            if window_chrome::is_maximize_button_click(row, col, title_row, window_col, bounds.width) {
                return ImmediateClickResult::ToggleMaximize;
            }
            // Click on title bar (not on button) - start resize if not maximized
            if window_chrome::is_title_bar_click(row, col, title_row, window_col, bounds.width) && !is_maximized {
                return ImmediateClickResult::StartResize;
            }
        }

        // Click anywhere else in the window - focus it
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
        // Delegate to existing draw method
        let has_focus = state.focus == Focus::Immediate;
        ImmediateWindow::draw(self, screen, state, bounds, has_focus);
    }

    fn handle_event(&mut self, event: &crate::input::InputEvent, state: &mut AppState, bounds: Rect) -> WidgetAction {
        use crate::input::InputEvent;

        // Only handle events if we have focus (for keyboard) or event is in bounds (for mouse)
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
                // Escape returns to editor
                WidgetAction::SetFocus(Focus::Editor)
            }
            _ => {
                // Handle keyboard input when focused
                if let Some(cmd) = self.handle_input(event) {
                    WidgetAction::ExecuteCommand(cmd)
                } else {
                    WidgetAction::Consumed
                }
            }
        }
    }

    fn handle_scroll(&mut self, event: &crate::input::InputEvent, bounds: Rect) -> WidgetAction {
        // Immediate window doesn't scroll (yet), but absorb scroll events within bounds
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
