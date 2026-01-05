//! Simple input dialogs (NewSub, NewFunction, FindLabel, CommandArgs, HelpPath).
//!
//! Each is a separate struct implementing DialogController, sharing common UI building code.

use crate::input::InputEvent;
use crate::screen::Screen;
use crate::state::AppState;
use crate::ui::theme::Theme;
use crate::ui::widget::EventResult;
use crate::ui::widget_tree::WidgetNode;
use crate::ui::widgets::{Button, Label, Spacer, TextField};

use super::{DialogContext, DialogController, DialogResult, DialogWidget};

// Helper to build the common input dialog content
fn build_content(label_text: &str) -> WidgetNode {
    WidgetNode::vstack("root")
        .padding(1)
        .child(
            WidgetNode::hstack("input_row")
                .leaf("input_label", Label::new(label_text).min_width(12))
                .leaf("input_field", TextField::new("input"))
                .spacing(0)
                .build(),
        )
        .child(WidgetNode::leaf("spacer1", Spacer::fixed(1)))
        .child(
            WidgetNode::hstack("buttons_row")
                .child(WidgetNode::leaf("btn_spacer_left", Spacer::new()))
                .leaf("ok_button", Button::new("OK", "ok").min_width(8))
                .leaf("cancel_button", Button::new("Cancel", "cancel").min_width(10))
                .child(WidgetNode::leaf("btn_spacer_right", Spacer::new()))
                .spacing(2)
                .build(),
        )
        .build()
}

// Common methods for simple input dialogs
trait SimpleInputCommon {
    fn dialog(&self) -> &DialogWidget;
    fn dialog_mut(&mut self) -> &mut DialogWidget;

    fn sync_focus_decor(&mut self) {
        let focus_path = self.dialog().content().focus_path();
        let focused = focus_path.iter().any(|id| id == "input_field");
        if let Some(label) = self.dialog_mut().content_mut()
            .get_widget_mut(&["root", "input_row", "input_label"])
            .and_then(|w| w.as_any_mut().downcast_mut::<Label>())
        {
            label.set_highlight(focused);
        }
    }

    fn get_input_text(&self) -> String {
        self.dialog().content()
            .get_widget(&["root", "input_row", "input_field"])
            .and_then(|w| w.as_any().downcast_ref::<TextField>())
            .map(|tf| tf.text().to_string())
            .unwrap_or_default()
    }

    fn set_input_text(&mut self, text: &str) {
        if let Some(tf) = self.dialog_mut().content_mut()
            .get_widget_mut(&["root", "input_row", "input_field"])
            .and_then(|w| w.as_any_mut().downcast_mut::<TextField>())
        {
            tf.set_text(text);
            tf.set_cursor_pos(text.chars().count());
        }
    }

    fn clear_input(&mut self) {
        self.set_input_text("");
    }
}

// ============= NewSubDialog =============

pub struct NewSubDialog {
    dialog: DialogWidget,
    open: bool,
}

impl NewSubDialog {
    pub fn new() -> Self {
        let content = build_content("SUB name:");
        let mut dialog = DialogWidget::with_theme("New SUB", content, Theme::qbasic_dialog())
            .with_size(45, 7)
            .with_min_size(30, 7);
        dialog.set_show_maximize(false);
        Self { dialog, open: false }
    }

    fn insert_new_sub(&self, ctx: &mut DialogContext) {
        let name = self.get_input_text().trim().to_string();
        if name.is_empty() {
            ctx.state.set_status("SUB name cannot be empty");
            return;
        }

        // Insert SUB block at end of file
        let sub_block = format!("\n\nSUB {}\n    \nEND SUB", name);
        let line_count = ctx.editor.buffer.line_count();

        // Go to end of file and insert the SUB block
        ctx.editor.go_to_line(line_count);
        if let Some(line) = ctx.editor.buffer.line(ctx.editor.cursor_line) {
            ctx.editor.cursor_col = line.len();
        }
        ctx.editor.insert_text(&sub_block);

        // Move cursor to inside the SUB (the blank line)
        ctx.editor.go_to_line(line_count + 2);
        ctx.editor.cursor_col = 4;

        ctx.state.set_modified(true);
        ctx.state.set_status(format!("Created SUB {}", name));
    }
}

