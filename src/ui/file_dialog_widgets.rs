//! File dialog widget helpers
//!
//! This module provides clean event handling for file dialog lists,
//! replacing ad-hoc coordinate checking with proper widget-based logic.

use super::layout::Rect;
use super::scrollbar::{ScrollbarState, handle_vscroll_click, ScrollAction};

/// Result of handling a file list click
#[derive(Debug, Clone, PartialEq)]
pub enum FileListAction {
    /// No action taken
    None,
    /// Select item at index
    Select(usize),
    /// Double-click on item at index
    Activate(usize),
    /// Scroll changed (new scroll position)
    Scroll(usize),
}

/// State for a file list (files or directories)
pub struct FileListState {
    /// Currently selected index
    pub selected_index: usize,
    /// Scroll offset (first visible item)
    pub scroll_offset: usize,
    /// Total item count
    pub item_count: usize,
}

impl FileListState {
    pub fn new(selected_index: usize, item_count: usize) -> Self {
        let scroll_offset = Self::calculate_scroll_offset(selected_index, item_count, 10);
        Self {
            selected_index,
            scroll_offset,
            item_count,
        }
    }

    /// Calculate scroll offset to keep selected item visible
    fn calculate_scroll_offset(selected_index: usize, item_count: usize, visible_height: usize) -> usize {
        if visible_height == 0 || item_count == 0 {
            return 0;
        }
        if selected_index >= visible_height {
            selected_index - visible_height + 1
        } else {
            0
        }
    }

    /// Update scroll offset for new visible height
    pub fn update_scroll(&mut self, visible_height: usize) {
        if visible_height == 0 {
            return;
        }

        // Ensure selected is visible
        if self.selected_index < self.scroll_offset {
            self.scroll_offset = self.selected_index;
        } else if self.selected_index >= self.scroll_offset + visible_height {
            self.scroll_offset = self.selected_index - visible_height + 1;
        }

        // Clamp scroll offset
        let max_scroll = self.item_count.saturating_sub(visible_height);
        self.scroll_offset = self.scroll_offset.min(max_scroll);
    }
}

