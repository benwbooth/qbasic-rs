//! Dialog widget system
#![allow(dead_code)]
//!
//! This module provides widget-based dialog handling, replacing the ad-hoc
//! event handling in main.rs with a proper component-based system.

use crate::input::InputEvent;
use crate::screen::Screen;
use crate::terminal::Color;
use super::layout::Rect;
use super::widget::{Widget, EventResult};
use super::textfield::TextField;
use super::button::Button;
use super::listview::ListView;

/// A checkbox widget
pub struct Checkbox {
    label: String,
    checked: bool,
    focused: bool,
    action_name: String,
}

impl Checkbox {
    pub fn new(label: impl Into<String>, action_name: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            checked: false,
            focused: false,
            action_name: action_name.into(),
        }
    }

    pub fn checked(&self) -> bool {
        self.checked
    }

    pub fn set_checked(&mut self, checked: bool) {
        self.checked = checked;
    }

    pub fn toggle(&mut self) {
        self.checked = !self.checked;
    }
}

impl Widget for Checkbox {
    fn draw(&self, screen: &mut Screen, bounds: Rect) {
        if bounds.width < 4 || bounds.height < 1 {
            return;
        }

        let (fg, bg) = if self.focused {
            (Color::White, Color::Black)
        } else {
            (Color::Black, Color::LightGray)
        };

        let mark = if self.checked { "X" } else { " " };
        let text = format!("[{}] {}", mark, self.label);

        for (i, ch) in text.chars().take(bounds.width as usize).enumerate() {
            screen.set(bounds.y, bounds.x + i as u16, ch, fg, bg);
        }
    }

    fn handle_event(&mut self, event: &InputEvent, bounds: Rect) -> EventResult {
        if self.focused {
            match event {
                InputEvent::Enter | InputEvent::Char(' ') => {
                    self.toggle();
                    return EventResult::Action(self.action_name.clone());
                }
                _ => {}
            }
        }

        // Handle mouse click
        if let InputEvent::MouseClick { row, col } = event {
            if bounds.contains(*row, *col) {
                self.toggle();
                return EventResult::Action(self.action_name.clone());
            }
        }

        EventResult::Ignored
    }

    fn focusable(&self) -> bool {
        true
    }

    fn has_focus(&self) -> bool {
        self.focused
    }

    fn set_focus(&mut self, focused: bool) {
        self.focused = focused;
    }
}

/// A radio button widget
pub struct RadioButton {
    label: String,
    selected: bool,
    focused: bool,
    action_name: String,
}

impl RadioButton {
    pub fn new(label: impl Into<String>, action_name: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            selected: false,
            focused: false,
            action_name: action_name.into(),
        }
    }

    pub fn selected(&self) -> bool {
        self.selected
    }

    pub fn set_selected(&mut self, selected: bool) {
        self.selected = selected;
    }
}

impl Widget for RadioButton {
    fn draw(&self, screen: &mut Screen, bounds: Rect) {
        if bounds.width < 4 || bounds.height < 1 {
            return;
        }

        let (fg, bg) = if self.focused {
            (Color::White, Color::Black)
        } else {
            (Color::Black, Color::LightGray)
        };

        let mark = if self.selected { "o" } else { " " };
        let text = format!("({}) {}", mark, self.label);

        for (i, ch) in text.chars().take(bounds.width as usize).enumerate() {
            screen.set(bounds.y, bounds.x + i as u16, ch, fg, bg);
        }
    }

    fn handle_event(&mut self, event: &InputEvent, bounds: Rect) -> EventResult {
        if self.focused {
            match event {
                InputEvent::Enter | InputEvent::Char(' ') => {
                    return EventResult::Action(self.action_name.clone());
                }
                _ => {}
            }
        }

        // Handle mouse click
        if let InputEvent::MouseClick { row, col } = event {
            if bounds.contains(*row, *col) {
                return EventResult::Action(self.action_name.clone());
            }
        }

        EventResult::Ignored
    }

    fn focusable(&self) -> bool {
        true
    }

    fn has_focus(&self) -> bool {
        self.focused
    }

    fn set_focus(&mut self, focused: bool) {
        self.focused = focused;
    }
}

/// A label widget (non-interactive text)
pub struct Label {
    text: String,
    highlight: bool,
}

impl Label {
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            highlight: false,
        }
    }

    pub fn set_text(&mut self, text: impl Into<String>) {
        self.text = text.into();
    }

    pub fn set_highlight(&mut self, highlight: bool) {
        self.highlight = highlight;
    }
}

