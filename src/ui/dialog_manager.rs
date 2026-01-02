//! Dialog Manager - bridges widget system with AppState
#![allow(dead_code)]
//!
//! This module manages dialog widgets and synchronizes their state
//! with AppState for integration with the existing main.rs code.

use crate::input::InputEvent;
use crate::state::{AppState, DialogType};
use super::layout::{Rect, ComputedLayout, compute_layout};
use super::layout::{find_dialog_layout, replace_dialog_layout, goto_line_dialog_layout, simple_input_dialog_layout};
use super::widget::EventResult;
use super::dialog_widgets::{CompositeDialog, create_find_dialog, create_replace_dialog, create_goto_dialog, create_simple_input_dialog};

/// Actions that can be triggered by dialogs
#[derive(Debug, Clone, PartialEq)]
pub enum DialogAction {
    /// No action
    None,
    /// Close the dialog
    Close,
    /// Find text (for Find dialog)
    Find,
    /// Find next occurrence
    FindNext,
    /// Replace current match
    Replace,
    /// Replace all matches
    ReplaceAll,
    /// Go to line (for GoToLine dialog)
    GoToLine,
    /// OK button pressed
    Ok,
    /// Yes button pressed
    Yes,
    /// No button pressed
    No,
    /// Cancel button pressed
    Cancel,
    /// Toggle case sensitivity
    ToggleCase,
    /// Toggle whole word matching
    ToggleWholeWord,
}

/// Manages dialog widgets and their state
pub struct DialogManager {
    /// Current active dialog widget (if any)
    current_dialog: Option<CompositeDialog>,
    /// Cached layout for current dialog
    cached_layout: Option<ComputedLayout>,
}

impl DialogManager {
    pub fn new() -> Self {
        Self {
            current_dialog: None,
            cached_layout: None,
        }
    }

    /// Check if a dialog widget is active
    pub fn has_widget(&self) -> bool {
        self.current_dialog.is_some()
    }

