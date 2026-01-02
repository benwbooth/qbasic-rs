//! Floating window component
#![allow(dead_code)]
//!
//! Provides shared drag, resize, and chrome rendering for modal dialogs
//! and other floating windows.

use crate::input::InputEvent;
use crate::screen::Screen;
use crate::terminal::Color;
use super::layout::Rect;

/// A floating window with drag and resize support
pub struct FloatingWindow {
    /// Window position
    pub x: u16,
    pub y: u16,
    /// Window size
    pub width: u16,
    pub height: u16,
    /// Window title
    pub title: String,
    /// Minimum size constraints
    pub min_width: u16,
    pub min_height: u16,
    /// Whether window is being dragged
    dragging: bool,
    /// Drag offset from window corner
    drag_offset: (u16, u16),
    /// Whether window is being resized
    resizing: bool,
    /// Whether window is maximized
    maximized: bool,
    /// Saved bounds for restore (x, y, width, height)
    saved_bounds: Option<(u16, u16, u16, u16)>,
    /// Last click time for double-click detection
    last_click_time: std::time::Instant,
    /// Last click position
    last_click_pos: (u16, u16),
}

impl FloatingWindow {
    /// Create a new floating window
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            x: 0,
            y: 0,
            width: 60,
            height: 18,
            title: title.into(),
            min_width: 20,
            min_height: 8,
            dragging: false,
            drag_offset: (0, 0),
            resizing: false,
            maximized: false,
            saved_bounds: None,
            last_click_time: std::time::Instant::now(),
            last_click_pos: (0, 0),
        }
    }

    /// Set window size
    pub fn with_size(mut self, width: u16, height: u16) -> Self {
        self.width = width;
        self.height = height;
        self
    }

    /// Set minimum size constraints
    pub fn with_min_size(mut self, min_width: u16, min_height: u16) -> Self {
        self.min_width = min_width;
        self.min_height = min_height;
        self
    }

    /// Center the window on screen
    pub fn center(&mut self, screen_width: u16, screen_height: u16) {
        self.x = (screen_width.saturating_sub(self.width)) / 2;
        self.y = (screen_height.saturating_sub(self.height)) / 2;
    }

    /// Get the content area rect (inside the border)
    pub fn content_rect(&self) -> Rect {
        Rect {
            x: self.x + 1,
            y: self.y + 1,
            width: self.width.saturating_sub(2),
            height: self.height.saturating_sub(2),
        }
    }

    /// Get the full window bounds
    pub fn bounds(&self) -> Rect {
        Rect {
            x: self.x,
            y: self.y,
            width: self.width,
            height: self.height,
        }
    }

    /// Check if a point is in the title bar (excluding maximize button)
    fn in_title_bar(&self, row: u16, col: u16) -> bool {
        row == self.y && col >= self.x && col < self.x + self.width - 3
    }

    /// Check if a point is on the maximize button [^] (2 chars from right corner)
    fn in_maximize_button(&self, row: u16, col: u16) -> bool {
        row == self.y && col >= self.x + self.width - 3 && col < self.x + self.width - 1
    }

    /// Check if a point is in the resize handle (bottom-right corner)
    fn in_resize_handle(&self, row: u16, col: u16) -> bool {
        row == self.y + self.height - 1 && col >= self.x + self.width - 2
    }

    /// Toggle maximize state
    pub fn toggle_maximize(&mut self, screen_width: u16, screen_height: u16) {
        if self.maximized {
            // Restore
            if let Some((x, y, w, h)) = self.saved_bounds {
                self.x = x;
                self.y = y;
                self.width = w;
                self.height = h;
            }
            self.saved_bounds = None;
            self.maximized = false;
        } else {
            // Maximize
            self.saved_bounds = Some((self.x, self.y, self.width, self.height));
            self.x = 1;
            self.y = 1;
            self.width = screen_width - 2;
            self.height = screen_height - 2;
            self.maximized = true;
        }
    }

    /// Check if maximized
    pub fn is_maximized(&self) -> bool {
        self.maximized
    }

    /// Handle mouse events for drag/resize
    /// Returns true if event was consumed
    pub fn handle_event(&mut self, event: &InputEvent) -> bool {
        self.handle_event_with_screen(event, 80, 25)
    }

    /// Handle mouse events with screen dimensions for maximize support
    pub fn handle_event_with_screen(&mut self, event: &InputEvent, screen_width: u16, screen_height: u16) -> bool {
        // Handle drag in progress
        if self.dragging {
            match event {
                InputEvent::MouseDrag { row, col } => {
                    self.x = col.saturating_sub(self.drag_offset.0);
                    self.y = row.saturating_sub(self.drag_offset.1);
                    return true;
                }
                InputEvent::MouseRelease { .. } => {
                    self.dragging = false;
                    return true;
                }
                _ => {}
            }
        }

        // Handle resize in progress
        if self.resizing {
            match event {
                InputEvent::MouseDrag { row, col } => {
                    let new_width = col.saturating_sub(self.x).saturating_add(1).max(self.min_width);
                    let new_height = row.saturating_sub(self.y).saturating_add(1).max(self.min_height);
                    self.width = new_width;
                    self.height = new_height;
                    return true;
                }
                InputEvent::MouseRelease { .. } => {
                    self.resizing = false;
                    return true;
                }
                _ => {}
            }
        }

        // Check for click to start drag, resize, or maximize
        if let InputEvent::MouseClick { row, col } = event {
            // Check maximize button first
            if self.in_maximize_button(*row, *col) {
                self.toggle_maximize(screen_width, screen_height);
                return true;
            }

            if self.in_title_bar(*row, *col) {
                // Check for double-click to toggle maximize
                let now = std::time::Instant::now();
                let elapsed = now.duration_since(self.last_click_time);
                let same_row = self.last_click_pos.0 == *row;
                let is_double_click = same_row && elapsed.as_millis() < 400;

                self.last_click_time = now;
                self.last_click_pos = (*row, *col);

                if is_double_click {
                    self.toggle_maximize(screen_width, screen_height);
                    return true;
                }

                self.dragging = true;
                self.drag_offset = (col - self.x, row - self.y);
                return true;
            }
            if self.in_resize_handle(*row, *col) {
                self.resizing = true;
                return true;
            }
        }

        false
    }

    /// Draw window chrome (shadow, border, title bar, maximize button)
    pub fn draw_chrome(&self, screen: &mut Screen) {
        // Draw shadow
        screen.draw_shadow(self.y, self.x, self.width, self.height);

        // Draw window background
        screen.fill(self.y, self.x, self.width, self.height, ' ', Color::Black, Color::LightGray);

        // Draw border
        screen.draw_box(self.y, self.x, self.width, self.height, Color::Black, Color::LightGray);

        // Draw title (centered in title bar, but leave room for maximize button)
        let title_str = format!(" {} ", self.title);
        let title_x = self.x + (self.width.saturating_sub(title_str.len() as u16 + 3)) / 2;
        screen.write_str(self.y, title_x, &title_str, Color::Black, Color::LightGray);

        // Draw maximize/restore button (2 chars from right corner)
        let btn_col = self.x + self.width - 3;
        let btn_char = if self.maximized { '▼' } else { '▲' };
        screen.set(self.y, btn_col, btn_char, Color::Black, Color::LightGray);
    }

    /// Check if currently dragging
    pub fn is_dragging(&self) -> bool {
        self.dragging
    }

    /// Check if currently resizing
    pub fn is_resizing(&self) -> bool {
        self.resizing
    }
}
