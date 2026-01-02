//! QBasic Immediate window for direct BASIC expression evaluation
#![allow(dead_code)]

use crate::screen::Screen;
use crate::terminal::Color;
use crate::state::AppState;
use super::layout::{Rect, LayoutItem, compute_layout};

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

        // Maximize/Minimize button at right of title bar
        let button_x = col + width - 4;
        if state.immediate_maximized {
            // Show restore button when maximized (vertical up-down arrow)
            screen.write_str(row, button_x, "[↕]", Color::White, Color::Blue);
        } else {
            // Show maximize button when normal (up arrow)
            screen.write_str(row, button_x, "[↑]", Color::White, Color::Blue);
        }

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
        // Layout: [border][...title...][button][border]
        LayoutItem::hstack(vec![
            LayoutItem::leaf("left_border").fixed_width(1),
            LayoutItem::leaf("title").width(super::layout::Size::Flex(1)),
            LayoutItem::leaf("toggle_button").fixed_width(3), // [↑] or [↓]
            LayoutItem::leaf("right_border").fixed_width(1),
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

        // Check if click is on the title bar row
        if row == title_row {
            // Build and compute title bar layout
            let title_bounds = Rect {
                x: bounds.x + 1, // 1-based column
                y: title_row,
                width: bounds.width,
                height: 1,
            };
            let layout = compute_layout(&self.title_bar_layout(bounds), title_bounds);

            // Check which element was hit
            if let Some(hit_id) = layout.hit_test(row, col) {
                match hit_id.as_str() {
                    "toggle_button" => return ImmediateClickResult::ToggleMaximize,
                    "title" | "left_border" | "right_border" => {
                        if !is_maximized {
                            return ImmediateClickResult::StartResize;
                        }
                    }
                    _ => {}
                }
            }
        }

        // Click anywhere else in the window - focus it
        ImmediateClickResult::Focus
    }
}
