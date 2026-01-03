//! DialogWidget - A composite widget for modal dialogs
//!
//! Combines FloatingWindow chrome (drag, resize, maximize) with a WidgetTree
//! for content. Handles focus management and Tab navigation.

use crate::input::InputEvent;
use crate::screen::Screen;
use crate::ui::theme::Theme;
use crate::ui::widget::{EventResult, mouse_position};
use crate::ui::widget_tree::{WidgetNode, WidgetTree};
use crate::ui::floating_window::FloatingWindow;

/// A modal dialog widget with chrome and content
pub struct DialogWidget {
    /// The floating window for chrome
    window: FloatingWindow,
    /// The widget tree for content
    content: WidgetTree,
    /// Whether this dialog is visible
    visible: bool,
    /// Screen dimensions for maximize support
    screen_size: (u16, u16),
    /// Whether chrome is interactive (drag/resize/maximize)
    chrome_interactive: bool,
    /// Whether to show maximize button
    show_maximize: bool,
}

impl DialogWidget {
    /// Create with a specific theme
    pub fn with_theme(title: impl Into<String>, content: WidgetNode, theme: Theme) -> Self {
        Self {
            window: FloatingWindow::new(title),
            content: WidgetTree::with_theme(content, theme),
            visible: true,
            screen_size: (80, 25),
            chrome_interactive: true,
            show_maximize: true,
        }
    }

    /// Set dialog size
    pub fn with_size(mut self, width: u16, height: u16) -> Self {
        self.window = self.window.with_size(width, height);
        self
    }

    /// Set minimum size
    pub fn with_min_size(mut self, min_width: u16, min_height: u16) -> Self {
        self.window = self.window.with_min_size(min_width, min_height);
        self
    }

    /// Center the dialog on screen
    pub fn center(&mut self) {
        self.window.center(self.screen_size.0, self.screen_size.1);
    }

    /// Set screen size (for centering and maximize)
    pub fn set_screen_size(&mut self, width: u16, height: u16) {
        self.screen_size = (width, height);
    }

    /// Enable or disable interactive chrome behavior (drag/resize/maximize)
    pub fn set_chrome_interactive(&mut self, interactive: bool) {
        self.chrome_interactive = interactive;
    }

    /// Show or hide the maximize button
    pub fn set_show_maximize(&mut self, show: bool) {
        self.show_maximize = show;
    }

    /// Get mutable access to the widget tree
    pub fn content_mut(&mut self) -> &mut WidgetTree {
        &mut self.content
    }

    /// Get access to the widget tree
    pub fn content(&self) -> &WidgetTree {
        &self.content
    }

    /// Focus the first focusable widget
    pub fn focus_first(&mut self) {
        self.content.focus_next();
    }

    /// Draw the dialog with theme-based colors
    pub fn draw_with_theme(&self, screen: &mut Screen) {
        if !self.visible {
            return;
        }

        let theme = self.content.theme();
        let bounds = self.window.bounds();

        // Draw shadow
        screen.draw_shadow(bounds.y, bounds.x, bounds.width, bounds.height);

        // Draw window background with theme colors
        screen.fill(
            bounds.y,
            bounds.x,
            bounds.width,
            bounds.height,
            ' ',
            theme.dialog_fg,
            theme.dialog_bg,
        );

        // Draw border with theme colors
        screen.draw_box(
            bounds.y,
            bounds.x,
            bounds.width,
            bounds.height,
            theme.dialog_border_fg,
            theme.dialog_border_bg,
        );

        // Draw title bar
        self.draw_title_bar(screen, theme);

        // Draw content
        let content_rect = self.window.content_rect();
        self.content.draw(screen, content_rect);
    }

    /// Draw the title bar with theme colors
    fn draw_title_bar(&self, screen: &mut Screen, theme: &Theme) {
        let bounds = self.window.bounds();
        let title = &self.window.title;

        // Title text centered on top border
        let title_x = bounds.x + (bounds.width.saturating_sub(title.len() as u16 + 2)) / 2;
        screen.set(bounds.y, title_x, ' ', theme.dialog_title_fg, theme.dialog_title_bg);
        screen.write_str(bounds.y, title_x + 1, title, theme.dialog_title_fg, theme.dialog_title_bg);
        screen.set(bounds.y, title_x + 1 + title.len() as u16, ' ', theme.dialog_title_fg, theme.dialog_title_bg);

        // Maximize button
        if self.show_maximize {
            let btn_col = bounds.x + bounds.width - crate::ui::window_chrome::MAXIMIZE_BUTTON_OFFSET;
            let btn_str = if self.window.is_maximized() { "[↕]" } else { "[↑]" };
            screen.write_str(bounds.y, btn_col, btn_str, theme.dialog_border_fg, theme.dialog_border_bg);
        }
    }

    /// Handle events
    pub fn handle_event(&mut self, event: &InputEvent) -> EventResult {
        if !self.visible {
            return EventResult::Ignored;
        }

        // Handle Escape to close
        if matches!(event, InputEvent::Escape) {
            return EventResult::Action("dialog_cancel".to_string());
        }

        // Click outside dialog closes it (matches legacy behavior)
        if matches!(event, InputEvent::MouseClick { .. }) {
            if let Some((row, col)) = mouse_position(event) {
                if !self.window.bounds().contains(row, col) {
                    return EventResult::Action("dialog_cancel".to_string());
                }
            }
        }

        // Handle window chrome events (drag, resize, maximize)
        if self.chrome_interactive {
            if self.window.handle_event_with_screen(event, self.screen_size.0, self.screen_size.1) {
                return EventResult::Consumed;
            }
        }

        // Route to content widget tree
        let content_rect = self.window.content_rect();
        self.content.handle_event(event, content_rect)
    }
}
