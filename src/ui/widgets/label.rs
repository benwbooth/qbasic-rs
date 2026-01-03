//! Label widget - static text display

use crate::input::InputEvent;
use crate::screen::Screen;
use crate::ui::layout::{Rect, SizeHint};
use crate::ui::theme::Theme;
use crate::ui::widget::EventResult;
use crate::ui::widget_tree::{TreeWidget, EventPhase};

/// Text alignment for labels
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum LabelAlign {
    #[default]
    Left,
    Center,
}

/// A static text label widget
#[derive(Clone, Debug)]
pub struct Label {
    text: String,
    align: LabelAlign,
    /// If true, use highlight colors from theme
    highlight: bool,
    /// If true, use tight width based on text size
    tight_width: bool,
    min_width: Option<u16>,
}

impl Label {
    /// Create a new label with the given text
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            align: LabelAlign::Left,
            highlight: false,
            tight_width: false,
            min_width: None,
        }
    }

    /// Center the text
    pub fn centered(mut self) -> Self {
        self.align = LabelAlign::Center;
        self
    }

    /// Set highlight explicitly
    pub fn set_highlight(&mut self, highlight: bool) {
        self.highlight = highlight;
    }

    /// Set a minimum width (useful for aligning labels)
    pub fn min_width(mut self, width: u16) -> Self {
        self.min_width = Some(width);
        self.tight_width = true;
        self
    }

    /// Update minimum width at runtime
    pub fn set_min_width(&mut self, width: u16) {
        self.min_width = Some(width);
        self.tight_width = true;
    }

    /// Update the label text at runtime
    pub fn set_text(&mut self, text: impl Into<String>) {
        self.text = text.into();
    }
}

impl TreeWidget for Label {
    fn draw(&self, screen: &mut Screen, bounds: Rect, theme: &Theme) {
        let (fg, bg) = if self.highlight {
            (theme.label_highlight_fg, theme.label_highlight_bg)
        } else {
            (theme.label_fg, theme.label_bg)
        };

        // Calculate text position based on alignment
        let text_len = self.text.chars().count() as u16;
        let available = bounds.width;

        let x = match self.align {
            LabelAlign::Left => bounds.x,
            LabelAlign::Center => {
                bounds.x + available.saturating_sub(text_len) / 2
            }
        };

        // Truncate text if necessary
        let display_text: String = self.text
            .chars()
            .take(available as usize)
            .collect();

        screen.write_str(bounds.y, x, &display_text, fg, bg);
    }

    fn handle_event(&mut self, _event: &InputEvent, _bounds: Rect, _phase: EventPhase) -> EventResult {
        // Labels don't handle events
        EventResult::Ignored
    }

    fn size_hint(&self) -> SizeHint {
        let text_width = self.text.chars().count() as u16;
        let min_width = self.min_width.unwrap_or(text_width).max(text_width);
        SizeHint {
            min_width,
            min_height: 1,
            flex: 0,
        }
    }

    fn focusable(&self) -> bool {
        false
    }

    fn wants_tight_width(&self) -> bool {
        self.tight_width
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}
