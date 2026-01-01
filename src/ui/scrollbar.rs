//! Shared scrollbar drawing and interaction logic

use crate::screen::Screen;
use crate::terminal::Color;

/// State needed to render and interact with a scrollbar
#[derive(Clone, Debug)]
pub struct ScrollbarState {
    /// Current scroll position (0-based)
    pub scroll_pos: usize,
    /// Total content size (lines for vertical, columns for horizontal)
    pub content_size: usize,
    /// Visible size (visible lines for vertical, visible columns for horizontal)
    pub visible_size: usize,
}

impl ScrollbarState {
    pub fn new(scroll_pos: usize, content_size: usize, visible_size: usize) -> Self {
        Self {
            scroll_pos,
            content_size,
            visible_size,
        }
    }

    /// Maximum scroll position (content_size - visible_size)
    pub fn max_scroll(&self) -> usize {
        self.content_size.saturating_sub(self.visible_size)
    }

    /// Calculate thumb position within track (0 to track_size-1)
    pub fn thumb_pos(&self, track_size: usize) -> usize {
        if track_size == 0 {
            return 0;
        }
        // Simple direct mapping: scroll 0 = thumb at 0, scroll max = thumb at end
        let max_scroll = self.content_size.saturating_sub(1);
        if max_scroll == 0 {
            return 0;
        }
        let pos = (self.scroll_pos.min(max_scroll) * (track_size.saturating_sub(1))) / max_scroll;
        pos.min(track_size.saturating_sub(1))
    }
}

/// Result of clicking on a scrollbar
#[derive(Clone, Debug, PartialEq)]
pub enum ScrollAction {
    /// Scroll up/left by N units
    ScrollBack(usize),
    /// Scroll down/right by N units
    ScrollForward(usize),
    /// Page up/left
    PageBack,
    /// Page down/right
    PageForward,
    /// Start dragging the thumb
    StartDrag,
    /// Set scroll position directly (from drag)
    SetPosition(usize),
    /// No action (click outside scrollbar)
    None,
}

/// Scrollbar colors
#[derive(Clone, Copy)]
pub struct ScrollbarColors {
    pub track_fg: Color,
    pub track_bg: Color,
    pub arrow_fg: Color,
    pub arrow_bg: Color,
    pub thumb_fg: Color,
    pub thumb_bg: Color,
}

impl Default for ScrollbarColors {
    fn default() -> Self {
        // QBasic-style blue scrollbar
        Self {
            track_fg: Color::LightGray,
            track_bg: Color::Blue,
            arrow_fg: Color::Black,
            arrow_bg: Color::LightGray,
            thumb_fg: Color::Black,
            thumb_bg: Color::Blue,
        }
    }
}

impl ScrollbarColors {
    /// Dark theme scrollbar (for help dialog)
    pub fn dark() -> Self {
        Self {
            track_fg: Color::DarkGray,
            track_bg: Color::Black,
            arrow_fg: Color::LightGray,
            arrow_bg: Color::DarkGray,
            thumb_fg: Color::Cyan,
            thumb_bg: Color::Black,
        }
    }
}

/// Draw a vertical scrollbar
///
/// * `col` - Column to draw the scrollbar
/// * `start_row` - First row (will contain up arrow)
/// * `end_row` - Last row (will contain down arrow)
/// * `state` - Scrollbar state
/// * `colors` - Colors to use
pub fn draw_vertical(
    screen: &mut Screen,
    col: u16,
    start_row: u16,
    end_row: u16,
    state: &ScrollbarState,
    colors: &ScrollbarColors,
) {
    let height = end_row.saturating_sub(start_row) + 1;
    if height < 3 {
        return; // Not enough space
    }

    // Draw up arrow
    screen.set(start_row, col, '↑', colors.arrow_fg, colors.arrow_bg);

    // Draw down arrow
    screen.set(end_row, col, '↓', colors.arrow_fg, colors.arrow_bg);

    // Draw track (between arrows)
    for r in (start_row + 1)..end_row {
        screen.set(r, col, '░', colors.track_fg, colors.track_bg);
    }

    // Draw thumb if there's any content
    let track_size = (height.saturating_sub(2)) as usize;
    if track_size >= 1 && state.content_size >= 1 {
        let thumb_pos = state.thumb_pos(track_size);
        let thumb_row = start_row + 1 + thumb_pos as u16;
        if thumb_row < end_row {
            screen.set(thumb_row, col, '█', colors.thumb_fg, colors.thumb_bg);
        }
    }
}

