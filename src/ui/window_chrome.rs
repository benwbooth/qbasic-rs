//! Shared window chrome drawing and hit testing
//!
//! Provides consistent title bar rendering with maximize buttons across
//! floating windows, editor panes, and immediate window.

use crate::screen::Screen;
use crate::terminal::Color;

/// Width of the maximize button including brackets: [↑] or [↕]
pub const MAXIMIZE_BUTTON_WIDTH: u16 = 3;

/// Position of the maximize button relative to the right edge of the title bar
/// Button is 5 characters from the right corner to leave 2 chars space
pub const MAXIMIZE_BUTTON_OFFSET: u16 = 5;

/// Draw the maximize/restore button
/// Uses [↑] when not maximized (click to maximize)
/// Uses [↕] when maximized (click to restore)
pub fn draw_maximize_button(
    screen: &mut Screen,
    row: u16,
    col: u16,
    width: u16,
    maximized: bool,
    fg: Color,
    bg: Color,
) {
    let btn_col = col + width - MAXIMIZE_BUTTON_OFFSET;
    let btn_str = if maximized { "[↕]" } else { "[↑]" };
    screen.write_str(row, btn_col, btn_str, fg, bg);
}

/// Draw a title bar with centered title and maximize button
pub fn draw_title_bar(
    screen: &mut Screen,
    row: u16,
    col: u16,
    width: u16,
    title: &str,
    maximized: bool,
    fg: Color,
    bg: Color,
) {
    // Draw title (centered, leaving room for maximize button)
    let title_str = format!(" {} ", title);
    let available_width = width.saturating_sub(MAXIMIZE_BUTTON_OFFSET + 1);
    let title_x = col + (available_width.saturating_sub(title_str.len() as u16)) / 2;
    screen.write_str(row, title_x, &title_str, fg, bg);

    // Draw maximize/restore button
    draw_maximize_button(screen, row, col, width, maximized, fg, bg);
}

/// Check if a point is on the maximize button
pub fn is_maximize_button_click(
    row: u16,
    col: u16,
    title_row: u16,
    window_col: u16,
    window_width: u16,
) -> bool {
    let btn_col = window_col + window_width - MAXIMIZE_BUTTON_OFFSET;
    row == title_row && col >= btn_col && col < btn_col + MAXIMIZE_BUTTON_WIDTH
}

/// Check if a point is in the title bar (excluding maximize button)
pub fn is_title_bar_click(
    row: u16,
    col: u16,
    title_row: u16,
    window_col: u16,
    window_width: u16,
) -> bool {
    let btn_col = window_col + window_width - MAXIMIZE_BUTTON_OFFSET;
    row == title_row && col >= window_col && col < btn_col
}
