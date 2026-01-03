//! Centralized theme/styling system for UI widgets
//!
//! The Theme provides all colors used by widgets, ensuring consistent
//! styling across the application and enabling future theme switching.

use crate::terminal::Color;

/// Centralized theme for all widget colors
#[derive(Clone, Debug)]
#[allow(dead_code)]
pub struct Theme {
    // Dialog/window colors
    pub dialog_fg: Color,
    pub dialog_bg: Color,
    pub dialog_border_fg: Color,
    pub dialog_border_bg: Color,
    pub dialog_title_fg: Color,
    pub dialog_title_bg: Color,
    pub dialog_shadow: bool,

    // Button colors
    pub button_fg: Color,
    pub button_bg: Color,
    pub button_focused_fg: Color,
    pub button_focused_bg: Color,
    pub button_bracket_fg: Color,

    // TextField colors
    pub text_field_fg: Color,
    pub text_field_bg: Color,
    pub text_field_focused_fg: Color,
    pub text_field_focused_bg: Color,
    pub text_field_selection_fg: Color,
    pub text_field_selection_bg: Color,
    pub text_field_cursor_fg: Color,
    pub text_field_cursor_bg: Color,

    // Label colors
    pub label_fg: Color,
    pub label_bg: Color,
    pub label_highlight_fg: Color,
    pub label_highlight_bg: Color,

    // Separator colors (HRule, VRule)
    pub separator_fg: Color,
    pub separator_bg: Color,

    // ListView colors
    pub list_fg: Color,
    pub list_bg: Color,
    pub list_selected_fg: Color,
    pub list_selected_bg: Color,
    pub list_focused_selected_fg: Color,
    pub list_focused_selected_bg: Color,

    // Scrollbar colors
    pub scrollbar_track_fg: Color,
    pub scrollbar_track_bg: Color,
    pub scrollbar_thumb_fg: Color,
    pub scrollbar_thumb_bg: Color,

    // Checkbox/Radio colors
    pub checkbox_fg: Color,
    pub checkbox_bg: Color,
    pub checkbox_focused_fg: Color,
    pub checkbox_focused_bg: Color,
    pub checkbox_checked_char: char,
    pub checkbox_unchecked_char: char,

    // Status bar colors
    pub statusbar_fg: Color,
    pub statusbar_bg: Color,

    // Menu colors
    pub menu_fg: Color,
    pub menu_bg: Color,
    pub menu_highlight_fg: Color,
    pub menu_highlight_bg: Color,
    pub menu_hotkey_fg: Color,

    // Editor colors
    pub editor_fg: Color,
    pub editor_bg: Color,
    pub editor_border_fg: Color,
    pub editor_border_bg: Color,
    pub editor_title_fg: Color,
    pub editor_title_bg: Color,
    pub editor_line_number_fg: Color,
    pub editor_line_number_bg: Color,
    pub editor_current_line_bg: Color,
    pub editor_selection_fg: Color,
    pub editor_selection_bg: Color,

    // Immediate window colors
    pub immediate_fg: Color,
    pub immediate_bg: Color,
    pub immediate_border_fg: Color,
    pub immediate_border_bg: Color,
}

