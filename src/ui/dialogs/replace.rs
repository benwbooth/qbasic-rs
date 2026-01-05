//! Replace (Change) dialog.

use crate::input::InputEvent;
use crate::screen::Screen;
use crate::state::AppState;
use crate::ui::theme::Theme;
use crate::ui::widget::EventResult;
use crate::ui::widget_tree::WidgetNode;
use crate::ui::widgets::{Button, Checkbox, Label, Spacer, TextField};

use super::{DialogContext, DialogController, DialogResult, DialogWidget};

pub struct ReplaceDialog {
    dialog: DialogWidget,
    open: bool,
}

impl ReplaceDialog {
    pub fn new() -> Self {
        let content = Self::build_content();
        let mut dialog = DialogWidget::with_theme("Change", content, Theme::qbasic_dialog())
            .with_size(55, 12)
            .with_min_size(40, 10);
        dialog.set_show_maximize(false);
        Self { dialog, open: false }
    }

    fn build_content() -> WidgetNode {
        WidgetNode::vstack("root")
            .padding(1)
            .child(
                WidgetNode::hstack("find_row")
                    .leaf("find_label", Label::new("Find What:").min_width(12))
                    .leaf("find_field", TextField::new("find"))
                    .spacing(0)
                    .build(),
            )
            .child(WidgetNode::leaf("spacer1", Spacer::fixed(1)))
            .child(
                WidgetNode::hstack("replace_row")
                    .leaf("replace_label", Label::new("Change To:").min_width(12))
                    .leaf("replace_field", TextField::new("replace"))
                    .spacing(0)
                    .build(),
            )
            .child(WidgetNode::leaf("spacer2", Spacer::fixed(1)))
            .child(
                WidgetNode::hstack("options_row")
                    .leaf("case_checkbox", Checkbox::new("Match Case", "toggle_case").min_width(20))
                    .leaf("whole_checkbox", Checkbox::new("Whole Word", "toggle_whole").min_width(18))
                    .spacing(0)
                    .build(),
            )
            .child(WidgetNode::leaf("spacer3", Spacer::fixed(1)))
            .child(
                WidgetNode::hstack("buttons_row")
                    .child(WidgetNode::leaf("btn_spacer_left", Spacer::new()))
                    .leaf("find_next_button", Button::new("Find Next", "find_next").min_width(12))
                    .leaf("replace_button", Button::new("Replace", "replace").min_width(10))
                    .leaf("replace_all_button", Button::new("Replace All", "replace_all").min_width(14))
                    .leaf("cancel_button", Button::new("Cancel", "cancel").min_width(10))
                    .child(WidgetNode::leaf("btn_spacer_right", Spacer::new()))
                    .spacing(1)
                    .build(),
            )
            .build()
    }

    fn sync_focus_decor(&mut self) {
        let focus_path = self.dialog.content().focus_path();
        let find_focused = focus_path.iter().any(|id| id == "find_field");
        let replace_focused = focus_path.iter().any(|id| id == "replace_field");

        if let Some(label) = self.dialog.content_mut()
            .get_widget_mut(&["root", "find_row", "find_label"])
            .and_then(|w| w.as_any_mut().downcast_mut::<Label>())
        {
            label.set_highlight(find_focused);
        }
        if let Some(label) = self.dialog.content_mut()
            .get_widget_mut(&["root", "replace_row", "replace_label"])
            .and_then(|w| w.as_any_mut().downcast_mut::<Label>())
        {
            label.set_highlight(replace_focused);
        }
    }

    fn sync_from_state(&mut self, state: &AppState) {
        if let Some(tf) = self.dialog.content_mut()
            .get_widget_mut(&["root", "find_row", "find_field"])
            .and_then(|w| w.as_any_mut().downcast_mut::<TextField>())
        {
            if tf.text().is_empty() && !state.last_search.is_empty() {
                tf.set_text(&state.last_search);
                tf.set_cursor_pos(state.last_search.chars().count());
            }
        }
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

    fn read_state(&self) -> (String, String, bool, bool) {
        let find_text = self.dialog.content()
            .get_widget(&["root", "find_row", "find_field"])
            .and_then(|w| w.as_any().downcast_ref::<TextField>())
            .map(|tf| tf.text().to_string())
            .unwrap_or_default();
        let replace_text = self.dialog.content()
            .get_widget(&["root", "replace_row", "replace_field"])
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
        (find_text, replace_text, case_sensitive, whole_word)
    }
}

impl ReplaceDialog {
    fn find_and_verify(&self, ctx: &mut DialogContext) {
        let (search, _, case_sensitive, whole_word) = self.read_state();
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

    fn replace_selection(&self, ctx: &mut DialogContext) {
        let (_, replacement, _, _) = self.read_state();
        if ctx.editor.replace_selection(&replacement) {
            ctx.state.set_modified(true);
        }
    }

    fn replace_all(&self, ctx: &mut DialogContext) {
        let (search, replacement, case_sensitive, whole_word) = self.read_state();
        ctx.state.last_search = search.clone();
        ctx.state.search_case_sensitive = case_sensitive;
        ctx.state.search_whole_word = whole_word;

        let count = ctx.editor.replace_all(&search, &replacement, case_sensitive, whole_word);
        if count > 0 {
            ctx.state.set_modified(true);
            ctx.state.set_status(format!("Replaced {} occurrences", count));
        } else {
            ctx.state.set_status("No matches found");
        }
    }
}

impl DialogController for ReplaceDialog {

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
        let (_, _, case_sensitive, whole_word) = self.read_state();

        // Keep search options in sync
        ctx.state.search_case_sensitive = case_sensitive;
        ctx.state.search_whole_word = whole_word;

        if let EventResult::Action(a) = result {
            match a.as_str() {
                "find_next" | "find_submit" => {
                    self.find_and_verify(ctx);
                    self.sync_focus_decor();
                    // Stay open - don't close after find
                    return DialogResult::Open;
                }
                "replace" | "replace_submit" => {
                    self.replace_selection(ctx);
                    self.sync_focus_decor();
                    // Stay open - don't close after single replace
                    return DialogResult::Open;
                }
                "replace_all" => {
                    self.replace_all(ctx);
                    self.sync_focus_decor();
                    return DialogResult::Closed;
                }
                "cancel" | "dialog_cancel" => {
                    return DialogResult::Closed;
                }
                _ => {}
            }
        }

        self.sync_focus_decor();
        DialogResult::Open
    }

}