impl Widget for Label {
    fn draw(&self, screen: &mut Screen, bounds: Rect) {
        let (fg, bg) = if self.highlight {
            (Color::White, Color::Black)
        } else {
            (Color::Black, Color::LightGray)
        };

        for (i, ch) in self.text.chars().take(bounds.width as usize).enumerate() {
            screen.set(bounds.y, bounds.x + i as u16, ch, fg, bg);
        }
    }

    fn handle_event(&mut self, _event: &InputEvent, _bounds: Rect) -> EventResult {
        EventResult::Ignored
    }
}

/// Enumeration of widget types for a dialog
pub enum DialogWidget {
    Label(Label),
    TextField(TextField),
    Button(Button),
    Checkbox(Checkbox),
    RadioButton(RadioButton),
    ListView(ListView),
}

impl DialogWidget {
    pub fn as_widget(&self) -> &dyn Widget {
        match self {
            DialogWidget::Label(w) => w,
            DialogWidget::TextField(w) => w,
            DialogWidget::Button(w) => w,
            DialogWidget::Checkbox(w) => w,
            DialogWidget::RadioButton(w) => w,
            DialogWidget::ListView(w) => w,
        }
    }

    pub fn as_widget_mut(&mut self) -> &mut dyn Widget {
        match self {
            DialogWidget::Label(w) => w,
            DialogWidget::TextField(w) => w,
            DialogWidget::Button(w) => w,
            DialogWidget::Checkbox(w) => w,
            DialogWidget::RadioButton(w) => w,
            DialogWidget::ListView(w) => w,
        }
    }

    pub fn as_textfield(&self) -> Option<&TextField> {
        match self {
            DialogWidget::TextField(tf) => Some(tf),
            _ => None,
        }
    }

    pub fn as_textfield_mut(&mut self) -> Option<&mut TextField> {
        match self {
            DialogWidget::TextField(tf) => Some(tf),
            _ => None,
        }
    }

    pub fn as_checkbox(&self) -> Option<&Checkbox> {
        match self {
            DialogWidget::Checkbox(cb) => Some(cb),
            _ => None,
        }
    }

    pub fn as_checkbox_mut(&mut self) -> Option<&mut Checkbox> {
        match self {
            DialogWidget::Checkbox(cb) => Some(cb),
            _ => None,
        }
    }

    pub fn as_listview(&self) -> Option<&ListView> {
        match self {
            DialogWidget::ListView(lv) => Some(lv),
            _ => None,
        }
    }

    pub fn as_listview_mut(&mut self) -> Option<&mut ListView> {
        match self {
            DialogWidget::ListView(lv) => Some(lv),
            _ => None,
        }
    }
}

/// A field in a dialog with its layout id and widget
pub struct DialogField {
    pub id: String,
    pub widget: DialogWidget,
}

impl DialogField {
    pub fn new(id: impl Into<String>, widget: DialogWidget) -> Self {
        Self {
            id: id.into(),
            widget,
        }
    }

    pub fn label(id: impl Into<String>, text: impl Into<String>) -> Self {
        Self::new(id, DialogWidget::Label(Label::new(text)))
    }

    pub fn textfield(id: impl Into<String>, action_prefix: impl Into<String>) -> Self {
        Self::new(id, DialogWidget::TextField(TextField::new(action_prefix)))
    }

    pub fn button(id: impl Into<String>, label: impl Into<String>, action: impl Into<String>) -> Self {
        Self::new(id, DialogWidget::Button(Button::new(label, action)))
    }

    pub fn checkbox(id: impl Into<String>, label: impl Into<String>, action: impl Into<String>) -> Self {
        Self::new(id, DialogWidget::Checkbox(Checkbox::new(label, action)))
    }

    pub fn radio(id: impl Into<String>, label: impl Into<String>, action: impl Into<String>) -> Self {
        Self::new(id, DialogWidget::RadioButton(RadioButton::new(label, action)))
    }

    pub fn listview(id: impl Into<String>, action_prefix: impl Into<String>) -> Self {
        Self::new(id, DialogWidget::ListView(ListView::new(action_prefix)))
    }
}

/// A composite dialog containing multiple fields
pub struct CompositeDialog {
    title: String,
    fields: Vec<DialogField>,
    focus_order: Vec<usize>,  // Indices into fields for focusable widgets
    focused_index: usize,
}

