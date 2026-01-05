//! Welcome dialog.

use crate::input::InputEvent;
use crate::screen::Screen;
use crate::state::AppState;
use crate::ui::layout::Rect;
use crate::ui::theme::Theme;
use crate::ui::widget_tree::{WidgetNode, WidgetTree};
use crate::ui::widgets::{Button, HRule, Label, Spacer};

use super::{DialogContext, DialogController, DialogResult};
use crate::ui::widget::EventResult;

/// Welcome dialog shown on startup
pub struct WelcomeDialog {
    tree: WidgetTree,
    x: u16,
    y: u16,
    width: u16,
    height: u16,
    open: bool,
    screen_size: (u16, u16),
    /// User clicked "Start" to see help
    pub show_help_on_close: bool,
}

impl WelcomeDialog {
    pub fn new() -> Self {
        let content = Self::build_content();
        let tree = WidgetTree::with_theme(content, Theme::classic_blue());

        Self {
            tree,
            x: 0,
            y: 0,
            width: 60,
            height: 10,
            open: false,
            screen_size: (80, 25),
            show_help_on_close: false,
        }
    }

    fn build_content() -> WidgetNode {
        WidgetNode::vstack("root")
            .leaf("welcome", Label::new("Welcome to BASIC-RS").centered())
            .child(WidgetNode::leaf("spacer1", Spacer::fixed(1)))
            .leaf("subtitle", Label::new("A BASIC interpreter written in Rust").centered())
            .child(WidgetNode::leaf("flex_spacer", Spacer::new()))
            .child(WidgetNode::leaf("spacer_above_hr", Spacer::fixed(1)))
            .child(WidgetNode::leaf("hr", HRule::t_connector()))
            .child(WidgetNode::hstack("start_row")
                .child(WidgetNode::leaf("sl1", Spacer::new()))
                .leaf("start_btn", Button::new("Press Enter to see the Survival Guide", "start"))
                .child(WidgetNode::leaf("sr1", Spacer::new()))
                .build())
            .child(WidgetNode::hstack("exit_row")
                .child(WidgetNode::leaf("sl2", Spacer::new()))
                .leaf("exit_btn", Button::new("Press ESC to clear this dialog box", "cancel"))
                .child(WidgetNode::leaf("sr2", Spacer::new()))
                .build())
            .build()
    }

    fn center(&mut self) {
        self.x = (self.screen_size.0.saturating_sub(self.width)) / 2;
        self.y = (self.screen_size.1.saturating_sub(self.height)) / 2;
    }

    fn content_rect(&self) -> Rect {
        Rect::new(
            self.x,
            self.y + 1,
            self.width,
            self.height.saturating_sub(2),
        )
    }
}

impl DialogController for WelcomeDialog {

    fn open(&mut self, ctx: &mut DialogContext) {
        self.open = true;
        self.tree.focus_next();
        self.center();
        ctx.state.focus_dialog();
    }

    fn is_open(&self) -> bool {
        self.open
    }

    fn close(&mut self) {
        self.open = false;
    }

    fn set_screen_size(&mut self, width: u16, height: u16) {
        self.screen_size = (width, height);
        self.center();
    }

    fn draw(&mut self, screen: &mut Screen, _state: &AppState) {
        if !self.open {
            return;
        }

        let theme = self.tree.theme();

        // Draw shadow
        screen.draw_shadow(self.y, self.x, self.width, self.height);

        // Draw background
        screen.fill(
            self.y,
            self.x,
            self.width,
            self.height,
            ' ',
            theme.dialog_fg,
            theme.dialog_bg,
        );

        // Draw border
        screen.draw_box(
            self.y,
            self.x,
            self.width,
            self.height,
            theme.dialog_border_fg,
            theme.dialog_border_bg,
        );

        // Draw title
        let title = " Welcome ";
        let title_x = self.x + (self.width.saturating_sub(title.len() as u16)) / 2;
        screen.write_str(self.y, title_x, title, theme.dialog_title_fg, theme.dialog_title_bg);

        // Draw content
        self.tree.draw(screen, self.content_rect());
    }

    fn handle_event(&mut self, event: &InputEvent, _ctx: &mut DialogContext) -> DialogResult {
        if !self.open {
            return DialogResult::Open;
        }

        // Handle Escape to close
        if matches!(event, InputEvent::Escape) {
            self.show_help_on_close = false;
            return DialogResult::Closed;
        }

        // Route to widget tree and check for button actions
        let result = self.tree.handle_event(event, self.content_rect());
        if let EventResult::Action(action) = result {
            match action.as_str() {
                "start" => {
                    self.show_help_on_close = true;
                    return DialogResult::Closed;
                }
                "cancel" => {
                    self.show_help_on_close = false;
                    return DialogResult::Closed;
                }
                _ => {}
            }
        }

        DialogResult::Open
    }
}
