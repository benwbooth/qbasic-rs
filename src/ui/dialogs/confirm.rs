//! Confirm dialog.

use crate::input::InputEvent;
use crate::screen::Screen;
use crate::state::AppState;
use crate::ui::theme::Theme;
use crate::ui::widget::EventResult;
use crate::ui::widget_tree::WidgetNode;
use crate::ui::widgets::{Button, Label, Spacer};

use super::{DialogContext, DialogController, DialogResult, DialogWidget};

pub struct ConfirmDialog {
    dialog: Option<DialogWidget>,
    title: String,
    text: String,
    open: bool,
    /// Result of the dialog: true if user clicked Yes, false if No/Cancel
    pub confirmed: bool,
}

impl ConfirmDialog {
    pub fn new() -> Self {
        Self {
            dialog: None,
            title: String::new(),
            text: String::new(),
            open: false,
            confirmed: false,
        }
    }

    /// Set the confirm dialog title and text (call before open)
    pub fn set_message(&mut self, title: String, text: String) {
        self.title = title;
        self.text = text;
        self.dialog = None; // Force rebuild
    }

    fn build_dialog(&mut self) {
        let lines: Vec<String> = self.text.lines().map(|s| s.to_string()).collect();
        let content = Self::build_content(&lines);
        let mut dialog = DialogWidget::with_theme(&self.title, content, Theme::qbasic_dialog())
            .with_size(50, 10)
            .with_min_size(30, 6);
        dialog.set_show_maximize(false);
        dialog.set_chrome_interactive(false);
        dialog.focus_first();
        self.dialog = Some(dialog);
    }

    fn build_content(lines: &[String]) -> WidgetNode {
        let mut root = WidgetNode::vstack("root").padding(1);
        for (idx, line) in lines.iter().enumerate() {
            let id = format!("line_{}", idx);
            root = root.child(WidgetNode::leaf(id, Label::new(line.clone())));
        }
        root
            .child(WidgetNode::leaf("spacer", Spacer::new()))
            .child(
                WidgetNode::hstack("buttons")
                    .child(WidgetNode::leaf("left_pad", Spacer::fixed(5)))
                    .leaf("yes_button", Button::new("Yes", "yes").min_width(7))
                    .child(WidgetNode::leaf("gap1", Spacer::fixed(2)))
                    .leaf("no_button", Button::new("No", "no").min_width(7))
                    .child(WidgetNode::leaf("gap2", Spacer::fixed(2)))
                    .leaf("cancel_button", Button::new("Cancel", "cancel").min_width(11))
                    .child(WidgetNode::leaf("right_spacer", Spacer::new()))
                    .spacing(0)
                    .build(),
            )
            .build()
    }

    fn ensure_dialog(&mut self) {
        if self.dialog.is_none() {
            self.build_dialog();
        }
    }
}

impl DialogController for ConfirmDialog {

    fn open(&mut self, ctx: &mut DialogContext) {
        self.open = true;
        self.ensure_dialog();
        if let Some(ref mut dialog) = self.dialog {
            dialog.focus_first();
            dialog.center();
        }
        ctx.state.focus_dialog();
    }

    fn is_open(&self) -> bool {
        self.open
    }

    fn close(&mut self) {
        self.open = false;
    }

    fn set_screen_size(&mut self, width: u16, height: u16) {
        if let Some(ref mut dialog) = self.dialog {
            dialog.set_screen_size(width, height);
        }
    }

    fn draw(&mut self, screen: &mut Screen, _state: &AppState) {
        if !self.open {
            return;
        }
        self.ensure_dialog();
        if let Some(ref mut dialog) = self.dialog {
            dialog.center();
            dialog.draw_with_theme(screen);
        }
    }

    fn handle_event(&mut self, event: &InputEvent, _ctx: &mut DialogContext) -> DialogResult {
        if !self.open {
            return DialogResult::Open;
        }
        self.ensure_dialog();
        if let Some(ref mut dialog) = self.dialog {
            let result = dialog.handle_event(event);
            if let EventResult::Action(action) = result {
                match action.as_str() {
                    "yes" => {
                        self.confirmed = true;
                        return DialogResult::Closed;
                    }
                    "no" | "cancel" | "dialog_cancel" => {
                        self.confirmed = false;
                        return DialogResult::Closed;
                    }
                    _ => {}
                }
            }
        }
        DialogResult::Open
    }
}