impl CompositeDialog {
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            fields: Vec::new(),
            focus_order: Vec::new(),
            focused_index: 0,
        }
    }

    pub fn add_field(&mut self, field: DialogField) {
        let idx = self.fields.len();
        if field.widget.as_widget().focusable() {
            self.focus_order.push(idx);
        }
        self.fields.push(field);
    }

    pub fn with_field(mut self, field: DialogField) -> Self {
        self.add_field(field);
        self
    }

    pub fn title(&self) -> &str {
        &self.title
    }

    pub fn field(&self, id: &str) -> Option<&DialogField> {
        self.fields.iter().find(|f| f.id == id)
    }

    pub fn field_mut(&mut self, id: &str) -> Option<&mut DialogField> {
        self.fields.iter_mut().find(|f| f.id == id)
    }

    pub fn fields(&self) -> &[DialogField] {
        &self.fields
    }

    pub fn fields_mut(&mut self) -> &mut [DialogField] {
        &mut self.fields
    }

    /// Get the currently focused field
    pub fn focused_field(&self) -> Option<&DialogField> {
        self.focus_order.get(self.focused_index)
            .and_then(|&idx| self.fields.get(idx))
    }

    pub fn focused_field_mut(&mut self) -> Option<&mut DialogField> {
        let idx = self.focus_order.get(self.focused_index).copied()?;
        self.fields.get_mut(idx)
    }

    pub fn focused_id(&self) -> Option<&str> {
        self.focused_field().map(|f| f.id.as_str())
    }

    /// Get the focused index in the focus order
    pub fn focused_index(&self) -> usize {
        self.focused_index
    }

    /// Set focus by field id
    pub fn set_focus(&mut self, id: &str) {
        // Clear current focus
        if let Some(&idx) = self.focus_order.get(self.focused_index) {
            if let Some(field) = self.fields.get_mut(idx) {
                field.widget.as_widget_mut().set_focus(false);
            }
        }

        // Find and set new focus
        for (i, &field_idx) in self.focus_order.iter().enumerate() {
            if self.fields.get(field_idx).map(|f| f.id.as_str()) == Some(id) {
                self.focused_index = i;
                if let Some(field) = self.fields.get_mut(field_idx) {
                    field.widget.as_widget_mut().set_focus(true);
                }
                break;
            }
        }
    }

    /// Move focus to next focusable field
    pub fn focus_next(&mut self) {
        if self.focus_order.is_empty() {
            return;
        }

        // Clear current focus
        if let Some(&idx) = self.focus_order.get(self.focused_index) {
            if let Some(field) = self.fields.get_mut(idx) {
                field.widget.as_widget_mut().set_focus(false);
            }
        }

        // Move to next
        self.focused_index = (self.focused_index + 1) % self.focus_order.len();

        // Set new focus
        if let Some(&idx) = self.focus_order.get(self.focused_index) {
            if let Some(field) = self.fields.get_mut(idx) {
                field.widget.as_widget_mut().set_focus(true);
            }
        }
    }

    /// Move focus to previous focusable field
    pub fn focus_prev(&mut self) {
        if self.focus_order.is_empty() {
            return;
        }

        // Clear current focus
        if let Some(&idx) = self.focus_order.get(self.focused_index) {
            if let Some(field) = self.fields.get_mut(idx) {
                field.widget.as_widget_mut().set_focus(false);
            }
        }

        // Move to previous
        self.focused_index = if self.focused_index == 0 {
            self.focus_order.len() - 1
        } else {
            self.focused_index - 1
        };

        // Set new focus
        if let Some(&idx) = self.focus_order.get(self.focused_index) {
            if let Some(field) = self.fields.get_mut(idx) {
                field.widget.as_widget_mut().set_focus(true);
            }
        }
    }

    /// Handle an event, returning the result
    pub fn handle_event(&mut self, event: &InputEvent, layout: &super::layout::ComputedLayout) -> EventResult {
        // Handle Tab/Shift+Tab for focus navigation
        match event {
            InputEvent::Tab => {
                self.focus_next();
                return EventResult::Consumed;
            }
            InputEvent::ShiftTab => {
                self.focus_prev();
                return EventResult::Consumed;
            }
            _ => {}
        }

        // Try to handle with focused widget first
        if let Some(&idx) = self.focus_order.get(self.focused_index) {
            if let Some(field) = self.fields.get_mut(idx) {
                if let Some(bounds) = layout.get(&field.id) {
                    let result = field.widget.as_widget_mut().handle_event(event, bounds);
                    if result.is_consumed() {
                        return result;
                    }
                }
            }
        }

        // Check for mouse clicks on other widgets
        if let InputEvent::MouseClick { row, col } = event {
            // First, find which widget was clicked
            let mut clicked_info: Option<(usize, Rect)> = None;
            for (i, &field_idx) in self.focus_order.iter().enumerate() {
                if let Some(field) = self.fields.get(field_idx) {
                    if let Some(bounds) = layout.get(&field.id) {
                        if bounds.contains(*row, *col) {
                            clicked_info = Some((i, bounds));
                            break;
                        }
                    }
                }
            }

            // Then handle the click with separate borrows
            if let Some((new_focus_idx, bounds)) = clicked_info {
                // Clear old focus
                if let Some(&old_idx) = self.focus_order.get(self.focused_index) {
                    if let Some(old_field) = self.fields.get_mut(old_idx) {
                        old_field.widget.as_widget_mut().set_focus(false);
                    }
                }

                // Set new focus and handle click
                self.focused_index = new_focus_idx;
                if let Some(&field_idx) = self.focus_order.get(new_focus_idx) {
                    if let Some(field) = self.fields.get_mut(field_idx) {
                        field.widget.as_widget_mut().set_focus(true);
                        let result = field.widget.as_widget_mut().handle_event(event, bounds);
                        if result.is_consumed() {
                            return result;
                        }
                        return EventResult::Consumed;
                    }
                }
            }
        }

        EventResult::Ignored
    }

    /// Draw all widgets using the layout
    pub fn draw(&self, screen: &mut Screen, layout: &super::layout::ComputedLayout) {
        for field in &self.fields {
            if let Some(bounds) = layout.get(&field.id) {
                field.widget.as_widget().draw(screen, bounds);
            }
        }
    }
}