/// Draw a horizontal scrollbar
///
/// * `row` - Row to draw the scrollbar
/// * `start_col` - First column (will contain left arrow)
/// * `end_col` - Last column (will contain right arrow)
/// * `state` - Scrollbar state
/// * `colors` - Colors to use
pub fn draw_horizontal(
    screen: &mut Screen,
    row: u16,
    start_col: u16,
    end_col: u16,
    state: &ScrollbarState,
    colors: &ScrollbarColors,
) {
    let width = end_col.saturating_sub(start_col) + 1;
    if width < 3 {
        return; // Not enough space
    }

    // Draw left arrow
    screen.set(row, start_col, '←', colors.arrow_fg, colors.arrow_bg);

    // Draw right arrow
    screen.set(row, end_col, '→', colors.arrow_fg, colors.arrow_bg);

    // Draw track (between arrows)
    for c in (start_col + 1)..end_col {
        screen.set(row, c, '░', colors.track_fg, colors.track_bg);
    }

    // Draw thumb if there's any content
    let track_size = (width.saturating_sub(2)) as usize;
    if track_size >= 1 && state.content_size >= 1 {
        let thumb_pos = state.thumb_pos(track_size);
        let thumb_col = start_col + 1 + thumb_pos as u16;
        if thumb_col < end_col {
            screen.set(row, thumb_col, '█', colors.thumb_fg, colors.thumb_bg);
        }
    }
}

/// Handle click on vertical scrollbar
///
/// * `click_row` - Row that was clicked
/// * `start_row` - First row of scrollbar (up arrow)
/// * `end_row` - Last row of scrollbar (down arrow)
/// * `state` - Current scrollbar state
/// * `_page_size` - How many lines to scroll for page up/down (unused, for API consistency)
pub fn handle_vscroll_click(
    click_row: u16,
    start_row: u16,
    end_row: u16,
    state: &ScrollbarState,
    _page_size: usize,
) -> ScrollAction {
    if click_row < start_row || click_row > end_row {
        return ScrollAction::None;
    }

    // Up arrow
    if click_row == start_row {
        return ScrollAction::ScrollBack(1);
    }

    // Down arrow
    if click_row == end_row {
        return ScrollAction::ScrollForward(1);
    }

    // Click on track
    let track_size = (end_row.saturating_sub(start_row).saturating_sub(1)) as usize;
    if track_size < 1 {
        return ScrollAction::None;
    }

    let thumb_pos = state.thumb_pos(track_size);
    let thumb_row = start_row + 1 + thumb_pos as u16;

    if click_row == thumb_row {
        ScrollAction::StartDrag
    } else if click_row < thumb_row {
        ScrollAction::PageBack
    } else {
        ScrollAction::PageForward
    }
}

/// Handle click on horizontal scrollbar
///
/// * `click_col` - Column that was clicked
/// * `start_col` - First column of scrollbar (left arrow)
/// * `end_col` - Last column of scrollbar (right arrow)
/// * `state` - Current scrollbar state
/// * `_page_size` - How many columns to scroll for page left/right (unused, for API consistency)
pub fn handle_hscroll_click(
    click_col: u16,
    start_col: u16,
    end_col: u16,
    state: &ScrollbarState,
    _page_size: usize,
) -> ScrollAction {
    if click_col < start_col || click_col > end_col {
        return ScrollAction::None;
    }

    // Left arrow
    if click_col == start_col {
        return ScrollAction::ScrollBack(1);
    }

    // Right arrow
    if click_col == end_col {
        return ScrollAction::ScrollForward(1);
    }

    // Click on track
    let track_size = (end_col.saturating_sub(start_col).saturating_sub(1)) as usize;
    if track_size < 1 {
        return ScrollAction::None;
    }

    let thumb_pos = state.thumb_pos(track_size);
    let thumb_col = start_col + 1 + thumb_pos as u16;

    if click_col == thumb_col {
        ScrollAction::StartDrag
    } else if click_col < thumb_col {
        ScrollAction::PageBack
    } else {
        ScrollAction::PageForward
    }
}