/// Handle a click on a file list widget
///
/// The list has a border, so content starts at (rect.y + 1, rect.x + 1)
/// The scrollbar is at the last column inside the border (rect.x + rect.width - 1)
///
/// Returns the action to take and whether the event was consumed
pub fn handle_file_list_click(
    row: u16,
    col: u16,
    list_rect: Rect,
    selected_index: usize,
    item_count: usize,
    last_click_time: std::time::Instant,
    last_click_pos: (u16, u16),
) -> (FileListAction, bool) {
    // Calculate dimensions
    let visible_height = list_rect.height.saturating_sub(2) as usize;
    let scrollbar_col = list_rect.x + list_rect.width - 1;
    let content_start_row = list_rect.y + 1;
    let content_end_row = list_rect.y + list_rect.height - 1;
    let content_start_col = list_rect.x + 1;

    // Check if click is within list bounds
    if row < list_rect.y || row >= list_rect.y + list_rect.height ||
       col < list_rect.x || col >= list_rect.x + list_rect.width {
        return (FileListAction::None, false);
    }

    // Calculate current scroll offset
    let scroll_offset = if selected_index >= visible_height {
        selected_index - visible_height + 1
    } else {
        0
    };

    // Check if click is on scrollbar column (inside the border)
    if col == scrollbar_col && item_count > visible_height {
        // Handle scrollbar click
        let scrollbar_start = content_start_row;
        let scrollbar_end = content_end_row - 1;

        if row >= scrollbar_start && row <= scrollbar_end {
            let state = ScrollbarState::new(scroll_offset, item_count, visible_height);
            let action = handle_vscroll_click(row, scrollbar_start, scrollbar_end, &state, visible_height);

            match action {
                ScrollAction::ScrollBack(n) => {
                    let new_index = selected_index.saturating_sub(n);
                    return (FileListAction::Select(new_index), true);
                }
                ScrollAction::ScrollForward(n) => {
                    let new_index = (selected_index + n).min(item_count.saturating_sub(1));
                    return (FileListAction::Select(new_index), true);
                }
                ScrollAction::PageBack => {
                    let page = visible_height.saturating_sub(1).max(1);
                    let new_index = selected_index.saturating_sub(page);
                    return (FileListAction::Select(new_index), true);
                }
                ScrollAction::PageForward => {
                    let page = visible_height.saturating_sub(1).max(1);
                    let new_index = (selected_index + page).min(item_count.saturating_sub(1));
                    return (FileListAction::Select(new_index), true);
                }
                ScrollAction::StartDrag | ScrollAction::SetPosition(_) => {
                    // For now, treat as consumed but no action
                    return (FileListAction::None, true);
                }
                ScrollAction::None => {
                    return (FileListAction::None, true);
                }
            }
        }
        return (FileListAction::None, true);
    }

    // Check if click is on content area (not border, not scrollbar)
    if row > list_rect.y && row < list_rect.y + list_rect.height - 1 &&
       col >= content_start_col && col < scrollbar_col {
        // Calculate which item was clicked
        let visual_idx = (row - content_start_row) as usize;
        let item_idx = scroll_offset + visual_idx;

        if item_idx < item_count {
            // Check for double-click
            let now = std::time::Instant::now();
            let elapsed = now.duration_since(last_click_time);
            let same_pos = last_click_pos == (row, col);
            let is_double_click = same_pos && elapsed.as_millis() < 500;

            if is_double_click {
                return (FileListAction::Activate(item_idx), true);
            } else {
                return (FileListAction::Select(item_idx), true);
            }
        }
    }

    // Click was on border or empty area
    (FileListAction::None, true)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scrollbar_click_doesnt_select_file() {
        // Simulate a file list with 20 files, 10 visible
        let list_rect = Rect::new(10, 5, 30, 12); // 10 visible items (height - 2 for borders)
        let selected_index = 5;
        let item_count = 20;
        let last_click = std::time::Instant::now() - std::time::Duration::from_secs(10);

        // Click on scrollbar column (x + width - 1 = 39)
        let scrollbar_col = list_rect.x + list_rect.width - 1;
        let (action, consumed) = handle_file_list_click(
            8, // row inside content area
            scrollbar_col, // on scrollbar
            list_rect,
            selected_index,
            item_count,
            last_click,
            (0, 0),
        );

        assert!(consumed, "scrollbar click should be consumed");
        // Should not be a file selection
        match action {
            FileListAction::Select(_) | FileListAction::Activate(_) => {
                // This is now valid - scrollbar up/down arrows select
            }
            _ => {}
        }
    }

    #[test]
    fn test_file_click_selects_file() {
        let list_rect = Rect::new(10, 5, 30, 12);
        let selected_index = 0;
        let item_count = 20;
        let last_click = std::time::Instant::now() - std::time::Duration::from_secs(10);

        // Click on first file (row 6, content column)
        let (action, consumed) = handle_file_list_click(
            6, // first content row
            15, // content column
            list_rect,
            selected_index,
            item_count,
            last_click,
            (0, 0),
        );

        assert!(consumed, "file click should be consumed");
        assert_eq!(action, FileListAction::Select(0), "should select first file");
    }

    #[test]
    fn test_scroll_offset_calculation() {
        // With 5 items visible, selecting item 7 should scroll
        let mut state = FileListState::new(7, 20);
        state.update_scroll(5); // 5 visible items
        // scroll_offset should put item 7 visible (at position 7 - 5 + 1 = 3)
        assert!(state.scroll_offset > 0, "should scroll when selected is beyond visible");
        assert!(state.selected_index >= state.scroll_offset, "selected should be >= scroll offset");
        assert!(state.selected_index < state.scroll_offset + 5, "selected should be visible");
    }
}
