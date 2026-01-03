//! Editor widget helpers
#![allow(dead_code)]
//!
//! Provides clean event handling for the editor area, including scrollbars.

use super::layout::Rect;
use super::scrollbar::{ScrollbarState, handle_vscroll_click, handle_hscroll_click, ScrollAction};
use super::window_chrome;

/// Result of handling an editor click
#[derive(Debug, Clone, PartialEq)]
pub enum EditorClickAction {
    /// No action
    None,
    /// Click in content area at (line, col) relative to scroll position
    ContentClick { editor_y: usize, editor_x: usize },
    /// Vertical scroll action
    VScroll(ScrollAction),
    /// Horizontal scroll action
    HScroll(ScrollAction),
    /// Start vertical scrollbar drag
    StartVDrag,
    /// Start horizontal scrollbar drag
    StartHDrag,
    /// Toggle maximize state
    MaximizeToggle,
    /// Double-click on title bar to toggle maximize
    TitleBarDoubleClick,
}

/// Handle a click in the editor area
///
/// The editor has a border, vertical scrollbar on right, horizontal scrollbar on bottom.
/// Layout: [border][content][vscroll][border]
///         [border][hscroll area     ][border]
pub fn handle_editor_click(
    row: u16,
    col: u16,
    editor_rect: Rect,
    scroll_row: usize,
    scroll_col: usize,
    line_count: usize,
    max_line_len: usize,
    visible_lines: usize,
    visible_cols: usize,
) -> EditorClickAction {
    // Convert to 1-based screen coordinates (editor drawing uses 1-based)
    let editor_row = editor_rect.y + 1;
    let editor_col = editor_rect.x + 1;
    let editor_width = editor_rect.width;
    let editor_height = editor_rect.height;

    // Check for maximize button click (on title bar)
    if window_chrome::is_maximize_button_click(row, col, editor_row, editor_col, editor_width) {
        return EditorClickAction::MaximizeToggle;
    }

    // Scrollbar positions
    let vscroll_col = editor_col + editor_width - 1;
    let hscroll_row = editor_row + editor_height - 1;

    // Vertical scrollbar bounds
    let vscroll_start = editor_row + 1;
    let vscroll_end = hscroll_row - 1;

    // Check vertical scrollbar
    if col == vscroll_col && row >= vscroll_start && row <= vscroll_end {
        // Use visible_size=1 so max_scroll = line_count - 1, matching drawing and click handling
        let state = ScrollbarState::new(scroll_row, line_count, 1);
        let action = handle_vscroll_click(row, vscroll_start, vscroll_end, &state, visible_lines);

        return match action {
            ScrollAction::StartDrag => EditorClickAction::StartVDrag,
            other => EditorClickAction::VScroll(other),
        };
    }

    // Horizontal scrollbar bounds
    let hscroll_start = editor_col + 1;
    let hscroll_end = vscroll_col - 1;

    // Check horizontal scrollbar
    if row == hscroll_row && col >= hscroll_start && col <= hscroll_end {
        // Use visible_size=1 so max_scroll = max_line_len - 1, matching drawing and click handling
        let state = ScrollbarState::new(scroll_col, max_line_len, 1);
        let action = handle_hscroll_click(col, hscroll_start, hscroll_end, &state, visible_cols);

        return match action {
            ScrollAction::StartDrag => EditorClickAction::StartHDrag,
            other => EditorClickAction::HScroll(other),
        };
    }

    // Content area bounds
    let content_left = editor_col + 1;
    let content_right = vscroll_col - 1;
    let content_top = editor_row + 1;
    let content_bottom = hscroll_row - 1;

    // Check content area
    if row >= content_top && row < content_bottom && col >= content_left && col < content_right {
        let editor_y = (row - content_top) as usize;
        let editor_x = (col - content_left) as usize;
        return EditorClickAction::ContentClick { editor_y, editor_x };
    }

    EditorClickAction::None
}

/// Handle vertical scrollbar drag
pub fn handle_vscroll_drag(
    row: u16,
    editor_rect: Rect,
    line_count: usize,
    visible_lines: usize,
) -> usize {
    let editor_row = editor_rect.y + 1;
    let editor_height = editor_rect.height;
    let hscroll_row = editor_row + editor_height - 1;

    let vscroll_start = editor_row + 1;
    let vscroll_end = hscroll_row - 1;

    let state = ScrollbarState::new(0, line_count, visible_lines);
    super::scrollbar::drag_to_vscroll(row, vscroll_start, vscroll_end, &state)
}

/// Handle horizontal scrollbar drag
pub fn handle_hscroll_drag(
    col: u16,
    editor_rect: Rect,
    max_line_len: usize,
    visible_cols: usize,
) -> usize {
    let editor_col = editor_rect.x + 1;
    let editor_width = editor_rect.width;
    let vscroll_col = editor_col + editor_width - 1;

    let hscroll_start = editor_col + 1;
    let hscroll_end = vscroll_col - 1;

    let state = ScrollbarState::new(0, max_line_len, visible_cols);
    super::scrollbar::drag_to_hscroll(col, hscroll_start, hscroll_end, &state)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_content_click() {
        let rect = Rect::new(0, 0, 80, 25);
        let action = handle_editor_click(
            5, 10,  // row, col
            rect,
            0, 0,   // scroll position
            100, 80, // content size
            20, 70,  // visible size
        );

        match action {
            EditorClickAction::ContentClick { editor_y, editor_x } => {
                assert!(editor_y < 20);
                assert!(editor_x < 70);
            }
            _ => panic!("Expected ContentClick"),
        }
    }

    #[test]
    fn test_vscroll_click() {
        let rect = Rect::new(0, 0, 80, 25);
        // Click on right edge (vscroll column)
        let vscroll_col = 1 + 80 - 1; // editor_col + width - 1
        let action = handle_editor_click(
            3, vscroll_col as u16,
            rect,
            0, 0,
            100, 80,
            20, 70,
        );

        match action {
            EditorClickAction::VScroll(_) => {}
            _ => panic!("Expected VScroll action"),
        }
    }
}
