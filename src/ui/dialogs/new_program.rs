//! New Program confirmation dialog.

use crate::input::InputEvent;
use crate::screen::Screen;
use crate::state::AppState;
use crate::ui::theme::Theme;
use crate::ui::widget::EventResult;
use crate::ui::widget_tree::WidgetNode;
use crate::ui::widgets::{Button, Label, Spacer};

use super::{DialogContext, DialogController, DialogResult, DialogWidget};

const NEW_PROGRAM_TITLE: &str = "New Program";
const NEW_PROGRAM_TEXT: &str = "Current program will be cleared.\nSave it first?";

pub struct NewProgramDialog {
    dialog: DialogWidget,
    open: bool,
}

impl NewProgramDialog {
    pub fn new() -> Self {
        let lines: Vec<String> = NEW_PROGRAM_TEXT.lines().map(|s| s.to_string()).collect();
        let content = Self::build_content(&lines);
        let mut dialog = DialogWidget::with_theme(NEW_PROGRAM_TITLE, content, Theme::qbasic_dialog())
            .with_size(40, 8)
            .with_min_size(30, 6);
        dialog.set_show_maximize(false);
        Self { dialog, open: false }
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
}

impl NewProgramDialog {
    fn save_then_clear(&self, ctx: &mut DialogContext) {
        // Save current file first
        if let Some(path) = &ctx.state.file_path {
            let content = ctx.editor.content();
            if let Err(e) = std::fs::write(path, &content) {
                ctx.state.set_status(format!("Error saving: {}", e));
                return;
            }
        }
        // Then clear
        ctx.editor.clear();
        ctx.state.file_path = None;
        ctx.state.set_modified(false);
        ctx.state.set_status("New program");
    }

    fn discard_and_clear(&self, ctx: &mut DialogContext) {
        ctx.editor.clear();
        ctx.state.file_path = None;
        ctx.state.set_modified(false);
        ctx.state.set_status("New program");
    }
}

impl DialogController for NewProgramDialog {

    fn open(&mut self, ctx: &mut DialogContext) {
        self.open = true;
        self.dialog.focus_first();
        self.dialog.center();
        ctx.state.focus_dialog();
    }

    fn is_open(&self) -> bool {
        self.open
    }

    fn close(&mut self) {
        self.open = false;
    }

    fn set_screen_size(&mut self, width: u16, height: u16) {
        self.dialog.set_screen_size(width, height);
    }

    fn draw(&mut self, screen: &mut Screen, _state: &AppState) {
        if !self.open {
            return;
        }
        self.dialog.center();
        self.dialog.draw_with_theme(screen);
    }

    fn handle_event(&mut self, event: &InputEvent, ctx: &mut DialogContext) -> DialogResult {
        if !self.open {
            return DialogResult::Open;
        }
        let result = self.dialog.handle_event(event);
        if let EventResult::Action(action) = result {
            match action.as_str() {
                "yes" => {
                    self.save_then_clear(ctx);
                    return DialogResult::Closed;
                }
                "no" => {
                    self.discard_and_clear(ctx);
                    return DialogResult::Closed;
                }
                "cancel" | "dialog_cancel" => return DialogResult::Closed,
                _ => {}
            }
        }
        DialogResult::Open
    }
}