/// Calculate new scroll position from drag position on vertical scrollbar
///
/// * `drag_row` - Current mouse row during drag
/// * `start_row` - First row of scrollbar track (after up arrow)
/// * `end_row` - Last row of scrollbar track (before down arrow)
/// * `state` - Current scrollbar state
pub fn drag_to_vscroll(
    drag_row: u16,
    start_row: u16,
    end_row: u16,
    state: &ScrollbarState,
) -> usize {
    // Track is between arrows: from start_row+1 to end_row-1 inclusive
    let track_start = start_row + 1;
    // Track size must match draw_vertical: height - 2 = (end_row - start_row + 1) - 2
    let track_size = end_row.saturating_sub(start_row).saturating_sub(1) as usize;

    if track_size <= 1 || state.content_size <= 1 {
        return 0;
    }

    let track_pos = drag_row.saturating_sub(track_start) as usize;
    // Use content_size - 1 as max scroll (matches thumb_pos calculation)
    let max_scroll = state.content_size.saturating_sub(1);

    // Map track position to scroll position (inverse of thumb_pos)
    (track_pos * max_scroll / (track_size - 1)).min(max_scroll)
}

/// Calculate new scroll position from drag position on horizontal scrollbar
///
/// * `drag_col` - Current mouse column during drag
/// * `start_col` - First column of scrollbar track (after left arrow)
/// * `end_col` - Last column of scrollbar track (before right arrow)
/// * `state` - Current scrollbar state
pub fn drag_to_hscroll(
    drag_col: u16,
    start_col: u16,
    end_col: u16,
    state: &ScrollbarState,
) -> usize {
    // Track is between arrows: from start_col+1 to end_col-1 inclusive
    let track_start = start_col + 1;
    // Track size must match draw_horizontal: width - 2 = (end_col - start_col + 1) - 2
    let track_size = end_col.saturating_sub(start_col).saturating_sub(1) as usize;

    if track_size <= 1 || state.content_size <= 1 {
        return 0;
    }

    let track_pos = drag_col.saturating_sub(track_start) as usize;
    // Use content_size - 1 as max scroll (matches thumb_pos calculation)
    let max_scroll = state.content_size.saturating_sub(1);

    // Map track position to scroll position (inverse of thumb_pos)
    (track_pos * max_scroll / (track_size - 1)).min(max_scroll)
}

// ============================================================================
// Widget implementations
// ============================================================================

use crate::input::InputEvent;
use super::layout::Rect;
use super::widget::{Widget, EventResult, mouse_position};

/// A vertical scrollbar widget
pub struct VerticalScrollbar {
    /// Current scroll position
    scroll_pos: usize,
    /// Total content size
    content_size: usize,
    /// Visible size
    visible_size: usize,
    /// Colors
    colors: ScrollbarColors,
    /// Whether currently dragging
    dragging: bool,
    /// Action prefix for events (e.g., "files_list" -> "files_list_scroll")
    action_prefix: String,
}

impl VerticalScrollbar {
    pub fn new(action_prefix: impl Into<String>) -> Self {
        Self {
            scroll_pos: 0,
            content_size: 0,
            visible_size: 0,
            colors: ScrollbarColors::default(),
            dragging: false,
            action_prefix: action_prefix.into(),
        }
    }

    pub fn with_colors(mut self, colors: ScrollbarColors) -> Self {
        self.colors = colors;
        self
    }

    /// Update the scrollbar state
    pub fn update(&mut self, scroll_pos: usize, content_size: usize, visible_size: usize) {
        self.scroll_pos = scroll_pos;
        self.content_size = content_size;
        self.visible_size = visible_size;
    }

    /// Get current scroll position
    pub fn scroll_pos(&self) -> usize {
        self.scroll_pos
    }

    /// Set scroll position (clamped to valid range)
    pub fn set_scroll_pos(&mut self, pos: usize) {
        let max = self.content_size.saturating_sub(self.visible_size);
        self.scroll_pos = pos.min(max);
    }

    /// Scroll up by n items
    pub fn scroll_up(&mut self, n: usize) {
        self.scroll_pos = self.scroll_pos.saturating_sub(n);
    }

