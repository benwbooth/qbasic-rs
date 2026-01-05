//! ListView widget - a scrollable list with integrated scrollbar
#![allow(dead_code)]

use crate::input::InputEvent;
use crate::screen::Screen;
use crate::terminal::Color;
use crate::ui::layout::{Rect, SizeHint};
use crate::ui::theme::Theme;
use crate::ui::widget::{Widget, EventResult, mouse_position};
use crate::ui::widget_tree::{TreeWidget, EventPhase};
use crate::ui::scrollbar::{self, ScrollbarColors, ScrollbarState, draw_vertical};

/// Colors for the list view
#[derive(Clone, Copy)]
pub struct ListViewColors {
    pub item_fg: Color,
    pub item_bg: Color,
    pub selected_fg: Color,
    pub selected_bg: Color,
    pub border_fg: Color,
    pub border_bg: Color,
}

impl Default for ListViewColors {
    fn default() -> Self {
        Self {
            item_fg: Color::Black,
            item_bg: Color::LightGray,
            selected_fg: Color::White,
            selected_bg: Color::Black,
            border_fg: Color::Black,
            border_bg: Color::LightGray,
        }
    }
}

/// A scrollable list view widget with optional border and scrollbar
pub struct ListView {
    /// List items
    items: Vec<String>,
    /// Currently selected index
    selected_index: usize,
    /// Scroll offset (first visible item)
    scroll_offset: usize,
    /// Colors
    colors: ListViewColors,
    /// Scrollbar colors
    scrollbar_colors: ScrollbarColors,
    /// Whether to show border
    show_border: bool,
    /// Whether widget has focus
    focused: bool,
    /// Action prefix for events
    action_prefix: String,
    /// Optional fixed minimum width override (used by widget tree layout)
    min_width_override: Option<u16>,
    /// Whether scrollbar is being dragged
    scrollbar_dragging: bool,
    /// Last click time for double-click detection
    last_click_time: std::time::Instant,
    /// Last clicked index for double-click detection
    last_click_index: Option<usize>,
}

impl ListView {
    pub fn new(action_prefix: impl Into<String>) -> Self {
        Self {
            items: Vec::new(),
            selected_index: 0,
            scroll_offset: 0,
            colors: ListViewColors::default(),
            scrollbar_colors: ScrollbarColors::default(),
            show_border: true,
            focused: false,
            action_prefix: action_prefix.into(),
            scrollbar_dragging: false,
            last_click_time: std::time::Instant::now(),
            last_click_index: None,
            min_width_override: None,
        }
    }

    pub fn with_colors(mut self, colors: ListViewColors) -> Self {
        self.colors = colors;
        self
    }

    pub fn with_scrollbar_colors(mut self, colors: ScrollbarColors) -> Self {
        self.scrollbar_colors = colors;
        self
    }

    pub fn with_border(mut self, show_border: bool) -> Self {
        self.show_border = show_border;
        self
    }

    /// Set a fixed minimum width for layout
    pub fn set_min_width(&mut self, width: u16) {
        self.min_width_override = Some(width);
    }

    /// Set the list items
    pub fn set_items(&mut self, items: Vec<String>) {
        self.items = items;
        // Clamp selection to valid range
        if !self.items.is_empty() {
            self.selected_index = self.selected_index.min(self.items.len() - 1);
        } else {
            self.selected_index = 0;
        }
        self.ensure_visible();
    }

    /// Get the items
    pub fn items(&self) -> &[String] {
        &self.items
    }

    /// Get selected index
    pub fn selected_index(&self) -> usize {
        self.selected_index
    }

    /// Set selected index
    pub fn set_selected_index(&mut self, index: usize) {
        if !self.items.is_empty() {
            self.selected_index = index.min(self.items.len() - 1);
            self.ensure_visible();
        }
    }

    /// Get selected item
    pub fn selected_item(&self) -> Option<&str> {
        self.items.get(self.selected_index).map(|s| s.as_str())
    }

