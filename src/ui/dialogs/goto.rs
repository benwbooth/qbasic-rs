//! Go To Line dialog.

use crate::input::InputEvent;
use crate::screen::Screen;
use crate::state::AppState;
use crate::ui::theme::Theme;
use crate::ui::widget::EventResult;
use crate::ui::widget_tree::WidgetNode;
use crate::ui::widgets::{Button, Label, Spacer, TextField};

use super::{DialogContext, DialogController, DialogResult, DialogWidget};

pub struct GoToDialog {
    dialog: DialogWidget,
    open: bool,
}

impl GoToDialog {
    pub fn new() -> Self {
        let content = Self::build_content();
        let mut dialog = DialogWidget::with_theme("Go To Line", content, Theme::qbasic_dialog())
            .with_size(40, 7)
            .with_min_size(30, 7);
        dialog.set_show_maximize(false);
        dialog.set_chrome_interactive(false);
        Self { dialog, open: false }
    }

    fn build_content() -> WidgetNode {
        WidgetNode::vstack("root")
            .padding(1)
            .child(
                WidgetNode::hstack("line_row")
                    .leaf("line_label", Label::new("Line number:").min_width(14))
                    .leaf("line_field", TextField::new("line"))
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

    fn sync_focus_decor(&mut self) {
        let focus_path = self.dialog.content().focus_path();
        let focused = focus_path.iter().any(|id| id == "line_field");
        if let Some(label) = self.dialog.content_mut()
            .get_widget_mut(&["root", "line_row", "line_label"])
            .and_then(|w| w.as_any_mut().downcast_mut::<Label>())
        {
            label.set_highlight(focused);
        }
    }

    fn get_line_text(&self) -> String {
        self.dialog.content()
            .get_widget(&["root", "line_row", "line_field"])
            .and_then(|w| w.as_any().downcast_ref::<TextField>())
            .map(|tf| tf.text().to_string())
            .unwrap_or_default()
    }

    fn clear_text(&mut self) {
        if let Some(tf) = self.dialog.content_mut()
            .get_widget_mut(&["root", "line_row", "line_field"])
            .and_then(|w| w.as_any_mut().downcast_mut::<TextField>())
        {
            tf.set_text("");
        }
    }

    /// Do the actual go-to-line work
    fn go_to_line(&self, ctx: &mut DialogContext) {
        let line_text = self.get_line_text();
        if let Ok(line_num) = line_text.parse::<usize>() {
            let target = line_num.saturating_sub(1).min(ctx.editor.buffer.line_count().saturating_sub(1));
            ctx.editor.cursor_line = target;
            ctx.editor.cursor_col = 0;
            ctx.editor.ensure_cursor_visible(20, 80);
            ctx.state.set_status(format!("Jumped to line {}", line_num));
        } else {
            ctx.state.set_status("Invalid line number");
        }
    }
}

impl DialogController for GoToDialog {

    fn open(&mut self, ctx: &mut DialogContext) {
        self.open = true;
        self.clear_text();
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
        self.sync_focus_decor();
        self.dialog.center();
        self.dialog.draw_with_theme(screen);
    }

    fn handle_event(&mut self, event: &InputEvent, ctx: &mut DialogContext) -> DialogResult {
        if !self.open {
            return DialogResult::Open;
        }
        let result = self.dialog.handle_event(event);
        self.sync_focus_decor();

        if let EventResult::Action(action) = result {
            match action.as_str() {
                "ok" | "line_submit" => {
                    self.go_to_line(ctx);
                    return DialogResult::Closed;
                }
                "cancel" | "dialog_cancel" => return DialogResult::Closed,
                _ => {}
            }
        }
        DialogResult::Open
    }
}
