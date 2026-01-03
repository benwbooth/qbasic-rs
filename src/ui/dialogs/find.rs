//! Find dialog.

use crate::input::InputEvent;
use crate::screen::Screen;
use crate::state::AppState;
use crate::ui::theme::Theme;
use crate::ui::widget::EventResult;
use crate::ui::widget_tree::WidgetNode;
use crate::ui::widgets::{Button, Checkbox, Label, Spacer, TextField};

use super::{DialogContext, DialogController, DialogResult, DialogWidget};

pub struct FindDialog {
    dialog: DialogWidget,
    open: bool,
}

impl FindDialog {
    pub fn new() -> Self {
        let content = Self::build_content();
        let mut dialog = DialogWidget::with_theme("Find", content, Theme::qbasic_dialog())
            .with_size(55, 10)
            .with_min_size(40, 8);
        dialog.set_show_maximize(false);
        dialog.set_chrome_interactive(false);
        Self { dialog, open: false }
    }

    fn build_content() -> WidgetNode {
        WidgetNode::vstack("root")
            .padding(1)
            .child(
                WidgetNode::hstack("find_row")
                    .leaf("find_label", Label::new("Find:").min_width(8))
                    .leaf("find_field", TextField::new("find"))
                    .spacing(0)
                    .build(),
            )
            .child(WidgetNode::leaf("spacer1", Spacer::fixed(1)))
            .child(
                WidgetNode::hstack("options_row")
                    .leaf("case_checkbox", Checkbox::new("Match Case", "toggle_case").min_width(20))
                    .leaf("whole_checkbox", Checkbox::new("Whole Word", "toggle_whole").min_width(18))
                    .spacing(0)
                    .build(),
            )
            .child(WidgetNode::leaf("spacer2", Spacer::fixed(1)))
            .child(
                WidgetNode::hstack("buttons_row")
                    .child(WidgetNode::leaf("btn_spacer_left", Spacer::new()))
                    .leaf("ok_button", Button::new("Find", "find").min_width(8))
                    .leaf("cancel_button", Button::new("Cancel", "cancel").min_width(10))
                    .child(WidgetNode::leaf("btn_spacer_right", Spacer::new()))
                    .spacing(2)
                    .build(),
            )
            .build()
    }

    fn sync_focus_decor(&mut self) {
        let focus_path = self.dialog.content().focus_path();
        let find_focused = focus_path.iter().any(|id| id == "find_field");
        if let Some(label) = self.dialog.content_mut()
            .get_widget_mut(&["root", "find_row", "find_label"])
            .and_then(|w| w.as_any_mut().downcast_mut::<Label>())
        {
            label.set_highlight(find_focused);
        }
    }

    fn sync_from_state(&mut self, state: &AppState) {
        // Populate search text from last search if empty
        if let Some(tf) = self.dialog.content_mut()
            .get_widget_mut(&["root", "find_row", "find_field"])
            .and_then(|w| w.as_any_mut().downcast_mut::<TextField>())
        {
            if tf.text().is_empty() && !state.last_search.is_empty() {
                tf.set_text(&state.last_search);
                tf.set_cursor_pos(state.last_search.chars().count());
            }
        }
        // Sync checkboxes
        if let Some(cb) = self.dialog.content_mut()
            .get_widget_mut(&["root", "options_row", "case_checkbox"])
            .and_then(|w| w.as_any_mut().downcast_mut::<Checkbox>())
        {
            cb.set_checked(state.search_case_sensitive);
        }
        if let Some(cb) = self.dialog.content_mut()
            .get_widget_mut(&["root", "options_row", "whole_checkbox"])
            .and_then(|w| w.as_any_mut().downcast_mut::<Checkbox>())
        {
            cb.set_checked(state.search_whole_word);
        }
    }

    fn read_search_state(&self) -> (String, bool, bool) {
        let query = self.dialog.content()
            .get_widget(&["root", "find_row", "find_field"])
            .and_then(|w| w.as_any().downcast_ref::<TextField>())
            .map(|tf| tf.text().to_string())
            .unwrap_or_default();
        let case_sensitive = self.dialog.content()
            .get_widget(&["root", "options_row", "case_checkbox"])
            .and_then(|w| w.as_any().downcast_ref::<Checkbox>())
            .map(|cb| cb.checked())
            .unwrap_or(false);
        let whole_word = self.dialog.content()
            .get_widget(&["root", "options_row", "whole_checkbox"])
            .and_then(|w| w.as_any().downcast_ref::<Checkbox>())
            .map(|cb| cb.checked())
            .unwrap_or(false);
        (query, case_sensitive, whole_word)
    }
}

impl FindDialog {
    fn find_next(&self, ctx: &mut DialogContext) {
        let (search, case_sensitive, whole_word) = self.read_search_state();
        if search.is_empty() {
            ctx.state.set_status("No search text");
            return;
        }

        ctx.state.last_search = search.clone();
        ctx.state.search_case_sensitive = case_sensitive;
        ctx.state.search_whole_word = whole_word;

        if let Some((line, col)) = ctx.editor.find_text(&search, case_sensitive, whole_word) {
            ctx.editor.go_to_and_select(line, col, search.len());
            ctx.state.set_status(format!("Found at line {}", line + 1));
        } else {
            ctx.state.set_status("Match not found");
        }
    }
}

impl DialogController for FindDialog {

    fn open(&mut self, ctx: &mut DialogContext) {
        self.open = true;
        self.sync_from_state(ctx.state);
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
        let (_, case_sensitive, whole_word) = self.read_search_state();

        // Keep search options in sync
        ctx.state.search_case_sensitive = case_sensitive;
        ctx.state.search_whole_word = whole_word;

        if let EventResult::Action(a) = result {
            match a.as_str() {
                "find" | "find_submit" | "find_next" => {
                    self.find_next(ctx);
                    self.sync_focus_decor();
                    return DialogResult::Closed;
                }
                "cancel" | "dialog_cancel" => return DialogResult::Closed,
                _ => {}
            }
        }

        self.sync_focus_decor();
        DialogResult::Open
    }

}
