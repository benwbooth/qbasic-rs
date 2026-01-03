//! Widget container that holds all main widgets and provides unified iteration
//!
//! This is the single place where the widget list is defined. All drawing and event
//! routing goes through this container, which recursively handles its children.

use crate::screen::Screen;
use crate::state::{AppState, Focus};
use super::layout::ComputedLayout;
use super::main_widget::{MainWidget, WidgetAction};
use super::{MenuBar, Editor, ImmediateWindow, OutputWindow, StatusBar};
use crate::input::InputEvent;

/// Container for all main UI widgets.
/// This is effectively the "root widget" that draws and routes events to children.
pub struct Widgets {
    pub menubar: MenuBar,
    pub editor: Editor,
    pub immediate: ImmediateWindow,
    pub output: OutputWindow,
}

impl Widgets {
    pub fn new() -> Self {
        Self {
            menubar: MenuBar::new(),
            editor: Editor::new(),
            immediate: ImmediateWindow::new(),
            output: OutputWindow::new(),
        }
    }

    /// Draw all widgets in order (back to front).
    /// StatusBar is special - it needs editor cursor info, so it's drawn separately.
    pub fn draw(&mut self, screen: &mut Screen, state: &AppState, layout: &ComputedLayout) {
        // Menu bar
        let menu_rect = layout.get("menu_bar").unwrap_or_default();
        self.menubar.draw(screen, state, menu_rect);

        // Editor
        let editor_rect = layout.get("editor").unwrap_or_default();
        self.editor.draw(screen, state, editor_rect);

        // Immediate window (when visible)
        if state.show_immediate {
            let imm_rect = layout.get("immediate").unwrap_or_default();
            let has_focus = state.focus == Focus::Immediate;
            self.immediate.draw(screen, state, imm_rect, has_focus);
        }

        // Output window (when visible in split mode - rarely used now)
        if state.show_output {
            let out_rect = layout.get("output").unwrap_or_default();
            self.output.draw(screen, state, out_rect);
        }

        // Status bar is special - needs editor cursor info
        let status_rect = layout.get("status_bar").unwrap_or_default();
        StatusBar::draw(
            screen,
            state,
            self.editor.cursor_line,
            self.editor.cursor_col,
            status_rect,
        );
    }

    /// Handle a mouse event by routing to appropriate widget.
    /// Widgets are checked in front-to-back order (reverse of draw order).
    pub fn handle_mouse_event(&mut self, event: &InputEvent, state: &mut AppState, layout: &ComputedLayout) -> WidgetAction {
        // Menu bar first (handles dropdown overlay when open)
        let menu_rect = layout.get("menu_bar").unwrap_or_default();
        let action = self.menubar.handle_event(event, state, menu_rect);
        if !matches!(action, WidgetAction::Ignored) {
            return action;
        }

        // Editor
        let editor_rect = layout.get("editor").unwrap_or_default();
        let action = self.editor.handle_event(event, state, editor_rect);
        if !matches!(action, WidgetAction::Ignored) {
            return action;
        }

        // Immediate window
        if state.show_immediate {
            let imm_rect = layout.get("immediate").unwrap_or_default();
            let action = self.immediate.handle_event(event, state, imm_rect);
            if !matches!(action, WidgetAction::Ignored) {
                return action;
            }
        }

        // Output window
        let out_rect = layout.get("output").unwrap_or_default();
        let action = self.output.handle_event(event, state, out_rect);
        if !matches!(action, WidgetAction::Ignored) {
            return action;
        }

        WidgetAction::Ignored
    }

    /// Handle a keyboard event by routing to the focused widget.
    pub fn handle_keyboard_event(&mut self, event: &InputEvent, state: &mut AppState, layout: &ComputedLayout) -> WidgetAction {
        // Route to widget based on focus
        match state.focus {
            Focus::Menu => {
                let menu_rect = layout.get("menu_bar").unwrap_or_default();
                self.menubar.handle_event(event, state, menu_rect)
            }
            Focus::Editor => {
                let editor_rect = layout.get("editor").unwrap_or_default();
                self.editor.handle_event(event, state, editor_rect)
            }
            Focus::Immediate => {
                let imm_rect = layout.get("immediate").unwrap_or_default();
                self.immediate.handle_event(event, state, imm_rect)
            }
            Focus::Dialog => {
                // Dialog handles its own events, not through widgets
                WidgetAction::Ignored
            }
        }
    }

    /// Handle scroll wheel event by finding the widget under the mouse.
    pub fn handle_scroll(&mut self, event: &InputEvent, state: &mut AppState, layout: &ComputedLayout) -> WidgetAction {
        // Try each widget's scroll handler
        let editor_rect = layout.get("editor").unwrap_or_default();
        let action = self.editor.handle_scroll(event, editor_rect);
        if !matches!(action, WidgetAction::Ignored) {
            return action;
        }

        if state.show_immediate {
            let imm_rect = layout.get("immediate").unwrap_or_default();
            let action = self.immediate.handle_scroll(event, imm_rect);
            if !matches!(action, WidgetAction::Ignored) {
                return action;
            }
        }

        WidgetAction::Ignored
    }
}

impl Default for Widgets {
    fn default() -> Self {
        Self::new()
    }
}