    /// Get scroll offset
    pub fn scroll_offset(&self) -> usize {
        self.scroll_offset
    }

    /// Calculate visible height (accounting for border)
    fn visible_height(&self, bounds: Rect) -> usize {
        let border_offset = if self.show_border { 2 } else { 0 };
        bounds.height.saturating_sub(border_offset) as usize
    }

    /// Calculate content area (inside border)
    fn content_rect(&self, bounds: Rect) -> Rect {
        if self.show_border {
            Rect {
                x: bounds.x + 1,
                y: bounds.y + 1,
                width: bounds.width.saturating_sub(2),
                height: bounds.height.saturating_sub(2),
            }
        } else {
            bounds
        }
    }

    /// Calculate scrollbar column
    fn scrollbar_col(&self, bounds: Rect) -> u16 {
        if self.show_border {
            bounds.x + bounds.width - 1
        } else {
            bounds.x + bounds.width - 1
        }
    }

    /// Ensure selected item is visible
    fn ensure_visible(&mut self) {
        // We need bounds to know visible height, but we'll use a reasonable default
        // This will be properly calculated when drawing
    }

    /// Ensure selected item is visible given visible height
    fn ensure_visible_with_height(&mut self, visible_height: usize) {
        if visible_height == 0 {
            return;
        }

        // If selected is above scroll window, scroll up
        if self.selected_index < self.scroll_offset {
            self.scroll_offset = self.selected_index;
        }
        // If selected is below scroll window, scroll down
        else if self.selected_index >= self.scroll_offset + visible_height {
            self.scroll_offset = self.selected_index - visible_height + 1;
        }
    }

    /// Move selection up
    pub fn select_prev(&mut self) {
        if self.selected_index > 0 {
            self.selected_index -= 1;
        }
    }

    /// Move selection down
    pub fn select_next(&mut self) {
        if self.selected_index + 1 < self.items.len() {
            self.selected_index += 1;
        }
    }

    /// Page up
    pub fn page_up(&mut self, visible_height: usize) {
        let page = visible_height.saturating_sub(1).max(1);
        self.selected_index = self.selected_index.saturating_sub(page);
        self.ensure_visible_with_height(visible_height);
    }

    /// Page down
    pub fn page_down(&mut self, visible_height: usize) {
        let page = visible_height.saturating_sub(1).max(1);
        self.selected_index = (self.selected_index + page).min(self.items.len().saturating_sub(1));
        self.ensure_visible_with_height(visible_height);
    }

    /// Scroll up without changing selection
    pub fn scroll_up(&mut self, n: usize) {
        self.scroll_offset = self.scroll_offset.saturating_sub(n);
    }

    /// Scroll down without changing selection
    pub fn scroll_down(&mut self, n: usize, visible_height: usize) {
        let max_scroll = self.items.len().saturating_sub(visible_height);
        self.scroll_offset = (self.scroll_offset + n).min(max_scroll);
    }

    fn draw_border(&self, screen: &mut Screen, bounds: Rect) {
        let (fg, bg) = (self.colors.border_fg, self.colors.border_bg);

        // Top border
        screen.set(bounds.y, bounds.x, '┌', fg, bg);
        for col in (bounds.x + 1)..(bounds.x + bounds.width - 1) {
            screen.set(bounds.y, col, '─', fg, bg);
        }
        screen.set(bounds.y, bounds.x + bounds.width - 1, '┐', fg, bg);

        // Side borders
        for row in (bounds.y + 1)..(bounds.y + bounds.height - 1) {
            screen.set(row, bounds.x, '│', fg, bg);
            // Right border is where scrollbar goes
            screen.set(row, bounds.x + bounds.width - 1, '│', fg, bg);
        }

        // Bottom border
        screen.set(bounds.y + bounds.height - 1, bounds.x, '└', fg, bg);
        for col in (bounds.x + 1)..(bounds.x + bounds.width - 1) {
            screen.set(bounds.y + bounds.height - 1, col, '─', fg, bg);
        }
        screen.set(bounds.y + bounds.height - 1, bounds.x + bounds.width - 1, '┘', fg, bg);
    }
}