    /// Create/update dialog widget for current dialog type
    pub fn sync_with_state(&mut self, state: &AppState) {
        match &state.dialog {
            DialogType::None => {
                self.current_dialog = None;
                self.cached_layout = None;
            }
            DialogType::Find => {
                if self.current_dialog.is_none() {
                    let mut dialog = create_find_dialog();
                    // Sync state
                    if let Some(field) = dialog.field_mut("find_field") {
                        if let Some(tf) = field.widget.as_textfield_mut() {
                            tf.set_text(&state.dialog_find_text);
                            tf.set_cursor_pos(state.dialog_input_cursor);
                        }
                    }
                    if let Some(field) = dialog.field_mut("case_checkbox") {
                        if let Some(cb) = field.widget.as_checkbox_mut() {
                            cb.set_checked(state.search_case_sensitive);
                        }
                    }
                    if let Some(field) = dialog.field_mut("whole_checkbox") {
                        if let Some(cb) = field.widget.as_checkbox_mut() {
                            cb.set_checked(state.search_whole_word);
                        }
                    }
                    // Set initial focus
                    dialog.set_focus("find_field");
                    self.current_dialog = Some(dialog);
                }
                // Update layout
                let bounds = Rect::new(state.dialog_x, state.dialog_y, state.dialog_width, state.dialog_height);
                self.cached_layout = Some(compute_layout(&find_dialog_layout(), bounds));
            }
            DialogType::Replace => {
                if self.current_dialog.is_none() {
                    let mut dialog = create_replace_dialog();
                    // Sync state
                    if let Some(field) = dialog.field_mut("find_field") {
                        if let Some(tf) = field.widget.as_textfield_mut() {
                            tf.set_text(&state.dialog_find_text);
                        }
                    }
                    if let Some(field) = dialog.field_mut("replace_field") {
                        if let Some(tf) = field.widget.as_textfield_mut() {
                            tf.set_text(&state.dialog_replace_text);
                        }
                    }
                    if let Some(field) = dialog.field_mut("case_checkbox") {
                        if let Some(cb) = field.widget.as_checkbox_mut() {
                            cb.set_checked(state.search_case_sensitive);
                        }
                    }
                    if let Some(field) = dialog.field_mut("whole_checkbox") {
                        if let Some(cb) = field.widget.as_checkbox_mut() {
                            cb.set_checked(state.search_whole_word);
                        }
                    }
                    dialog.set_focus("find_field");
                    self.current_dialog = Some(dialog);
                }
                let bounds = Rect::new(state.dialog_x, state.dialog_y, state.dialog_width, state.dialog_height);
                self.cached_layout = Some(compute_layout(&replace_dialog_layout(), bounds));
            }
            DialogType::GoToLine => {
                if self.current_dialog.is_none() {
                    let mut dialog = create_goto_dialog();
                    if let Some(field) = dialog.field_mut("line_field") {
                        if let Some(tf) = field.widget.as_textfield_mut() {
                            tf.set_text(&state.dialog_goto_line);
                            tf.set_cursor_pos(state.dialog_input_cursor);
                        }
                    }
                    dialog.set_focus("line_field");
                    self.current_dialog = Some(dialog);
                }
                let bounds = Rect::new(state.dialog_x, state.dialog_y, state.dialog_width, state.dialog_height);
                self.cached_layout = Some(compute_layout(&goto_line_dialog_layout(), bounds));
            }
            DialogType::NewSub => {
                if self.current_dialog.is_none() {
                    let mut dialog = create_simple_input_dialog("New SUB", "SUB name:");
                    if let Some(field) = dialog.field_mut("input_field") {
                        if let Some(tf) = field.widget.as_textfield_mut() {
                            tf.set_text(&state.dialog_find_text);
                            tf.set_cursor_pos(state.dialog_input_cursor);
                        }
                    }
                    dialog.set_focus("input_field");
                    self.current_dialog = Some(dialog);
                }
                let bounds = Rect::new(state.dialog_x, state.dialog_y, state.dialog_width, state.dialog_height);
                self.cached_layout = Some(compute_layout(&simple_input_dialog_layout(), bounds));
            }
            DialogType::NewFunction => {
                if self.current_dialog.is_none() {
                    let mut dialog = create_simple_input_dialog("New FUNCTION", "FUNCTION:");
                    if let Some(field) = dialog.field_mut("input_field") {
                        if let Some(tf) = field.widget.as_textfield_mut() {
                            tf.set_text(&state.dialog_find_text);
                            tf.set_cursor_pos(state.dialog_input_cursor);
                        }
                    }
                    dialog.set_focus("input_field");
                    self.current_dialog = Some(dialog);
                }
                let bounds = Rect::new(state.dialog_x, state.dialog_y, state.dialog_width, state.dialog_height);
                self.cached_layout = Some(compute_layout(&simple_input_dialog_layout(), bounds));
            }
            DialogType::FindLabel => {
                if self.current_dialog.is_none() {
                    let mut dialog = create_simple_input_dialog("Find Label", "Label:");
                    if let Some(field) = dialog.field_mut("input_field") {
                        if let Some(tf) = field.widget.as_textfield_mut() {
                            tf.set_text(&state.dialog_find_text);
                            tf.set_cursor_pos(state.dialog_input_cursor);
                        }
                    }
                    dialog.set_focus("input_field");
                    self.current_dialog = Some(dialog);
                }
                let bounds = Rect::new(state.dialog_x, state.dialog_y, state.dialog_width, state.dialog_height);
                self.cached_layout = Some(compute_layout(&simple_input_dialog_layout(), bounds));
            }
            DialogType::CommandArgs => {
                if self.current_dialog.is_none() {
                    let mut dialog = create_simple_input_dialog("Modify COMMAND$", "Command:");
                    if let Some(field) = dialog.field_mut("input_field") {
                        if let Some(tf) = field.widget.as_textfield_mut() {
                            tf.set_text(&state.command_args);
                            tf.set_cursor_pos(state.dialog_input_cursor);
                        }
                    }
                    dialog.set_focus("input_field");
                    self.current_dialog = Some(dialog);
                }
                let bounds = Rect::new(state.dialog_x, state.dialog_y, state.dialog_width, state.dialog_height);
                self.cached_layout = Some(compute_layout(&simple_input_dialog_layout(), bounds));
            }
            DialogType::HelpPath => {
                if self.current_dialog.is_none() {
                    let mut dialog = create_simple_input_dialog("Help Path", "Path:");
                    if let Some(field) = dialog.field_mut("input_field") {
                        if let Some(tf) = field.widget.as_textfield_mut() {
                            tf.set_text(&state.help_path);
                            tf.set_cursor_pos(state.dialog_input_cursor);
                        }
                    }
                    dialog.set_focus("input_field");
                    self.current_dialog = Some(dialog);
                }
                let bounds = Rect::new(state.dialog_x, state.dialog_y, state.dialog_width, state.dialog_height);
                self.cached_layout = Some(compute_layout(&simple_input_dialog_layout(), bounds));
            }
            // File dialogs, Help, About, etc. use existing drawing code
            _ => {
                self.current_dialog = None;
                self.cached_layout = None;
            }
        }
    }

