//! Display options dialog.

use crate::input::InputEvent;
use crate::screen::Screen;
use crate::state::AppState;
use crate::ui::layout::{Rect, SizeHint};
use crate::ui::theme::Theme;
use crate::ui::widget::EventResult;
use crate::ui::widget_tree::{EventPhase, TreeWidget, WidgetNode};
use crate::ui::widgets::{Button, Checkbox, Label, RadioButton, Spacer};

use super::{DialogContext, DialogController, DialogResult, DialogWidget};

pub struct DisplayOptionsDialog {
    dialog: DialogWidget,
    open: bool,
}

impl DisplayOptionsDialog {
    pub fn new() -> Self {
        let content = Self::build_content();
        let mut dialog = DialogWidget::with_theme("Display", content, Theme::qbasic_dialog())
            .with_size(50, 14)
            .with_min_size(40, 10);
        dialog.set_show_maximize(false);
        dialog.set_chrome_interactive(false);
        Self { dialog, open: false }
    }

    fn build_content() -> WidgetNode {
        WidgetNode::vstack("root")
            .padding(1)
            .child(
                WidgetNode::hstack("tabs_row")
                    .leaf("tabs_label", Label::new("Tab Stops:").min_width(12))
                    .leaf("tabs_field", TabStopsField::new())
                    .spacing(0)
                    .build(),
            )
            .child(WidgetNode::leaf("spacer1", Spacer::fixed(1)))
            .child(WidgetNode::leaf("scrollbars_checkbox", Checkbox::new("Scroll Bars", "toggle_scrollbars")))
            .child(WidgetNode::leaf("spacer2", Spacer::fixed(1)))
            .child(WidgetNode::leaf("scheme_label", Label::new("Color Scheme:")))
            .child(
                WidgetNode::hstack("scheme_blue_row")
                    .child(WidgetNode::leaf("scheme_blue_pad", Spacer::fixed(2)))
                    .leaf("scheme_blue", RadioButton::new("Classic Blue", "scheme_blue"))
                    .spacing(0)
                    .build(),
            )
            .child(
                WidgetNode::hstack("scheme_dark_row")
                    .child(WidgetNode::leaf("scheme_dark_pad", Spacer::fixed(2)))
                    .leaf("scheme_dark", RadioButton::new("Dark", "scheme_dark"))
                    .spacing(0)
                    .build(),
            )
            .child(
                WidgetNode::hstack("scheme_light_row")
                    .child(WidgetNode::leaf("scheme_light_pad", Spacer::fixed(2)))
                    .leaf("scheme_light", RadioButton::new("Light", "scheme_light"))
                    .spacing(0)
                    .build(),
            )
            .child(WidgetNode::leaf("spacer_flex", Spacer::new()))
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

    fn sync_from_state(&mut self, state: &AppState) {
        if let Some(field) = self.get_tab_field_mut() {
            field.set_text(state.tab_stops.to_string());
            field.set_cursor_pos(field.text().chars().count());
        }
        if let Some(cb) = self.dialog.content_mut()
            .get_widget_mut(&["root", "scrollbars_checkbox"])
            .and_then(|w| w.as_any_mut().downcast_mut::<Checkbox>())
        {
            cb.set_checked(state.show_scrollbars);
        }
        self.sync_scheme_radios(state.color_scheme);
    }

    fn sync_scheme_radios(&mut self, scheme: usize) {
        if let Some(rb) = self.dialog.content_mut()
            .get_widget_mut(&["root", "scheme_blue_row", "scheme_blue"])
            .and_then(|w| w.as_any_mut().downcast_mut::<RadioButton>())
        {
            rb.set_selected(scheme == 0);
        }
        if let Some(rb) = self.dialog.content_mut()
            .get_widget_mut(&["root", "scheme_dark_row", "scheme_dark"])
            .and_then(|w| w.as_any_mut().downcast_mut::<RadioButton>())
        {
            rb.set_selected(scheme == 1);
        }
        if let Some(rb) = self.dialog.content_mut()
            .get_widget_mut(&["root", "scheme_light_row", "scheme_light"])
            .and_then(|w| w.as_any_mut().downcast_mut::<RadioButton>())
        {
            rb.set_selected(scheme == 2);
        }
    }

    fn get_tab_field_mut(&mut self) -> Option<&mut TabStopsField> {
        self.dialog.content_mut()
            .get_widget_mut(&["root", "tabs_row", "tabs_field"])?
            .as_any_mut()
            .downcast_mut::<TabStopsField>()
    }

    fn get_tab_field(&self) -> Option<&TabStopsField> {
        self.dialog.content()
            .get_widget(&["root", "tabs_row", "tabs_field"])?
            .as_any()
            .downcast_ref::<TabStopsField>()
    }

    fn sync_focus_decor(&mut self) {
        let focus_path = self.dialog.content().focus_path();
        let tab_focused = focus_path.iter().any(|id| id == "tabs_field");
        if let Some(label) = self.dialog.content_mut()
            .get_widget_mut(&["root", "tabs_row", "tabs_label"])
            .and_then(|w| w.as_any_mut().downcast_mut::<Label>())
        {
            label.set_highlight(tab_focused);
        }
    }

    fn read_settings(&self) -> (usize, bool) {
        let tab_stops = self.get_tab_field()
            .and_then(|f| f.text().parse::<usize>().ok())
            .unwrap_or(4);
        let show_scrollbars = self.dialog.content()
            .get_widget(&["root", "scrollbars_checkbox"])
            .and_then(|w| w.as_any().downcast_ref::<Checkbox>())
            .map(|cb| cb.checked())
            .unwrap_or(true);
        (tab_stops, show_scrollbars)
    }
}

impl DialogController for DisplayOptionsDialog {
    fn open(&mut self, ctx: &mut DialogContext) {
        self.open = true;
        self.sync_from_state(ctx.state);
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

        if let EventResult::Action(action) = result {
            match action.as_str() {
                "scheme_blue" => {
                    self.sync_scheme_radios(0);
                }
                "scheme_dark" => {
                    self.sync_scheme_radios(1);
                }
                "scheme_light" => {
                    self.sync_scheme_radios(2);
                }
                "ok" => {
                    // Save settings to state
                    let (tab_stops, show_scrollbars) = self.read_settings();
                    ctx.state.tab_stops = tab_stops;
                    ctx.state.show_scrollbars = show_scrollbars;
                    // Determine which scheme is selected
                    if let Some(rb) = self.dialog.content()
                        .get_widget(&["root", "scheme_dark_row", "scheme_dark"])
                        .and_then(|w| w.as_any().downcast_ref::<RadioButton>())
                    {
                        if rb.selected() { ctx.state.color_scheme = 1; }
                    }
                    if let Some(rb) = self.dialog.content()
                        .get_widget(&["root", "scheme_light_row", "scheme_light"])
                        .and_then(|w| w.as_any().downcast_ref::<RadioButton>())
                    {
                        if rb.selected() { ctx.state.color_scheme = 2; }
                    }
                    if let Some(rb) = self.dialog.content()
                        .get_widget(&["root", "scheme_blue_row", "scheme_blue"])
                        .and_then(|w| w.as_any().downcast_ref::<RadioButton>())
                    {
                        if rb.selected() { ctx.state.color_scheme = 0; }
                    }
                    ctx.state.set_status("Display options saved".to_string());
                    return DialogResult::Closed;
                }
                "cancel" | "dialog_cancel" => return DialogResult::Closed,
                _ => {}
            }
        }

        DialogResult::Open
    }
}

// Custom numeric-only text field for tab stops
#[derive(Clone, Debug)]
struct TabStopsField {
    text: String,
    cursor: usize,
    focused: bool,
}

impl TabStopsField {
    fn new() -> Self {
        Self { text: String::new(), cursor: 0, focused: false }
    }