impl Widget for ListView {
    fn draw(&self, screen: &mut Screen, bounds: Rect) {
        if bounds.width < 3 || bounds.height < 3 {
            return;
        }

        // Draw border if enabled
        if self.show_border {
            self.draw_border(screen, bounds);
        }

        let content = self.content_rect(bounds);
        let visible_height = content.height as usize;
        let item_width = content.width.saturating_sub(1) as usize; // Leave room for scrollbar

        // Draw items
        for i in 0..visible_height {
            let item_index = self.scroll_offset + i;
            let row = content.y + i as u16;

            if item_index < self.items.len() {
                let item = &self.items[item_index];
                let is_selected = item_index == self.selected_index;
                let (fg, bg) = if is_selected {
                    (self.colors.selected_fg, self.colors.selected_bg)
                } else {
                    (self.colors.item_fg, self.colors.item_bg)
                };

                // Draw item text (truncated or padded to width)
                for (j, ch) in item.chars().take(item_width).enumerate() {
                    screen.set(row, content.x + j as u16, ch, fg, bg);
                }
                // Pad remainder
                for j in item.chars().count()..item_width {
                    screen.set(row, content.x + j as u16, ' ', fg, bg);
                }
            } else {
                // Empty row
                for j in 0..item_width {
                    screen.set(row, content.x + j as u16, ' ', self.colors.item_fg, self.colors.item_bg);
                }
            }
        }

        // Draw scrollbar if needed
        if self.items.len() > visible_height {
            let scrollbar_col = content.x + content.width - 1;
            let state = ScrollbarState::new(self.scroll_offset, self.items.len(), visible_height);
            draw_vertical(
                screen,
                scrollbar_col,
                content.y,
                content.y + content.height - 1,
                &state,
                &self.scrollbar_colors,
            );
        }
    }