    /// Handle an event and return the action
    pub fn handle_event(&mut self, event: &InputEvent, _state: &AppState) -> DialogAction {
        // Handle escape key globally
        if matches!(event, InputEvent::Escape) {
            return DialogAction::Cancel;
        }

        // For dialogs with widget support
        if let Some(dialog) = &mut self.current_dialog {
            if let Some(layout) = &self.cached_layout {
                let result = dialog.handle_event(event, layout);

                // Map event results to actions
                match result {
                    EventResult::Action(action_name) => {
                        return self.map_action(&action_name);
                    }
                    EventResult::Consumed => return DialogAction::None,
                    EventResult::Ignored => {}
                }
            }
        }

        DialogAction::None
    }

    /// Map an action name string to a DialogAction
    fn map_action(&self, action_name: &str) -> DialogAction {
        match action_name {
            "find" => DialogAction::Find,
            "find_next" => DialogAction::FindNext,
            "replace" => DialogAction::Replace,
            "replace_all" => DialogAction::ReplaceAll,
            "cancel" => DialogAction::Cancel,
            "ok" => DialogAction::Ok,
            "yes" => DialogAction::Yes,
            "no" => DialogAction::No,
            "toggle_case" => DialogAction::ToggleCase,
            "toggle_whole" => DialogAction::ToggleWholeWord,
            _ => DialogAction::None,
        }
    }

    /// Sync widget state back to AppState
    pub fn sync_to_state(&self, state: &mut AppState) {
        if let Some(dialog) = &self.current_dialog {
            // Sync text fields back
            match &state.dialog {
                DialogType::Find => {
                    if let Some(field) = dialog.field("find_field") {
                        if let Some(tf) = field.widget.as_textfield() {
                            state.dialog_find_text = tf.text().to_string();
                            state.dialog_input_cursor = tf.cursor_pos();
                        }
                    }
                    if let Some(field) = dialog.field("case_checkbox") {
                        if let Some(cb) = field.widget.as_checkbox() {
                            state.search_case_sensitive = cb.checked();
                        }
                    }
                    if let Some(field) = dialog.field("whole_checkbox") {
                        if let Some(cb) = field.widget.as_checkbox() {
                            state.search_whole_word = cb.checked();
                        }
                    }
                }
                DialogType::Replace => {
                    if let Some(field) = dialog.field("find_field") {
                        if let Some(tf) = field.widget.as_textfield() {
                            state.dialog_find_text = tf.text().to_string();
                        }
                    }
                    if let Some(field) = dialog.field("replace_field") {
                        if let Some(tf) = field.widget.as_textfield() {
                            state.dialog_replace_text = tf.text().to_string();
                        }
                    }
                    if let Some(field) = dialog.field("case_checkbox") {
                        if let Some(cb) = field.widget.as_checkbox() {
                            state.search_case_sensitive = cb.checked();
                        }
                    }
                    if let Some(field) = dialog.field("whole_checkbox") {
                        if let Some(cb) = field.widget.as_checkbox() {
                            state.search_whole_word = cb.checked();
                        }
                    }
                }
                DialogType::GoToLine => {
                    if let Some(field) = dialog.field("line_field") {
                        if let Some(tf) = field.widget.as_textfield() {
                            state.dialog_goto_line = tf.text().to_string();
                            state.dialog_input_cursor = tf.cursor_pos();
                        }
                    }
                }
                DialogType::NewSub | DialogType::NewFunction | DialogType::FindLabel => {
                    if let Some(field) = dialog.field("input_field") {
                        if let Some(tf) = field.widget.as_textfield() {
                            state.dialog_find_text = tf.text().to_string();
                            state.dialog_input_cursor = tf.cursor_pos();
                        }
                    }
                }
                DialogType::CommandArgs => {
                    if let Some(field) = dialog.field("input_field") {
                        if let Some(tf) = field.widget.as_textfield() {
                            state.command_args = tf.text().to_string();
                            state.dialog_input_cursor = tf.cursor_pos();
                        }
                    }
                }
                DialogType::HelpPath => {
                    if let Some(field) = dialog.field("input_field") {
                        if let Some(tf) = field.widget.as_textfield() {
                            state.help_path = tf.text().to_string();
                            state.dialog_input_cursor = tf.cursor_pos();
                        }
                    }
                }
                _ => {}
            }
        }
    }

    /// Clear the current dialog widget
    pub fn clear(&mut self) {
        self.current_dialog = None;
        self.cached_layout = None;
    }
}

impl Default for DialogManager {
    fn default() -> Self {
        Self::new()
    }
}