    /// Scroll down by n items
    pub fn scroll_down(&mut self, n: usize) {
        let max = self.content_size.saturating_sub(self.visible_size);
        self.scroll_pos = (self.scroll_pos + n).min(max);
    }

    /// Page up
    pub fn page_up(&mut self) {
        self.scroll_up(self.visible_size.saturating_sub(1).max(1));
    }

    /// Page down
    pub fn page_down(&mut self) {
        self.scroll_down(self.visible_size.saturating_sub(1).max(1));
    }

    /// Check if scrollbar is needed (content exceeds visible size)
    pub fn is_needed(&self) -> bool {
        self.content_size > self.visible_size
    }

    /// Start dragging
    pub fn start_drag(&mut self) {
        self.dragging = true;
    }

    /// Stop dragging
    pub fn stop_drag(&mut self) {
        self.dragging = false;
    }

    /// Is currently dragging?
    pub fn is_dragging(&self) -> bool {
        self.dragging
    }

    fn state(&self) -> ScrollbarState {
        ScrollbarState::new(self.scroll_pos, self.content_size, self.visible_size)
    }
}

impl Widget for VerticalScrollbar {
    fn draw(&self, screen: &mut Screen, bounds: Rect) {
        if bounds.height < 3 || bounds.width < 1 {
            return;
        }

        let start_row = bounds.y;
        let end_row = bounds.y + bounds.height - 1;
        let col = bounds.x;

        draw_vertical(screen, col, start_row, end_row, &self.state(), &self.colors);
    }

    fn handle_event(&mut self, event: &InputEvent, bounds: Rect) -> EventResult {
        // Handle drag continuation regardless of position
        if self.dragging {
            match event {
                InputEvent::MouseDrag { row, .. } => {
                    let start_row = bounds.y;
                    let end_row = bounds.y + bounds.height - 1;
                    let new_pos = drag_to_vscroll(*row, start_row, end_row, &self.state());
                    self.scroll_pos = new_pos;
                    return EventResult::Action(format!("{}_scroll", self.action_prefix));
                }
                InputEvent::MouseRelease { .. } => {
                    self.dragging = false;
                    return EventResult::Consumed;
                }
                _ => {}
            }
        }

        // Check if click is within bounds
        let (row, col) = match mouse_position(event) {
            Some(pos) => pos,
            None => return EventResult::Ignored,
        };

        if !bounds.contains(row, col) {
            return EventResult::Ignored;
        }

        // Only handle clicks
        if !matches!(event, InputEvent::MouseClick { .. }) {
            return EventResult::Ignored;
        }

        let start_row = bounds.y;
        let end_row = bounds.y + bounds.height - 1;

        let action = handle_vscroll_click(row, start_row, end_row, &self.state(), self.visible_size);

        match action {
            ScrollAction::ScrollBack(n) => {
                self.scroll_up(n);
                EventResult::Action(format!("{}_scroll", self.action_prefix))
            }
            ScrollAction::ScrollForward(n) => {
                self.scroll_down(n);
                EventResult::Action(format!("{}_scroll", self.action_prefix))
            }
            ScrollAction::PageBack => {
                self.page_up();
                EventResult::Action(format!("{}_scroll", self.action_prefix))
            }
            ScrollAction::PageForward => {
                self.page_down();
                EventResult::Action(format!("{}_scroll", self.action_prefix))
            }
            ScrollAction::StartDrag => {
                self.dragging = true;
                EventResult::Consumed
            }
            ScrollAction::SetPosition(pos) => {
                self.scroll_pos = pos;
                EventResult::Action(format!("{}_scroll", self.action_prefix))
            }
            ScrollAction::None => EventResult::Ignored,
        }
    }
}

/// A horizontal scrollbar widget
pub struct HorizontalScrollbar {
    /// Current scroll position
    scroll_pos: usize,
    /// Total content size
    content_size: usize,
    /// Visible size
    visible_size: usize,
    /// Colors
    colors: ScrollbarColors,
    /// Whether currently dragging
    dragging: bool,
    /// Action prefix for events
    action_prefix: String,
}