    fn handle_event(&mut self, event: &InputEvent, bounds: Rect) -> EventResult {
        let content = self.content_rect(bounds);
        let visible_height = content.height as usize;

        // Handle scrollbar dragging
        if self.scrollbar_dragging {
            match event {
                InputEvent::MouseDrag { row, .. } => {
                    let start_row = content.y;
                    let end_row = content.y + content.height - 1;
                    let state = ScrollbarState::new(self.scroll_offset, self.items.len(), visible_height);
                    let new_scroll = scrollbar::drag_to_vscroll(*row, start_row, end_row, &state);
                    self.scroll_offset = new_scroll;
                    return EventResult::Action(format!("{}_scroll", self.action_prefix));
                }
                InputEvent::MouseRelease { .. } => {
                    self.scrollbar_dragging = false;
                    return EventResult::Consumed;
                }
                _ => {}
            }
        }

        // Handle keyboard events if focused
        if self.focused {
            match event {
                InputEvent::CursorUp => {
                    self.select_prev();
                    self.ensure_visible_with_height(visible_height);
                    return EventResult::Action(format!("{}_select", self.action_prefix));
                }
                InputEvent::CursorDown => {
                    self.select_next();
                    self.ensure_visible_with_height(visible_height);
                    return EventResult::Action(format!("{}_select", self.action_prefix));
                }
                InputEvent::PageUp => {
                    self.page_up(visible_height);
                    return EventResult::Action(format!("{}_select", self.action_prefix));
                }
                InputEvent::PageDown => {
                    self.page_down(visible_height);
                    return EventResult::Action(format!("{}_select", self.action_prefix));
                }
                InputEvent::Home => {
                    self.selected_index = 0;
                    self.ensure_visible_with_height(visible_height);
                    return EventResult::Action(format!("{}_select", self.action_prefix));
                }
                InputEvent::End => {
                    if !self.items.is_empty() {
                        self.selected_index = self.items.len() - 1;
                        self.ensure_visible_with_height(visible_height);
                    }
                    return EventResult::Action(format!("{}_select", self.action_prefix));
                }
                InputEvent::Enter => {
                    return EventResult::Action(format!("{}_activate", self.action_prefix));
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

        // Check if click is on scrollbar
        let scrollbar_col = content.x + content.width - 1;
        if col == scrollbar_col && self.items.len() > visible_height {
            if let InputEvent::MouseClick { .. } = event {
                let start_row = content.y;
                let end_row = content.y + content.height - 1;
                let state = ScrollbarState::new(self.scroll_offset, self.items.len(), visible_height);

                // Check different scrollbar regions
                if row == start_row {
                    // Up arrow
                    self.scroll_up(1);
                    return EventResult::Action(format!("{}_scroll", self.action_prefix));
                } else if row == end_row {
                    // Down arrow
                    self.scroll_down(1, visible_height);
                    return EventResult::Action(format!("{}_scroll", self.action_prefix));
                } else {
                    // Track area - check if on thumb
                    let track_size = (end_row.saturating_sub(start_row).saturating_sub(1)) as usize;
                    if track_size > 0 {
                        let thumb_pos = state.thumb_pos(track_size);
                        let thumb_row = start_row + 1 + thumb_pos as u16;

                        if row == thumb_row {
                            // Start dragging
                            self.scrollbar_dragging = true;
                            return EventResult::Consumed;
                        } else if row < thumb_row {
                            // Page up
                            self.scroll_offset = self.scroll_offset.saturating_sub(visible_height.saturating_sub(1).max(1));
                            return EventResult::Action(format!("{}_scroll", self.action_prefix));
                        } else {
                            // Page down
                            let max_scroll = self.items.len().saturating_sub(visible_height);
                            self.scroll_offset = (self.scroll_offset + visible_height.saturating_sub(1).max(1)).min(max_scroll);
                            return EventResult::Action(format!("{}_scroll", self.action_prefix));
                        }
                    }
                }
            }
            return EventResult::Consumed;
        }

        // Check if click is on content area
        if row > bounds.y && row < bounds.y + bounds.height - 1 &&
           col > bounds.x && col < bounds.x + bounds.width - 1 {
            if let InputEvent::MouseClick { .. } = event {
                let visual_idx = (row - content.y) as usize;
                let item_idx = self.scroll_offset + visual_idx;

                if item_idx < self.items.len() {
                    // Check for double-click
                    let now = std::time::Instant::now();
                    let elapsed = now.duration_since(self.last_click_time);
                    let same_item = self.last_click_index == Some(item_idx);
                    let is_double_click = same_item && elapsed.as_millis() < 400;

                    self.selected_index = item_idx;
                    self.last_click_time = now;
                    self.last_click_index = Some(item_idx);

                    if is_double_click {
                        return EventResult::Action(format!("{}_activate", self.action_prefix));
                    } else {
                        return EventResult::Action(format!("{}_select", self.action_prefix));
                    }
                }
            }
        }

        // Handle scroll wheel
        match event {
            InputEvent::ScrollUp { .. } => {
                self.scroll_up(3);
                return EventResult::Action(format!("{}_scroll", self.action_prefix));
            }
            InputEvent::ScrollDown { .. } => {
                self.scroll_down(3, visible_height);
                return EventResult::Action(format!("{}_scroll", self.action_prefix));
            }
            _ => {}
        }

        EventResult::Ignored
    }

    fn focusable(&self) -> bool {
        true
    }

    fn set_focus(&mut self, focused: bool) {
        self.focused = focused;
    }

}

/// TreeWidget implementation for use in widget trees
/// Uses Theme for colors instead of ListViewColors
impl TreeWidget for ListView {
    fn draw(&self, screen: &mut Screen, bounds: Rect, theme: &Theme) {
        if bounds.width < 3 || bounds.height < 3 {
            return;
        }

        // Draw border if enabled using theme colors
        if self.show_border {
            let (fg, bg) = (theme.dialog_border_fg, theme.dialog_border_bg);

            // Top border
            screen.set(bounds.y, bounds.x, '┌', fg, bg);
            for col in (bounds.x + 1)..(bounds.x + bounds.width - 1) {
                screen.set(bounds.y, col, '─', fg, bg);
            }
            screen.set(bounds.y, bounds.x + bounds.width - 1, '┐', fg, bg);

            // Side borders
            for row in (bounds.y + 1)..(bounds.y + bounds.height - 1) {
                screen.set(row, bounds.x, '│', fg, bg);
                screen.set(row, bounds.x + bounds.width - 1, '│', fg, bg);
            }

            // Bottom border
            screen.set(bounds.y + bounds.height - 1, bounds.x, '└', fg, bg);
            for col in (bounds.x + 1)..(bounds.x + bounds.width - 1) {
                screen.set(bounds.y + bounds.height - 1, col, '─', fg, bg);
            }
            screen.set(bounds.y + bounds.height - 1, bounds.x + bounds.width - 1, '┘', fg, bg);
        }

        let content = self.content_rect(bounds);
        let visible_height = content.height as usize;
        let item_width = content.width.saturating_sub(1) as usize; // Leave room for scrollbar

        // Draw items with theme colors
        let item_fg = theme.list_fg;
        let item_bg = theme.list_bg;
        let selected_fg = if self.focused { theme.list_focused_selected_fg } else { theme.list_selected_fg };
        let selected_bg = if self.focused { theme.list_focused_selected_bg } else { theme.list_selected_bg };

        for i in 0..visible_height {
            let item_index = self.scroll_offset + i;
            let row = content.y + i as u16;

            if item_index < self.items.len() {
                let item = &self.items[item_index];
                let is_selected = item_index == self.selected_index;
                let (fg, bg) = if is_selected {
                    (selected_fg, selected_bg)
                } else {
                    (item_fg, item_bg)
                };

                // Draw item text (truncated or padded to width)
                for (j, ch) in item.chars().take(item_width).enumerate() {
                    screen.set(row, content.x + j as u16, ch, fg, bg);
                }
                // Pad remainder
                for j in item.chars().count()..item_width {
                    screen.set(row, content.x + j as u16, ' ', fg, bg);
                }
            } else {
                // Empty row
                for j in 0..item_width {
                    screen.set(row, content.x + j as u16, ' ', item_fg, item_bg);
                }
            }
        }

        // Draw scrollbar if needed with theme colors
        if self.items.len() > visible_height {
            let scrollbar_col = content.x + content.width - 1;
            let state = ScrollbarState::new(self.scroll_offset, self.items.len(), visible_height);
            let scrollbar_colors = ScrollbarColors {
                track_fg: theme.scrollbar_track_fg,
                track_bg: theme.scrollbar_track_bg,
                thumb_fg: theme.scrollbar_thumb_fg,
                thumb_bg: theme.scrollbar_thumb_bg,
                arrow_fg: theme.scrollbar_track_fg,
                arrow_bg: theme.scrollbar_track_bg,
            };
            draw_vertical(
                screen,
                scrollbar_col,
                content.y,
                content.y + content.height - 1,
                &state,
                &scrollbar_colors,
            );
        }
    }

    fn handle_event(&mut self, event: &InputEvent, bounds: Rect, phase: EventPhase) -> EventResult {
        if phase != EventPhase::Target {
            return EventResult::Ignored;
        }
        Widget::handle_event(self, event, bounds)
    }

    fn size_hint(&self) -> SizeHint {
        SizeHint {
            min_width: self.min_width_override.unwrap_or(10),
            min_height: 3, // Minimum 3 rows (top border, 1 item, bottom border)
            flex: 1, // Lists can expand
        }
    }

    fn focusable(&self) -> bool {
        true
    }

    fn set_focus(&mut self, focused: bool) {
        self.focused = focused;
    }

    fn wants_tight_width(&self) -> bool {
        self.min_width_override.is_some()
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}
