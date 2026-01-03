//! Print dialog.

use crate::input::InputEvent;
use crate::screen::Screen;
use crate::state::AppState;
use crate::ui::theme::Theme;
use crate::ui::widget::EventResult;
use crate::ui::widget_tree::WidgetNode;
use crate::ui::widgets::{Button, RadioButton, Spacer};

use super::{DialogContext, DialogController, DialogResult, DialogWidget};

pub struct PrintDialog {
    dialog: DialogWidget,
    selected: usize,
    open: bool,
}

impl PrintDialog {
    pub fn new() -> Self {
        let content = Self::build_content();
        let mut dialog = DialogWidget::with_theme("Print", content, Theme::qbasic_dialog())
            .with_size(50, 10)
            .with_min_size(30, 8);
        dialog.set_show_maximize(false);
        dialog.set_chrome_interactive(false);
        Self { dialog, selected: 0, open: false }
    }

    fn build_content() -> WidgetNode {
        WidgetNode::vstack("root")
            .padding(1)
            .leaf("option_selected", RadioButton::new("Selected Text Only", "option_selected"))
            .child(WidgetNode::leaf("spacer1", Spacer::fixed(1)))
            .leaf("option_range", RadioButton::new("Lines:", "option_range"))
            .child(WidgetNode::leaf("spacer2", Spacer::new()))
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

    fn sync_radio_state(&mut self) {
        if let Some(rb) = self.dialog.content_mut()
            .get_widget_mut(&["root", "option_selected"])
            .and_then(|w| w.as_any_mut().downcast_mut::<RadioButton>())
        {
            rb.set_selected(self.selected == 0);
        }
        if let Some(rb) = self.dialog.content_mut()
            .get_widget_mut(&["root", "option_range"])
            .and_then(|w| w.as_any_mut().downcast_mut::<RadioButton>())
        {
            rb.set_selected(self.selected == 1);
        }
    }
}

impl DialogController for PrintDialog {

    fn open(&mut self, ctx: &mut DialogContext) {
        self.open = true;
        self.selected = 0;
        self.sync_radio_state();
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
                "option_selected" => {
                    self.selected = 0;
                    self.sync_radio_state();
                }
                "option_range" => {
                    self.selected = 1;
                    self.sync_radio_state();
                }
                "ok" => {
                    ctx.state.set_status("Printing is not supported".to_string());
                    return DialogResult::Closed;
                }
                "cancel" | "dialog_cancel" => return DialogResult::Closed,
                _ => {}
            }
        }
        DialogResult::Open
    }

}