// ============================================================================
// Factory functions for standard dialogs
// ============================================================================

/// Create a Find dialog
pub fn create_find_dialog() -> CompositeDialog {
    CompositeDialog::new("Find")
        .with_field(DialogField::label("find_label", "Find:"))
        .with_field(DialogField::textfield("find_field", "find"))
        .with_field(DialogField::checkbox("case_checkbox", "Match Case", "toggle_case"))
        .with_field(DialogField::checkbox("whole_checkbox", "Whole Word", "toggle_whole"))
        .with_field(DialogField::button("ok_button", "Find", "find"))
        .with_field(DialogField::button("cancel_button", "Cancel", "cancel"))
        .with_field(DialogField::button("help_button", "Help", "help"))
}

/// Create a Replace dialog
pub fn create_replace_dialog() -> CompositeDialog {
    CompositeDialog::new("Change")
        .with_field(DialogField::label("find_label", "Find What:"))
        .with_field(DialogField::textfield("find_field", "find"))
        .with_field(DialogField::label("replace_label", "Change To:"))
        .with_field(DialogField::textfield("replace_field", "replace"))
        .with_field(DialogField::checkbox("case_checkbox", "Match Case", "toggle_case"))
        .with_field(DialogField::checkbox("whole_checkbox", "Whole Word", "toggle_whole"))
        .with_field(DialogField::button("find_next_button", "Find Next", "find_next"))
        .with_field(DialogField::button("replace_button", "Replace", "replace"))
        .with_field(DialogField::button("replace_all_button", "Replace All", "replace_all"))
        .with_field(DialogField::button("cancel_button", "Cancel", "cancel"))
}

/// Create a GoToLine dialog
pub fn create_goto_dialog() -> CompositeDialog {
    CompositeDialog::new("Go To Line")
        .with_field(DialogField::label("line_label", "Line number:"))
        .with_field(DialogField::textfield("line_field", "line"))
        .with_field(DialogField::button("ok_button", "OK", "ok"))
        .with_field(DialogField::button("cancel_button", "Cancel", "cancel"))
}

/// Create a simple input dialog (for NewSub, NewFunction, FindLabel, etc.)
pub fn create_simple_input_dialog(title: &str, label: &str) -> CompositeDialog {
    CompositeDialog::new(title)
        .with_field(DialogField::label("input_label", label))
        .with_field(DialogField::textfield("input_field", "input"))
        .with_field(DialogField::button("ok_button", "OK", "ok"))
        .with_field(DialogField::button("cancel_button", "Cancel", "cancel"))
}

/// Create a confirmation dialog
pub fn create_confirm_dialog(title: &str, message: &str) -> CompositeDialog {
    CompositeDialog::new(title)
        .with_field(DialogField::label("message", message))
        .with_field(DialogField::button("yes_button", "Yes", "yes"))
        .with_field(DialogField::button("no_button", "No", "no"))
        .with_field(DialogField::button("cancel_button", "Cancel", "cancel"))
}

/// Create a message dialog (OK only)
pub fn create_message_dialog(title: &str, message: &str) -> CompositeDialog {
    CompositeDialog::new(title)
        .with_field(DialogField::label("message", message))
        .with_field(DialogField::button("ok_button", "OK", "ok"))
}
