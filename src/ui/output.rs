//! Program output window for BASIC program execution

use crate::screen::Screen;
use crate::terminal::Color;
use crate::state::AppState;
use crate::basic::graphics::GraphicsMode;
use super::layout::{Rect, LayoutItem, compute_layout};

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
    #[allow(dead_code)]
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

    /// Draw the graphics text screen fullscreen
    pub fn draw_graphics_screen(&self, screen: &mut Screen, graphics: &GraphicsMode, state: &AppState) {
        let (term_width, term_height) = screen.size();

        // Check if in graphics mode (SCREEN 7, 12, 13, etc.)
        if graphics.is_graphics_mode() {
            // Render pixel graphics using sixel
            let scale = 1; // 1:1 pixel mapping
            let sixel_data = graphics.render_sixel(scale);
            screen.set_sixel(sixel_data);

            // Also render text overlay from the text screen buffer
            // This allows PRINT/LOCATE/INPUT to work in graphics mode
            for row in 1..=term_height.min(graphics.text_rows) {
                for col in 1..=term_width.min(graphics.text_cols) {
                    let cell = graphics.get_char(row, col);
                    // Only set non-space chars for overlay
                    if cell.char != ' ' {
                        let fg = dos_to_color(cell.fg);
                        let bg = dos_to_color(cell.bg);
                        screen.set(row, col, cell.char, fg, bg);
                    }
                }
            }

            // Show cursor if waiting for input
            if state.run_state == crate::state::RunState::WaitingForInput {
                screen.set_cursor(graphics.cursor_row, graphics.cursor_col);
                screen.set_cursor_visible(true);
            } else {
                screen.set_cursor_visible(false);
            }
            return;
        }

        // Text mode: Render each cell of the graphics text screen
        // The buffer should be sized to match the terminal, so this covers everything
        for row in 1..=term_height {
            for col in 1..=term_width {
                if row <= graphics.text_rows && col <= graphics.text_cols {
                    let cell = graphics.get_char(row, col);
                    let fg = dos_to_color(cell.fg);
                    let bg = dos_to_color(cell.bg);
                    screen.set(row, col, cell.char, fg, bg);
                } else {
                    // Fill any extra space with black
                    screen.set(row, col, ' ', Color::LightGray, Color::Black);
                }
            }
        }

        // Show status at bottom if program completed
        if state.run_state == crate::state::RunState::Finished {
            let msg = " Press any key to continue ";
            let msg_x = (term_width.saturating_sub(msg.len() as u16)) / 2 + 1;
            screen.write_str(term_height, msg_x, msg, Color::Black, Color::White);
        }

        // Show cursor if waiting for input (in text mode too)
        if state.run_state == crate::state::RunState::WaitingForInput {
            screen.set_cursor(graphics.cursor_row, graphics.cursor_col);
            screen.set_cursor_visible(true);
        } else {
            screen.set_cursor_visible(false);
        }
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
    #[allow(dead_code)]
    pub fn scroll_up(&mut self, lines: usize) {
        let max_scroll = self.output.len().saturating_sub(10); // Leave at least 10 lines visible
        self.scroll = (self.scroll + lines).min(max_scroll);
    }

    /// Scroll down
    #[allow(dead_code)]
    pub fn scroll_down(&mut self, lines: usize) {
        self.scroll = self.scroll.saturating_sub(lines);
    }
}

impl Default for OutputWindow {
    fn default() -> Self {
        Self::new()
    }
}

/// Result of handling an output window click
#[derive(Clone, Debug, PartialEq)]
pub enum OutputClickResult {
    /// No action taken
    None,
    /// Close the output window
    Close,
}

impl OutputWindow {
    /// Build a layout for the title bar (first row of the window)
    /// Creates regions for the close button
    pub fn title_bar_layout(&self, bounds: Rect) -> LayoutItem {
        // Title bar is the first row inside the window
        // Layout: [border][title...][close_button][border]
        LayoutItem::hstack(vec![
            LayoutItem::leaf("left_border").fixed_width(1),
            LayoutItem::leaf("title").width(super::layout::Size::Flex(1)),
            LayoutItem::leaf("close_button").fixed_width(3), // [X]
            LayoutItem::leaf("right_border").fixed_width(1),
        ])
        .fixed_height(1)
        .fixed_width(bounds.width)
    }

    /// Handle a click on the output window
    /// bounds is the output window rect from the main layout (0-based)
    /// row, col are 1-based screen coordinates
    pub fn handle_click(&self, row: u16, col: u16, bounds: Rect) -> OutputClickResult {
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
                if hit_id == "close_button" {
                    return OutputClickResult::Close;
                }
            }
        }

        OutputClickResult::None
    }
}

/// Convert DOS color (0-15) to terminal color
fn dos_to_color(color: u8) -> Color {
    match color & 0x0F {
        0 => Color::Black,
        1 => Color::Blue,
        2 => Color::Green,
        3 => Color::Cyan,
        4 => Color::Red,
        5 => Color::Magenta,
        6 => Color::Brown,
        7 => Color::LightGray,
        8 => Color::DarkGray,
        9 => Color::LightBlue,
        10 => Color::LightGreen,
        11 => Color::LightCyan,
        12 => Color::LightRed,
        13 => Color::LightMagenta,
        14 => Color::Yellow,
        15 => Color::White,
        _ => Color::White,
    }
}

// Implement MainWidget trait
use super::main_widget::{MainWidget, WidgetAction, event_in_bounds};
use crate::state::Focus;

impl MainWidget for OutputWindow {
    fn id(&self) -> &'static str {
        "output"
    }

    fn draw(&mut self, screen: &mut Screen, state: &AppState, bounds: Rect) {
        OutputWindow::draw(self, screen, state, bounds);
    }

    fn handle_event(&mut self, event: &crate::input::InputEvent, _state: &mut AppState, bounds: Rect) -> WidgetAction {
        use crate::input::InputEvent;

        if let InputEvent::MouseClick { row, col } = event {
            if event_in_bounds(event, bounds) {
                match self.handle_click(*row, *col, bounds) {
                    OutputClickResult::Close => return WidgetAction::Toggle("show_output"),
                    OutputClickResult::None => return WidgetAction::Consumed,
                }
            }
        }

        WidgetAction::Ignored
    }

    fn handle_scroll(&mut self, event: &crate::input::InputEvent, bounds: Rect) -> WidgetAction {
        use crate::input::InputEvent;

        if !event_in_bounds(event, bounds) {
            return WidgetAction::Ignored;
        }

        match event {
            InputEvent::ScrollUp { .. } => {
                self.scroll_up(3);
                WidgetAction::Consumed
            }
            InputEvent::ScrollDown { .. } => {
                self.scroll_down(3);
                WidgetAction::Consumed
            }
            _ => WidgetAction::Ignored,
        }
    }

    fn focusable(&self) -> bool {
        false
    }

    fn focus_type(&self) -> Option<Focus> {
        None
    }
}