    fn text(&self) -> &str { &self.text }

    fn set_text(&mut self, text: impl Into<String>) {
        self.text = text.into();
        self.cursor = self.cursor.min(self.text.len());
    }

    fn set_cursor_pos(&mut self, pos: usize) {
        self.cursor = pos.min(self.text.len());
    }

    fn try_set_text(&mut self, text: String, cursor: usize) -> bool {
        if let Ok(value) = text.parse::<usize>() {
            if value >= 1 && value <= 16 {
                self.text = text;
                self.cursor = cursor.min(self.text.len());
                return true;
            }
        }
        false
    }
}

impl TreeWidget for TabStopsField {
    fn draw(&self, screen: &mut Screen, bounds: Rect, theme: &Theme) {
        if bounds.width == 0 || bounds.height == 0 { return; }

        let text_fg = if self.focused { theme.text_field_focused_fg } else { theme.text_field_fg };
        let text_bg = if self.focused { theme.text_field_focused_bg } else { theme.text_field_bg };

        let visible_width = bounds.width as usize;
        for i in 0..visible_width {
            let ch = self.text.chars().nth(i).unwrap_or(' ');
            let (fg, bg) = if self.focused && i == self.cursor {
                (theme.text_field_cursor_fg, theme.text_field_cursor_bg)
            } else {
                (text_fg, text_bg)
            };
            screen.set(bounds.y, bounds.x + i as u16, ch, fg, bg);
        }
    }

    fn handle_event(&mut self, event: &InputEvent, bounds: Rect, phase: EventPhase) -> EventResult {
        if phase != EventPhase::Target { return EventResult::Ignored; }

        match event {
            InputEvent::Char(c) if c.is_ascii_digit() => {
                let mut new_text = self.text.clone();
                if self.cursor <= new_text.len() {
                    new_text.insert(self.cursor, *c);
                    if self.try_set_text(new_text, self.cursor + 1) {
                        return EventResult::Consumed;
                    }
                }
            }
            InputEvent::Backspace if self.cursor > 0 => {
                let mut new_text = self.text.clone();
                new_text.remove(self.cursor - 1);
                if self.try_set_text(new_text, self.cursor - 1) {
                    return EventResult::Consumed;
                }
            }
            InputEvent::Delete if self.cursor < self.text.len() => {
                let mut new_text = self.text.clone();
                new_text.remove(self.cursor);
                if self.try_set_text(new_text, self.cursor) {
                    return EventResult::Consumed;
                }
            }
            InputEvent::CursorLeft if self.cursor > 0 => {
                self.cursor -= 1;
                return EventResult::Consumed;
            }
            InputEvent::CursorRight if self.cursor < self.text.len() => {
                self.cursor += 1;
                return EventResult::Consumed;
            }
            InputEvent::Home => {
                self.cursor = 0;
                return EventResult::Consumed;
            }
            InputEvent::End => {
                self.cursor = self.text.len();
                return EventResult::Consumed;
            }
            InputEvent::Enter => return EventResult::Action("ok".to_string()),
            InputEvent::MouseClick { row, col } if bounds.contains(*row, *col) => {
                self.cursor = (col.saturating_sub(bounds.x) as usize).min(self.text.len());
                return EventResult::Consumed;
            }
            _ => {}
        }
        EventResult::Ignored
    }

    fn size_hint(&self) -> SizeHint {
        SizeHint { min_width: 6, min_height: 1, flex: 0 }
    }

    fn focusable(&self) -> bool { true }
    fn set_focus(&mut self, focused: bool) { self.focused = focused; }
    fn wants_tight_width(&self) -> bool { true }
    fn as_any(&self) -> &dyn std::any::Any { self }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any { self }
}