impl SimpleInputCommon for NewSubDialog {
    fn dialog(&self) -> &DialogWidget { &self.dialog }
    fn dialog_mut(&mut self) -> &mut DialogWidget { &mut self.dialog }
}

impl DialogController for NewSubDialog {
    fn open(&mut self, ctx: &mut DialogContext) {
        self.open = true;
        self.clear_input();
        self.dialog.focus_first();
        self.dialog.center();
        ctx.state.focus_dialog();
    }

    fn is_open(&self) -> bool { self.open }
    fn close(&mut self) { self.open = false; }

    fn set_screen_size(&mut self, width: u16, height: u16) {
        self.dialog.set_screen_size(width, height);
    }

    fn draw(&mut self, screen: &mut Screen, _state: &AppState) {
        if !self.open { return; }
        self.sync_focus_decor();
        self.dialog.center();
        self.dialog.draw_with_theme(screen);
    }

    fn handle_event(&mut self, event: &InputEvent, ctx: &mut DialogContext) -> DialogResult {
        if !self.open { return DialogResult::Open; }
        let result = self.dialog.handle_event(event);
        self.sync_focus_decor();
        if let EventResult::Action(a) = result {
            match a.as_str() {
                "ok" | "input_submit" => {
                    self.insert_new_sub(ctx);
                    return DialogResult::Closed;
                }
                "cancel" | "dialog_cancel" => return DialogResult::Closed,
                _ => {}
            }
        }
        DialogResult::Open
    }
}

// ============= NewFunctionDialog =============

pub struct NewFunctionDialog {
    dialog: DialogWidget,
    open: bool,
}

impl NewFunctionDialog {
    pub fn new() -> Self {
        let content = build_content("FUNCTION:");
        let mut dialog = DialogWidget::with_theme("New FUNCTION", content, Theme::qbasic_dialog())
            .with_size(45, 7)
            .with_min_size(30, 7);
        dialog.set_show_maximize(false);
        Self { dialog, open: false }
    }

    fn insert_new_function(&self, ctx: &mut DialogContext) {
        let name = self.get_input_text().trim().to_string();
        if name.is_empty() {
            ctx.state.set_status("FUNCTION name cannot be empty");
            return;
        }

        // Insert FUNCTION block at end of file
        let func_block = format!("\n\nFUNCTION {}\n    {} = 0\nEND FUNCTION", name, name);
        let line_count = ctx.editor.buffer.line_count();

        // Go to end of file and insert the FUNCTION block
        ctx.editor.go_to_line(line_count);
        if let Some(line) = ctx.editor.buffer.line(ctx.editor.cursor_line) {
            ctx.editor.cursor_col = line.len();
        }
        ctx.editor.insert_text(&func_block);

        // Move cursor to inside the FUNCTION
        ctx.editor.go_to_line(line_count + 2);
        ctx.editor.cursor_col = 4;

        ctx.state.set_modified(true);
        ctx.state.set_status(format!("Created FUNCTION {}", name));
    }
}

impl SimpleInputCommon for NewFunctionDialog {
    fn dialog(&self) -> &DialogWidget { &self.dialog }
    fn dialog_mut(&mut self) -> &mut DialogWidget { &mut self.dialog }
}

impl DialogController for NewFunctionDialog {
    fn open(&mut self, ctx: &mut DialogContext) {
        self.open = true;
        self.clear_input();
        self.dialog.focus_first();
        self.dialog.center();
        ctx.state.focus_dialog();
    }