impl Theme {
    /// Classic QBasic blue theme
    pub fn classic_blue() -> Self {
        Self {
            // Dialog/window colors
            dialog_fg: Color::Black,
            dialog_bg: Color::LightGray,
            dialog_border_fg: Color::Black,
            dialog_border_bg: Color::LightGray,
            dialog_title_fg: Color::Black,
            dialog_title_bg: Color::LightGray,
            dialog_shadow: true,

            // Button colors
            button_fg: Color::Black,
            button_bg: Color::LightGray,
            button_focused_fg: Color::White,
            button_focused_bg: Color::Black,
            button_bracket_fg: Color::Black,

            // TextField colors
            text_field_fg: Color::Black,
            text_field_bg: Color::White,
            text_field_focused_fg: Color::Black,
            text_field_focused_bg: Color::White,
            text_field_selection_fg: Color::White,
            text_field_selection_bg: Color::Blue,
            text_field_cursor_fg: Color::Black,
            text_field_cursor_bg: Color::LightGray,

            // Label colors
            label_fg: Color::Black,
            label_bg: Color::LightGray,
            label_highlight_fg: Color::LightRed,
            label_highlight_bg: Color::LightGray,

            // Separator colors
            separator_fg: Color::Black,
            separator_bg: Color::LightGray,

            // ListView colors
            list_fg: Color::Black,
            list_bg: Color::White,
            list_selected_fg: Color::Black,
            list_selected_bg: Color::LightGray,
            list_focused_selected_fg: Color::White,
            list_focused_selected_bg: Color::Blue,

            // Scrollbar colors
            scrollbar_track_fg: Color::LightGray,
            scrollbar_track_bg: Color::DarkGray,
            scrollbar_thumb_fg: Color::White,
            scrollbar_thumb_bg: Color::LightGray,

            // Checkbox/Radio colors
            checkbox_fg: Color::Black,
            checkbox_bg: Color::LightGray,
            checkbox_focused_fg: Color::White,
            checkbox_focused_bg: Color::Black,
            checkbox_checked_char: 'X',
            checkbox_unchecked_char: ' ',

            // Status bar colors
            statusbar_fg: Color::White,
            statusbar_bg: Color::Cyan,

            // Menu colors
            menu_fg: Color::Black,
            menu_bg: Color::LightGray,
            menu_highlight_fg: Color::White,
            menu_highlight_bg: Color::Black,
            menu_hotkey_fg: Color::LightRed,

            // Editor colors
            editor_fg: Color::Yellow,
            editor_bg: Color::Blue,
            editor_border_fg: Color::LightGray,
            editor_border_bg: Color::Blue,
            editor_title_fg: Color::Blue,
            editor_title_bg: Color::LightGray,
            editor_line_number_fg: Color::LightGray,
            editor_line_number_bg: Color::Blue,
            editor_current_line_bg: Color::Blue,
            editor_selection_fg: Color::Blue,
            editor_selection_bg: Color::Yellow,

            // Immediate window colors
            immediate_fg: Color::Yellow,
            immediate_bg: Color::Blue,
            immediate_border_fg: Color::LightGray,
            immediate_border_bg: Color::Blue,
        }
    }

    /// Classic QBasic dialog theme (light gray with cyan focus)
    pub fn qbasic_dialog() -> Self {
        let mut theme = Self::classic_blue();

        theme.dialog_fg = Color::Black;
        theme.dialog_bg = Color::LightGray;
        theme.dialog_border_fg = Color::Black;
        theme.dialog_border_bg = Color::LightGray;
        theme.dialog_title_fg = Color::Black;
        theme.dialog_title_bg = Color::LightGray;
        theme.dialog_shadow = true;

        theme.button_fg = Color::Black;
        theme.button_bg = Color::LightGray;
        theme.button_focused_fg = Color::White;
        theme.button_focused_bg = Color::Black;
        theme.button_bracket_fg = Color::Black;

        theme.text_field_fg = Color::Black;
        theme.text_field_bg = Color::LightGray;
        theme.text_field_focused_fg = Color::Black;
        theme.text_field_focused_bg = Color::Cyan;
        theme.text_field_selection_fg = Color::White;
        theme.text_field_selection_bg = Color::Black;
        theme.text_field_cursor_fg = Color::White;
        theme.text_field_cursor_bg = Color::Black;

        theme.label_fg = Color::Black;
        theme.label_bg = Color::LightGray;
        theme.label_highlight_fg = Color::White;
        theme.label_highlight_bg = Color::Black;

        theme.separator_fg = Color::Black;
        theme.separator_bg = Color::LightGray;

        theme.list_fg = Color::Black;
        theme.list_bg = Color::LightGray;
        theme.list_selected_fg = Color::LightGray;
        theme.list_selected_bg = Color::Black;
        theme.list_focused_selected_fg = Color::LightGray;
        theme.list_focused_selected_bg = Color::Black;

        theme.scrollbar_track_fg = Color::LightGray;
        theme.scrollbar_track_bg = Color::Blue;
        theme.scrollbar_thumb_fg = Color::Black;
        theme.scrollbar_thumb_bg = Color::Blue;

        theme.checkbox_fg = Color::Black;
        theme.checkbox_bg = Color::LightGray;
        theme.checkbox_focused_fg = Color::White;
        theme.checkbox_focused_bg = Color::Black;
        theme.checkbox_checked_char = 'X';
        theme.checkbox_unchecked_char = ' ';

        theme
    }

}

impl Default for Theme {
    fn default() -> Self {
        Self::classic_blue()
    }
}