impl HorizontalScrollbar {
    pub fn new(action_prefix: impl Into<String>) -> Self {
        Self {
            scroll_pos: 0,
            content_size: 0,
            visible_size: 0,
            colors: ScrollbarColors::default(),
            dragging: false,
            action_prefix: action_prefix.into(),
        }
    }

    pub fn with_colors(mut self, colors: ScrollbarColors) -> Self {
        self.colors = colors;
        self
    }

    /// Update the scrollbar state
    pub fn update(&mut self, scroll_pos: usize, content_size: usize, visible_size: usize) {
        self.scroll_pos = scroll_pos;
        self.content_size = content_size;
        self.visible_size = visible_size;
    }

    /// Get current scroll position
    pub fn scroll_pos(&self) -> usize {
        self.scroll_pos
    }

    /// Set scroll position (clamped to valid range)
    pub fn set_scroll_pos(&mut self, pos: usize) {
        let max = self.content_size.saturating_sub(self.visible_size);
        self.scroll_pos = pos.min(max);
    }

    /// Scroll left by n items
    pub fn scroll_left(&mut self, n: usize) {
        self.scroll_pos = self.scroll_pos.saturating_sub(n);
    }

    /// Scroll right by n items
    pub fn scroll_right(&mut self, n: usize) {
        let max = self.content_size.saturating_sub(self.visible_size);
        self.scroll_pos = (self.scroll_pos + n).min(max);
    }

    fn state(&self) -> ScrollbarState {
        ScrollbarState::new(self.scroll_pos, self.content_size, self.visible_size)
    }
}

impl Widget for HorizontalScrollbar {
    fn draw(&self, screen: &mut Screen, bounds: Rect) {
        if bounds.width < 3 || bounds.height < 1 {
            return;
        }

        let row = bounds.y;
        let start_col = bounds.x;
        let end_col = bounds.x + bounds.width - 1;

        draw_horizontal(screen, row, start_col, end_col, &self.state(), &self.colors);
    }