    fn is_open(&self) -> bool { self.open }
    fn close(&mut self) { self.open = false; }

    fn set_screen_size(&mut self, width: u16, height: u16) {
        self.dialog.set_screen_size(width, height);
    }

    fn draw(&mut self, screen: &mut Screen, _state: &AppState) {
        if !self.open { return; }
        self.sync_focus_decor();
        self.dialog.center();
        self.dialog.draw_with_theme(screen);
    }

    fn handle_event(&mut self, event: &InputEvent, ctx: &mut DialogContext) -> DialogResult {
        if !self.open { return DialogResult::Open; }
        let result = self.dialog.handle_event(event);
        self.sync_focus_decor();
        if let EventResult::Action(a) = result {
            match a.as_str() {
                "ok" | "input_submit" => {
                    self.insert_new_function(ctx);
                    return DialogResult::Closed;
                }
                "cancel" | "dialog_cancel" => return DialogResult::Closed,
                _ => {}
            }
        }
        DialogResult::Open
    }
}

// ============= FindLabelDialog =============

pub struct FindLabelDialog {
    dialog: DialogWidget,
    open: bool,
}

impl FindLabelDialog {
    pub fn new() -> Self {
        let content = build_content("Label:");
        let mut dialog = DialogWidget::with_theme("Find Label", content, Theme::qbasic_dialog())
            .with_size(45, 7)
            .with_min_size(30, 7);
        dialog.set_show_maximize(false);
        Self { dialog, open: false }
    }

    fn find_label(&self, ctx: &mut DialogContext) {
        let label = self.get_input_text().trim().to_string();
        if label.is_empty() {
            ctx.state.set_status("Label name cannot be empty");
            return;
        }

        // Search for label: (like "10:" or "MyLabel:")
        let label_pattern = format!("{}:", label);

        for (line_idx, line) in ctx.editor.buffer.lines.iter().enumerate() {
            let trimmed = line.trim_start();
            if trimmed.starts_with(&label_pattern) || trimmed == label_pattern.trim_end_matches(':') {
                ctx.editor.go_to_line(line_idx + 1);
                ctx.state.set_status(format!("Found label at line {}", line_idx + 1));
                return;
            }
        }

        ctx.state.set_status(format!("Label '{}' not found", label));
    }
}

impl SimpleInputCommon for FindLabelDialog {
    fn dialog(&self) -> &DialogWidget { &self.dialog }
    fn dialog_mut(&mut self) -> &mut DialogWidget { &mut self.dialog }
}

impl DialogController for FindLabelDialog {
    fn open(&mut self, ctx: &mut DialogContext) {
        self.open = true;
        self.clear_input();
        self.dialog.focus_first();
        self.dialog.center();
        ctx.state.focus_dialog();
    }

    fn is_open(&self) -> bool { self.open }
    fn close(&mut self) { self.open = false; }

    fn set_screen_size(&mut self, width: u16, height: u16) {
        self.dialog.set_screen_size(width, height);
    }

    fn draw(&mut self, screen: &mut Screen, _state: &AppState) {
        if !self.open { return; }
        self.sync_focus_decor();
        self.dialog.center();
        self.dialog.draw_with_theme(screen);
    }

    fn handle_event(&mut self, event: &InputEvent, ctx: &mut DialogContext) -> DialogResult {
        if !self.open { return DialogResult::Open; }
        let result = self.dialog.handle_event(event);
        self.sync_focus_decor();
        if let EventResult::Action(a) = result {
            match a.as_str() {
                "ok" | "input_submit" => {
                    self.find_label(ctx);
                    return DialogResult::Closed;
                }
                "cancel" | "dialog_cancel" => return DialogResult::Closed,
                _ => {}
            }
        }
        DialogResult::Open
    }
}

// ============= CommandArgsDialog =============

pub struct CommandArgsDialog {
    dialog: DialogWidget,
    open: bool,
}

