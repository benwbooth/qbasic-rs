//! Program output window for BASIC program execution

use crate::screen::Screen;
use crate::terminal::Color;
use crate::state::AppState;
use super::layout::Rect;

/// The output window for program execution (black background, white text)
pub struct OutputWindow {
    /// Output lines
    pub output: Vec<String>,
    /// Maximum output lines to keep
    pub max_output: usize,
    /// Scroll position (0 = showing latest lines)
    pub scroll: usize,
}

impl OutputWindow {
    pub fn new() -> Self {
        Self {
            output: Vec::new(),
            max_output: 1000,
            scroll: 0,
        }
    }

    /// Draw the output window in a bordered panel
    pub fn draw(&self, screen: &mut Screen, state: &AppState, bounds: Rect) {
        let row = bounds.y + 1; // 1-based row
        let col = bounds.x + 1;
        let width = bounds.width;
        let height = bounds.height;

        // Draw border with title
        let border_fg = Color::LightGray;
        screen.draw_box(row, col, width, height, border_fg, Color::Black);

        // Title with close button
        let title = " Output ";
        let title_x = col + 2;
        screen.write_str(row, title_x, title, border_fg, Color::Black);

        // Close button [X] at right of title bar
        let close_x = col + width - 4;
        screen.write_str(row, close_x, "[X]", Color::White, Color::Red);

        // Content area
        let content_row = row + 1;
        let content_width = width.saturating_sub(2);
        let content_height = height.saturating_sub(2);

        // Clear content area with black background
        for r in 0..content_height {
            for c in 0..content_width {
                screen.set(content_row + r, col + 1 + c, ' ', Color::White, Color::Black);
            }
        }

        // Show output lines
        let visible_lines = content_height as usize;
        let total_lines = self.output.len();

        // Calculate which lines to show based on scroll position
        let start_line = if total_lines > visible_lines {
            total_lines - visible_lines - self.scroll
        } else {
            0
        };

        for (i, line) in self.output.iter().skip(start_line).take(visible_lines).enumerate() {
            let display = if line.len() > content_width as usize {
                &line[..content_width as usize]
            } else {
                line.as_str()
            };
            screen.write_str(content_row + i as u16, col + 1, display, Color::White, Color::Black);
        }

        // Show "Press any key..." if program completed
        if state.run_state == crate::state::RunState::Editing && !self.output.is_empty() {
            let msg = " Press Escape to close ";
            let msg_x = col + (width.saturating_sub(msg.len() as u16)) / 2;
            screen.write_str(row + height - 1, msg_x, msg, Color::Yellow, Color::Black);
        }
    }

    /// Draw the output fullscreen (no borders, takes entire terminal)
    pub fn draw_fullscreen(&self, screen: &mut Screen, state: &AppState) {
        let (width, height) = screen.size();

        // Clear entire screen with black background
        for r in 1..=height {
            for c in 1..=width {
                screen.set(r, c, ' ', Color::White, Color::Black);
            }
        }

        // Show output lines
        let visible_lines = height as usize;
        let total_lines = self.output.len();

        // Calculate which lines to show based on scroll position
        let start_line = if total_lines > visible_lines {
            total_lines - visible_lines - self.scroll
        } else {
            0
        };

        for (i, line) in self.output.iter().skip(start_line).take(visible_lines).enumerate() {
            let display = if line.len() > width as usize {
                &line[..width as usize]
            } else {
                line.as_str()
            };
            screen.write_str(1 + i as u16, 1, display, Color::White, Color::Black);
        }

        // Show status at bottom if program completed
        if state.run_state == crate::state::RunState::Finished {
            let msg = " Press any key to continue ";
            // Draw on last line with highlight
            let msg_x = (width.saturating_sub(msg.len() as u16)) / 2 + 1;
            screen.write_str(height, msg_x, msg, Color::Black, Color::White);
        }

        // Hide cursor
        screen.set_cursor_visible(false);
    }

    /// Add output line
    pub fn add_output(&mut self, line: &str) {
        self.output.push(line.to_string());
        while self.output.len() > self.max_output {
            self.output.remove(0);
        }
        // Reset scroll to show latest
        self.scroll = 0;
    }

    /// Clear output
    pub fn clear(&mut self) {
        self.output.clear();
        self.scroll = 0;
    }

    /// Scroll up
    pub fn scroll_up(&mut self, lines: usize) {
        let max_scroll = self.output.len().saturating_sub(10); // Leave at least 10 lines visible
        self.scroll = (self.scroll + lines).min(max_scroll);
    }

    /// Scroll down
    pub fn scroll_down(&mut self, lines: usize) {
        self.scroll = self.scroll.saturating_sub(lines);
    }
}

impl Default for OutputWindow {
    fn default() -> Self {
        Self::new()
    }
}