    fn handle_event(&mut self, event: &InputEvent, bounds: Rect) -> EventResult {
        // Handle drag continuation regardless of position
        if self.dragging {
            match event {
                InputEvent::MouseDrag { col, .. } => {
                    let start_col = bounds.x;
                    let end_col = bounds.x + bounds.width - 1;
                    let new_pos = drag_to_hscroll(*col, start_col, end_col, &self.state());
                    self.scroll_pos = new_pos;
                    return EventResult::Action(format!("{}_scroll", self.action_prefix));
                }
                InputEvent::MouseRelease { .. } => {
                    self.dragging = false;
                    return EventResult::Consumed;
                }
                _ => {}
            }
        }

        // Check if click is within bounds
        let (row, col) = match mouse_position(event) {
            Some(pos) => pos,
            None => return EventResult::Ignored,
        };

        if !bounds.contains(row, col) {
            return EventResult::Ignored;
        }

        // Only handle clicks
        if !matches!(event, InputEvent::MouseClick { .. }) {
            return EventResult::Ignored;
        }

        let start_col = bounds.x;
        let end_col = bounds.x + bounds.width - 1;

        let action = handle_hscroll_click(col, start_col, end_col, &self.state(), self.visible_size);

        match action {
            ScrollAction::ScrollBack(n) => {
                self.scroll_left(n);
                EventResult::Action(format!("{}_scroll", self.action_prefix))
            }
            ScrollAction::ScrollForward(n) => {
                self.scroll_right(n);
                EventResult::Action(format!("{}_scroll", self.action_prefix))
            }
            ScrollAction::PageBack => {
                self.scroll_left(self.visible_size.saturating_sub(1).max(1));
                EventResult::Action(format!("{}_scroll", self.action_prefix))
            }
            ScrollAction::PageForward => {
                self.scroll_right(self.visible_size.saturating_sub(1).max(1));
                EventResult::Action(format!("{}_scroll", self.action_prefix))
            }
            ScrollAction::StartDrag => {
                self.dragging = true;
                EventResult::Consumed
            }
            ScrollAction::SetPosition(pos) => {
                self.scroll_pos = pos;
                EventResult::Action(format!("{}_scroll", self.action_prefix))
            }
            ScrollAction::None => EventResult::Ignored,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_max_scroll() {
        // 100 lines of content, 20 visible -> max scroll is 80
        let state = ScrollbarState::new(0, 100, 20);
        assert_eq!(state.max_scroll(), 80);

        // 50 lines, 20 visible -> max scroll is 30
        let state = ScrollbarState::new(0, 50, 20);
        assert_eq!(state.max_scroll(), 30);

        // 20 lines, 20 visible -> max scroll is 0 (no scrolling needed)
        let state = ScrollbarState::new(0, 20, 20);
        assert_eq!(state.max_scroll(), 0);

        // 10 lines, 20 visible -> max scroll is 0 (content fits)
        let state = ScrollbarState::new(0, 10, 20);
        assert_eq!(state.max_scroll(), 0);
    }

    #[test]
    fn test_thumb_pos_no_scroll_needed() {
        // When content fits in visible area, thumb_pos should not panic
        // and should return 0
        let state = ScrollbarState::new(0, 10, 20); // content smaller than visible
        let thumb_pos = state.thumb_pos(17);
        assert_eq!(thumb_pos, 0);

        let state = ScrollbarState::new(0, 20, 20); // content equals visible
        let thumb_pos = state.thumb_pos(17);
        assert_eq!(thumb_pos, 0);
    }

    #[test]
    fn test_thumb_at_max_scroll_is_at_bottom() {
        // 100 lines, 20 visible, scrolled to max (80)
        let state = ScrollbarState::new(80, 100, 20);
        let track_size = 18; // typical track size
        let thumb_pos = state.thumb_pos(track_size);
        // Thumb should be at the bottom of the track
        assert_eq!(thumb_pos, track_size - 1, "thumb should be at bottom when at max scroll");
    }

    #[test]
    fn test_drag_to_bottom_gives_max_scroll() {
        let state = ScrollbarState::new(0, 100, 20);
        // Scrollbar from row 5 to row 24 (height 20)
        // Track is from row 6 to row 23 (after arrows)
        let start_row = 5;
        let end_row = 24;

        // Drag to bottom of track (row 23)
        let new_pos = drag_to_vscroll(23, start_row, end_row, &state);
        assert_eq!(new_pos, 80, "dragging to bottom should give max scroll");
    }

    #[test]
    fn test_help_dialog_scrollbar_scenario() {
        // Simulate actual help dialog: 19 visible lines, ~50 lines of content
        let content_lines = 50;
        let visible_lines = 19;
        let vscroll_height = 19; // matches content height

        println!("Scenario: {} lines content, {} visible, {} scrollbar height",
            content_lines, visible_lines, vscroll_height);

        let state = ScrollbarState::new(0, content_lines, visible_lines);
        let max_scroll = state.max_scroll();
        println!("max_scroll = {}", max_scroll);
        assert_eq!(max_scroll, 31, "max scroll should be content - visible");

        // Scrollbar spans rows 7-25 (height 19)
        let start_row: u16 = 7;
        let end_row: u16 = 7 + vscroll_height - 1; // 25
        println!("scrollbar rows: {} to {}", start_row, end_row);

        // Track is between arrows: rows 8 to 24
        let track_start = start_row + 1; // 8
        let track_end = end_row - 1; // 24
        let track_size = (track_end - track_start + 1) as usize; // 17
        println!("track rows: {} to {}, size = {}", track_start, track_end, track_size);

        // Test dragging to various positions
        for drag_row in [track_start, (track_start + track_end) / 2, track_end] {
            let new_pos = drag_to_vscroll(drag_row, start_row, end_row, &state);
            println!("drag to row {} -> scroll pos {}", drag_row, new_pos);
        }

        // Drag to bottom should give max_scroll
        let bottom_pos = drag_to_vscroll(track_end, start_row, end_row, &state);
        println!("drag to track bottom (row {}) -> {}", track_end, bottom_pos);
        assert_eq!(bottom_pos, max_scroll, "drag to bottom should give max scroll");

        // Check thumb position at max scroll
        let state_at_max = ScrollbarState::new(max_scroll, content_lines, visible_lines);
        let thumb = state_at_max.thumb_pos(track_size);
        println!("at max scroll {}, thumb pos = {} (track size {})", max_scroll, thumb, track_size);
        assert_eq!(thumb, track_size - 1, "thumb should be at bottom of track");
    }
}