impl CommandArgsDialog {
    pub fn new() -> Self {
        let content = build_content("Command:");
        let mut dialog = DialogWidget::with_theme("Modify COMMAND$", content, Theme::qbasic_dialog())
            .with_size(55, 7)
            .with_min_size(30, 7);
        dialog.set_show_maximize(false);
        Self { dialog, open: false }
    }
}

impl SimpleInputCommon for CommandArgsDialog {
    fn dialog(&self) -> &DialogWidget { &self.dialog }
    fn dialog_mut(&mut self) -> &mut DialogWidget { &mut self.dialog }
}

impl DialogController for CommandArgsDialog {
    fn open(&mut self, ctx: &mut DialogContext) {
        self.open = true;
        self.set_input_text(&ctx.state.command_args);
        self.dialog.focus_first();
        self.dialog.center();
        ctx.state.focus_dialog();
    }

    fn is_open(&self) -> bool { self.open }
    fn close(&mut self) { self.open = false; }

    fn set_screen_size(&mut self, width: u16, height: u16) {
        self.dialog.set_screen_size(width, height);
    }

    fn draw(&mut self, screen: &mut Screen, _state: &AppState) {
        if !self.open { return; }
        self.sync_focus_decor();
        self.dialog.center();
        self.dialog.draw_with_theme(screen);
    }

    fn handle_event(&mut self, event: &InputEvent, ctx: &mut DialogContext) -> DialogResult {
        if !self.open { return DialogResult::Open; }
        let result = self.dialog.handle_event(event);
        self.sync_focus_decor();
        if let EventResult::Action(a) = result {
            match a.as_str() {
                "ok" | "input_submit" => {
                    ctx.state.command_args = self.get_input_text();
                    return DialogResult::Closed;
                }
                "cancel" | "dialog_cancel" => return DialogResult::Closed,
                _ => {}
            }
        }
        DialogResult::Open
    }
}

// ============= HelpPathDialog =============

pub struct HelpPathDialog {
    dialog: DialogWidget,
    open: bool,
}

impl HelpPathDialog {
    pub fn new() -> Self {
        let content = build_content("Path:");
        let mut dialog = DialogWidget::with_theme("Help Path", content, Theme::qbasic_dialog())
            .with_size(55, 7)
            .with_min_size(30, 7);
        dialog.set_show_maximize(false);
        Self { dialog, open: false }
    }
}

impl SimpleInputCommon for HelpPathDialog {
    fn dialog(&self) -> &DialogWidget { &self.dialog }
    fn dialog_mut(&mut self) -> &mut DialogWidget { &mut self.dialog }
}

impl DialogController for HelpPathDialog {
    fn open(&mut self, ctx: &mut DialogContext) {
        self.open = true;
        self.set_input_text(&ctx.state.help_path);
        self.dialog.focus_first();
        self.dialog.center();
        ctx.state.focus_dialog();
    }

    fn is_open(&self) -> bool { self.open }
    fn close(&mut self) { self.open = false; }

    fn set_screen_size(&mut self, width: u16, height: u16) {
        self.dialog.set_screen_size(width, height);
    }

    fn draw(&mut self, screen: &mut Screen, _state: &AppState) {
        if !self.open { return; }
        self.sync_focus_decor();
        self.dialog.center();
        self.dialog.draw_with_theme(screen);
    }

    fn handle_event(&mut self, event: &InputEvent, ctx: &mut DialogContext) -> DialogResult {
        if !self.open { return DialogResult::Open; }
        let result = self.dialog.handle_event(event);
        self.sync_focus_decor();
        if let EventResult::Action(a) = result {
            match a.as_str() {
                "ok" | "input_submit" => {
                    ctx.state.help_path = self.get_input_text();
                    return DialogResult::Closed;
                }
                "cancel" | "dialog_cancel" => return DialogResult::Closed,
                _ => {}
            }
        }
        DialogResult::Open
    }
}
