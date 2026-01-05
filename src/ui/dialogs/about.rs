//! About dialog.

use crate::input::InputEvent;
use crate::screen::Screen;
use crate::state::AppState;
use crate::ui::theme::Theme;
use crate::ui::widget::EventResult;
use crate::ui::widget_tree::WidgetNode;
use crate::ui::widgets::{Button, Label, Spacer};

use super::{DialogContext, DialogController, DialogResult, DialogWidget};

pub struct AboutDialog {
    dialog: DialogWidget,
    open: bool,
}

impl AboutDialog {
    pub fn new() -> Self {
        let content = Self::build_content();
        let mut dialog = DialogWidget::with_theme("About", content, Theme::qbasic_dialog())
            .with_size(50, 12)
            .with_min_size(30, 8);
        dialog.set_show_maximize(false);
        Self { dialog, open: false }
    }

    fn build_content() -> WidgetNode {
        WidgetNode::vstack("root")
            .padding(1)
            .child(WidgetNode::leaf("line1", Label::new("")))
            .child(WidgetNode::leaf("line2", Label::new("         BASIC-RS")))
            .child(WidgetNode::leaf("line3", Label::new("")))
            .child(WidgetNode::leaf("line4", Label::new("      A BASIC interpreter")))
            .child(WidgetNode::leaf("line5", Label::new("      written in Rust")))
            .child(WidgetNode::leaf("line6", Label::new("")))
            .child(WidgetNode::leaf("line7", Label::new("      Inspired by classic")))
            .child(WidgetNode::leaf("line8", Label::new("      BASIC environments")))
            .child(WidgetNode::leaf("line9", Label::new("")))
            .child(WidgetNode::leaf("spacer", Spacer::new()))
            .child(
                WidgetNode::hstack("buttons")
                    .child(WidgetNode::leaf("left_spacer", Spacer::new()))
                    .leaf("ok_button", Button::new("OK", "ok").min_width(6))
                    .child(WidgetNode::leaf("right_spacer", Spacer::new()))
                    .spacing(2)
                    .build(),
            )
            .build()
    }
}

impl DialogController for AboutDialog {

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

    fn handle_event(&mut self, event: &InputEvent, _ctx: &mut DialogContext) -> DialogResult {
        if !self.open {
            return DialogResult::Open;
        }
        let result = self.dialog.handle_event(event);
        if let EventResult::Action(action) = result {
            match action.as_str() {
                "ok" | "dialog_cancel" => return DialogResult::Closed,
                _ => {}
            }
        }
        DialogResult::Open
    }
}
