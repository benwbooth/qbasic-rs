//! QBasic-style dialog boxes

use crate::screen::Screen;
use crate::terminal::Color;
use crate::state::{AppState, DialogType};
use super::layout::{Rect, LayoutItem, Size, compute_layout, file_dialog_layout};

/// Dialog component
pub struct Dialog;

impl Dialog {
    /// Draw a dialog based on its type
    pub fn draw(screen: &mut Screen, state: &mut AppState, width: u16, height: u16) {
        let selected = state.dialog_button;
        let dialog_type = state.dialog.clone();
        match &dialog_type {
            DialogType::None => {}
            DialogType::About => Self::draw_about(screen, state, width, height, selected),
            DialogType::Message { title, text } => Self::draw_message(screen, state, width, height, title, text, selected),
            DialogType::Confirm { title, text } => Self::draw_confirm(screen, state, width, height, title, text, selected),
            DialogType::FileOpen => Self::draw_file_dialog(screen, state, width, height, "Open", selected),
            DialogType::FileSave | DialogType::FileSaveAs => Self::draw_file_dialog(screen, state, width, height, "Save As", selected),
            DialogType::Find => Self::draw_find_dialog(screen, state, width, height, selected),
            DialogType::Replace => Self::draw_replace_dialog(screen, state, width, height, selected),
            DialogType::GoToLine => Self::draw_goto_dialog(screen, state, width, height, selected),
            DialogType::Help(topic) => Self::draw_help(screen, state, width, height, topic, selected),
            DialogType::NewProgram => Self::draw_new_program(screen, state, width, height, selected),
            DialogType::Print => Self::draw_print_dialog(screen, state, width, height, selected),
            DialogType::Welcome => Self::draw_welcome(screen, state, width, height, selected),
            DialogType::NewSub => Self::draw_new_sub(screen, state, width, height, selected),
            DialogType::NewFunction => Self::draw_new_function(screen, state, width, height, selected),
            DialogType::FindLabel => Self::draw_find_label(screen, state, width, height, selected),
            DialogType::CommandArgs => Self::draw_command_args(screen, state, width, height, selected),
            DialogType::HelpPath => Self::draw_help_path(screen, state, width, height, selected),
            DialogType::DisplayOptions => Self::draw_display_options(screen, state, width, height, selected),
        }
    }

    fn draw_dialog_box_at(screen: &mut Screen, x: u16, y: u16, width: u16, height: u16, title: &str) {
        // Draw shadow first
        screen.draw_shadow(y, x, width, height);

        // Draw single-line box
        screen.draw_box(y, x, width, height, Color::Black, Color::LightGray);

        // Title bar - inverted colors
        let title_str = format!(" {} ", title);
        let title_x = x + (width.saturating_sub(title_str.len() as u16)) / 2;
        screen.write_str(y, title_x, &title_str, Color::LightGray, Color::Black);

        // Window controls at top-right: ↑ (maximize), X (close)
        if width >= 10 {
            screen.write_str(y, x + width - 7, "[↑]", Color::Black, Color::LightGray);
            screen.write_str(y, x + width - 3, "[X]", Color::Black, Color::LightGray);
        }
    }

    /// Legacy helper for dialogs that don't use state positioning yet
    fn draw_dialog_box(screen: &mut Screen, screen_width: u16, screen_height: u16, title: &str, dialog_width: u16, dialog_height: u16) -> (u16, u16) {
        let x = (screen_width - dialog_width) / 2;
        let y = (screen_height - dialog_height) / 2;
        Self::draw_dialog_box_at(screen, x, y, dialog_width, dialog_height, title);
        (x, y)
    }

    fn draw_button(screen: &mut Screen, row: u16, col: u16, label: &str, selected: bool) {
        let fg = if selected { Color::White } else { Color::Black };
        let bg = if selected { Color::Black } else { Color::LightGray };

        let btn = format!("< {} >", label);
        screen.write_str(row, col, &btn, fg, bg);
    }

    fn draw_about(screen: &mut Screen, state: &mut AppState, _screen_width: u16, _screen_height: u16, selected: usize) {
        // Use state-tracked dialog position
        let x = state.dialog_x;
        let y = state.dialog_y;
        let width = state.dialog_width;
        let height = state.dialog_height;

        Self::draw_dialog_box_at(screen, x, y, width, height, "About");

        let lines = [
            "",
            "      QBasic IDE Simulator",
            "",
            "      Written in Rust",
            "      Using raw ANSI escape sequences",
            "",
            "      A tribute to Microsoft QBasic",
            "      (1991-2000)",
            "",
        ];

        for (i, line) in lines.iter().enumerate() {
            screen.write_str(y + 1 + i as u16, x + 1, line, Color::Black, Color::LightGray);
        }

        // Compute layout for button positioning
        let btn_width = 6u16;
        let btn_x = x + (width.saturating_sub(btn_width)) / 2;
        let btn_y = y + height - 2;

        Self::draw_button(screen, btn_y, btn_x, "OK", selected == 0);

        // Cache layout for hit testing
        let layout_item = LayoutItem::vstack(vec![
            LayoutItem::leaf("title_bar").fixed_height(1),
            LayoutItem::spacer().height(Size::Flex(1)),
            LayoutItem::hstack(vec![
                LayoutItem::spacer(),
                LayoutItem::leaf("ok_button").fixed_width(btn_width + 4), // "< OK >"
                LayoutItem::spacer(),
            ]).fixed_height(1),
            LayoutItem::spacer().fixed_height(1),
        ]);
        let bounds = Rect::new(x, y, width, height);
        state.dialog_layout = Some(compute_layout(&layout_item, bounds));
    }

    fn draw_message(screen: &mut Screen, state: &mut AppState, _screen_width: u16, _screen_height: u16, title: &str, text: &str, selected: usize) {
        let x = state.dialog_x;
        let y = state.dialog_y;
        let width = state.dialog_width;
        let height = state.dialog_height;

        Self::draw_dialog_box_at(screen, x, y, width, height, title);

        let lines: Vec<&str> = text.lines().collect();
        for (i, line) in lines.iter().take((height - 4) as usize).enumerate() {
            screen.write_str(y + 2 + i as u16, x + 2, line, Color::Black, Color::LightGray);
        }

        let btn_y = y + height - 2;
        let btn_x = x + (width - 6) / 2;
        Self::draw_button(screen, btn_y, btn_x, "OK", selected == 0);

        // Cache layout for hit testing
        let layout_item = LayoutItem::vstack(vec![
            LayoutItem::leaf("title_bar").fixed_height(1),
            LayoutItem::spacer().height(Size::Flex(1)),
            LayoutItem::hstack(vec![
                LayoutItem::spacer(),
                LayoutItem::leaf("ok_button").fixed_width(10),
                LayoutItem::spacer(),
            ]).fixed_height(1),
            LayoutItem::spacer().fixed_height(1),
        ]);
        let bounds = Rect::new(x, y, width, height);
        state.dialog_layout = Some(compute_layout(&layout_item, bounds));
    }

    fn draw_confirm(screen: &mut Screen, state: &mut AppState, _screen_width: u16, _screen_height: u16, title: &str, text: &str, selected: usize) {
        let x = state.dialog_x;
        let y = state.dialog_y;
        let width = state.dialog_width;
        let height = state.dialog_height;

        Self::draw_dialog_box_at(screen, x, y, width, height, title);

        let lines: Vec<&str> = text.lines().collect();
        for (i, line) in lines.iter().take((height - 4) as usize).enumerate() {
            screen.write_str(y + 2 + i as u16, x + 2, line, Color::Black, Color::LightGray);
        }

        let btn_y = y + height - 2;
        Self::draw_button(screen, btn_y, x + 5, "Yes", selected == 0);
        Self::draw_button(screen, btn_y, x + 15, "No", selected == 1);
        Self::draw_button(screen, btn_y, x + 23, "Cancel", selected == 2);

        // Cache layout for hit testing
        let layout_item = LayoutItem::vstack(vec![
            LayoutItem::leaf("title_bar").fixed_height(1),
            LayoutItem::spacer().height(Size::Flex(1)),
            LayoutItem::hstack(vec![
                LayoutItem::spacer().fixed_width(5),
                LayoutItem::leaf("yes_button").fixed_width(8),
                LayoutItem::spacer().fixed_width(2),
                LayoutItem::leaf("no_button").fixed_width(7),
                LayoutItem::spacer().fixed_width(2),
                LayoutItem::leaf("cancel_button").fixed_width(11),
                LayoutItem::spacer(),
            ]).fixed_height(1),
            LayoutItem::spacer().fixed_height(1),
        ]);
        let bounds = Rect::new(x, y, width, height);
        state.dialog_layout = Some(compute_layout(&layout_item, bounds));
    }

    pub fn draw_file_dialog(screen: &mut Screen, state: &AppState, _screen_width: u16, _screen_height: u16, title: &str, selected: usize) {
        let x = state.dialog_x;
        let y = state.dialog_y;
        let width = state.dialog_width;
        let height = state.dialog_height;

        // Compute layout for the full dialog
        let bounds = Rect::new(x, y, width, height);
        let layout = compute_layout(&file_dialog_layout(), bounds);

        // Draw shadow
        screen.draw_shadow(y, x, width, height);

        // Draw dialog background
        screen.fill(y, x, width, height, ' ', Color::Black, Color::LightGray);

        // Draw border
        screen.draw_box(y, x, width, height, Color::Black, Color::LightGray);

        // Draw title bar elements
        if let Some(rect) = layout.get("title_bar") {
            let title_str = format!(" {} ", title);
            let title_x = rect.x + (rect.width.saturating_sub(title_str.len() as u16)) / 2;
            screen.write_str(rect.y, title_x, &title_str, Color::LightGray, Color::Black);
        }
        if let Some(rect) = layout.get("maximize") {
            // Show ↓ when maximized (to restore), ↑ when normal (to maximize)
            let icon = if state.dialog_saved_bounds.is_some() { "[↓]" } else { "[↑]" };
            screen.write_str(rect.y, rect.x, icon, Color::Black, Color::LightGray);
        }
        if let Some(rect) = layout.get("close") {
            screen.write_str(rect.y, rect.x, "[X]", Color::Black, Color::LightGray);
        }

        // Get current directory from state
        let cwd = state.dialog_path.to_string_lossy().to_string();

        // File name field
        if let Some(label_rect) = layout.get("filename_label") {
            screen.write_str(label_rect.y, label_rect.x, "File Name:", Color::Black, Color::LightGray);
        }
        if let Some(field_rect) = layout.get("filename_field") {
            screen.fill(field_rect.y, field_rect.x, field_rect.width, 1, ' ', Color::Black, Color::White);
            let display = if state.dialog_filename.is_empty() { "*.BAS" } else { &state.dialog_filename };
            let truncated: String = display.chars().take(field_rect.width as usize).collect();
            screen.write_str(field_rect.y, field_rect.x, &truncated, Color::Black, Color::White);
        }

        // Directory field
        if let Some(label_rect) = layout.get("directory_label") {
            screen.write_str(label_rect.y, label_rect.x, "Directory:", Color::Black, Color::LightGray);
        }
        if let Some(field_rect) = layout.get("directory_field") {
            screen.fill(field_rect.y, field_rect.x, field_rect.width, 1, ' ', Color::Black, Color::White);
            let max_len = field_rect.width as usize;
            let dir_display = if cwd.len() > max_len { &cwd[cwd.len()-max_len..] } else { &cwd };
            screen.write_str(field_rect.y, field_rect.x, dir_display, Color::Black, Color::White);
        }

        // Files label and list
        if let Some(rect) = layout.get("files_label") {
            screen.write_str(rect.y, rect.x, "Files:", Color::Black, Color::LightGray);
        }
        if let Some(list_rect) = layout.get("files_list") {
            screen.draw_box(list_rect.y, list_rect.x, list_rect.width, list_rect.height, Color::Black, Color::White);
            let max_items = list_rect.height.saturating_sub(2) as usize;
            let item_width = list_rect.width.saturating_sub(2) as usize;

            for (i, file) in state.dialog_files.iter().take(max_items).enumerate() {
                let is_selected = i == state.dialog_file_index;
                let fg = if is_selected { Color::White } else { Color::Black };
                let bg = if is_selected { Color::Cyan } else { Color::White };
                let display: String = if file.len() > item_width {
                    file[..item_width].to_string()
                } else {
                    format!("{:<width$}", file, width = item_width)
                };
                screen.write_str(list_rect.y + 1 + i as u16, list_rect.x + 1, &display, fg, bg);
            }
        }

        // Directories label and list
        if let Some(rect) = layout.get("dirs_label") {
            screen.write_str(rect.y, rect.x, "Dirs/Drives:", Color::Black, Color::LightGray);
        }
        if let Some(list_rect) = layout.get("dirs_list") {
            screen.draw_box(list_rect.y, list_rect.x, list_rect.width, list_rect.height, Color::Black, Color::White);
            let max_items = list_rect.height.saturating_sub(2) as usize;
            let item_width = list_rect.width.saturating_sub(2) as usize;

            for (i, dir) in state.dialog_dirs.iter().take(max_items).enumerate() {
                let is_selected = i == state.dialog_dir_index;
                let fg = if is_selected { Color::White } else { Color::Black };
                let bg = if is_selected { Color::Cyan } else { Color::White };
                let display_name = format!("[{}]", dir);
                let display: String = if display_name.len() > item_width {
                    display_name[..item_width].to_string()
                } else {
                    format!("{:<width$}", display_name, width = item_width)
                };
                screen.write_str(list_rect.y + 1 + i as u16, list_rect.x + 1, &display, fg, bg);
            }
        }

        // Buttons
        if let Some(rect) = layout.get("ok_button") {
            Self::draw_button(screen, rect.y, rect.x, "OK", selected == 0);
        }
        if let Some(rect) = layout.get("cancel_button") {
            Self::draw_button(screen, rect.y, rect.x, "Cancel", selected == 1);
        }
        if let Some(rect) = layout.get("help_button") {
            Self::draw_button(screen, rect.y, rect.x, "Help", selected == 2);
        }
    }

    pub fn draw_find_dialog(screen: &mut Screen, state: &AppState, screen_width: u16, screen_height: u16, selected: usize) {
        let (x, y) = Self::draw_dialog_box(screen, screen_width, screen_height, "Find", 55, 10);

        // Search field
        screen.write_str(y + 2, x + 2, "Find What:", Color::Black, Color::LightGray);
        screen.fill(y + 2, x + 14, 38, 1, ' ', Color::Black, Color::White);

        // Draw the input text
        let text = &state.dialog_find_text;
        let display_text: String = text.chars().take(37).collect();
        screen.write_str(y + 2, x + 14, &display_text, Color::Black, Color::White);

        // Draw cursor if this field is focused (field 0)
        if state.dialog_input_field == 0 {
            let cursor_x = x + 14 + state.dialog_input_cursor.min(37) as u16;
            let cursor_char = text.chars().nth(state.dialog_input_cursor).unwrap_or(' ');
            screen.write_str(y + 2, cursor_x, &cursor_char.to_string(), Color::White, Color::Black);
        }

        // Options
        let case_mark = if state.search_case_sensitive { "X" } else { " " };
        let word_mark = if state.search_whole_word { "X" } else { " " };
        screen.write_str(y + 4, x + 2, &format!("[{}] Match Upper/Lowercase", case_mark), Color::Black, Color::LightGray);
        screen.write_str(y + 5, x + 2, &format!("[{}] Whole Word", word_mark), Color::Black, Color::LightGray);

        // Buttons
        Self::draw_button(screen, y + 8, x + 10, "Find", selected == 0);
        Self::draw_button(screen, y + 8, x + 25, "Cancel", selected == 1);
        Self::draw_button(screen, y + 8, x + 40, "Help", selected == 2);
    }

    pub fn draw_replace_dialog(screen: &mut Screen, state: &AppState, screen_width: u16, screen_height: u16, selected: usize) {
        let (x, y) = Self::draw_dialog_box(screen, screen_width, screen_height, "Change", 55, 12);

        // Search field
        screen.write_str(y + 2, x + 2, "Find What:", Color::Black, Color::LightGray);
        screen.fill(y + 2, x + 14, 38, 1, ' ', Color::Black, Color::White);

        // Draw find input text
        let find_text = &state.dialog_find_text;
        let find_display: String = find_text.chars().take(37).collect();
        screen.write_str(y + 2, x + 14, &find_display, Color::Black, Color::White);

        // Draw cursor on find field if focused (field 0)
        if state.dialog_input_field == 0 {
            let cursor_x = x + 14 + state.dialog_input_cursor.min(37) as u16;
            let cursor_char = find_text.chars().nth(state.dialog_input_cursor).unwrap_or(' ');
            screen.write_str(y + 2, cursor_x, &cursor_char.to_string(), Color::White, Color::Black);
        }

        // Replace field
        screen.write_str(y + 4, x + 2, "Change To:", Color::Black, Color::LightGray);
        screen.fill(y + 4, x + 14, 38, 1, ' ', Color::Black, Color::White);

        // Draw replace input text
        let replace_text = &state.dialog_replace_text;
        let replace_display: String = replace_text.chars().take(37).collect();
        screen.write_str(y + 4, x + 14, &replace_display, Color::Black, Color::White);

        // Draw cursor on replace field if focused (field 1)
        if state.dialog_input_field == 1 {
            let cursor_x = x + 14 + state.dialog_input_cursor.min(37) as u16;
            let cursor_char = replace_text.chars().nth(state.dialog_input_cursor).unwrap_or(' ');
            screen.write_str(y + 4, cursor_x, &cursor_char.to_string(), Color::White, Color::Black);
        }

        // Options
        let case_mark = if state.search_case_sensitive { "X" } else { " " };
        let word_mark = if state.search_whole_word { "X" } else { " " };
        screen.write_str(y + 6, x + 2, &format!("[{}] Match Upper/Lowercase", case_mark), Color::Black, Color::LightGray);
        screen.write_str(y + 7, x + 2, &format!("[{}] Whole Word", word_mark), Color::Black, Color::LightGray);

        // Buttons
        Self::draw_button(screen, y + 10, x + 5, "Find & Verify", selected == 0);
        Self::draw_button(screen, y + 10, x + 22, "Change All", selected == 1);
        Self::draw_button(screen, y + 10, x + 36, "Cancel", selected == 2);
    }

    pub fn draw_goto_dialog(screen: &mut Screen, state: &AppState, screen_width: u16, screen_height: u16, selected: usize) {
        let (x, y) = Self::draw_dialog_box(screen, screen_width, screen_height, "Go To Line", 40, 7);

        screen.write_str(y + 2, x + 2, "Line number:", Color::Black, Color::LightGray);
        screen.fill(y + 2, x + 16, 20, 1, ' ', Color::Black, Color::White);

        // Draw the input text
        let text = &state.dialog_goto_line;
        let display_text: String = text.chars().take(19).collect();
        screen.write_str(y + 2, x + 16, &display_text, Color::Black, Color::White);

        // Draw cursor
        let cursor_x = x + 16 + state.dialog_input_cursor.min(19) as u16;
        let cursor_char = text.chars().nth(state.dialog_input_cursor).unwrap_or(' ');
        screen.write_str(y + 2, cursor_x, &cursor_char.to_string(), Color::White, Color::Black);

        Self::draw_button(screen, y + 5, x + 8, "OK", selected == 0);
        Self::draw_button(screen, y + 5, x + 20, "Cancel", selected == 1);
    }

    fn draw_help(screen: &mut Screen, state: &mut AppState, _screen_width: u16, _screen_height: u16, topic: &str, selected: usize) {
        let x = state.dialog_x;
        let y = state.dialog_y;
        let width = state.dialog_width;
        let height = state.dialog_height;

        Self::draw_dialog_box_at(screen, x, y, width, height, &format!("Help: {}", topic));

        let help_text = get_help_text(topic);
        for (i, line) in help_text.lines().take((height - 4) as usize).enumerate() {
            screen.write_str(y + 2 + i as u16, x + 2, line, Color::Black, Color::LightGray);
        }

        let btn_x = x + (width - 10) / 2;
        Self::draw_button(screen, y + height - 2, btn_x, "Close", selected == 0);

        // Cache layout for hit testing
        let layout_item = LayoutItem::vstack(vec![
            LayoutItem::leaf("title_bar").fixed_height(1),
            LayoutItem::spacer().height(Size::Flex(1)),
            LayoutItem::hstack(vec![
                LayoutItem::spacer(),
                LayoutItem::leaf("close_button").fixed_width(11),
                LayoutItem::spacer(),
            ]).fixed_height(1),
            LayoutItem::spacer().fixed_height(1),
        ]);
        let bounds = Rect::new(x, y, width, height);
        state.dialog_layout = Some(compute_layout(&layout_item, bounds));
    }

    fn draw_new_program(screen: &mut Screen, state: &mut AppState, _screen_width: u16, _screen_height: u16, selected: usize) {
        let x = state.dialog_x;
        let y = state.dialog_y;
        let width = state.dialog_width;
        let height = state.dialog_height;

        Self::draw_dialog_box_at(screen, x, y, width, height, "New Program");

        screen.write_str(y + 2, x + 2, "Current program will be cleared.", Color::Black, Color::LightGray);
        screen.write_str(y + 3, x + 2, "Save it first?", Color::Black, Color::LightGray);

        Self::draw_button(screen, y + height - 2, x + 5, "Yes", selected == 0);
        Self::draw_button(screen, y + height - 2, x + 15, "No", selected == 1);
        Self::draw_button(screen, y + height - 2, x + 23, "Cancel", selected == 2);

        // Cache layout for hit testing
        let layout_item = LayoutItem::vstack(vec![
            LayoutItem::leaf("title_bar").fixed_height(1),
            LayoutItem::spacer().height(Size::Flex(1)),
            LayoutItem::hstack(vec![
                LayoutItem::spacer().fixed_width(5),
                LayoutItem::leaf("yes_button").fixed_width(8),
                LayoutItem::spacer().fixed_width(2),
                LayoutItem::leaf("no_button").fixed_width(7),
                LayoutItem::spacer().fixed_width(2),
                LayoutItem::leaf("cancel_button").fixed_width(11),
                LayoutItem::spacer(),
            ]).fixed_height(1),
            LayoutItem::spacer().fixed_height(1),
        ]);
        let bounds = Rect::new(x, y, width, height);
        state.dialog_layout = Some(compute_layout(&layout_item, bounds));
    }

    fn draw_print_dialog(screen: &mut Screen, state: &mut AppState, _screen_width: u16, _screen_height: u16, selected: usize) {
        let x = state.dialog_x;
        let y = state.dialog_y;
        let width = state.dialog_width;
        let height = state.dialog_height;

        Self::draw_dialog_box_at(screen, x, y, width, height, "Print");

        screen.write_str(y + 2, x + 2, "Print:", Color::Black, Color::LightGray);
        screen.write_str(y + 3, x + 4, "(o) Selected Text Only", Color::Black, Color::LightGray);
        screen.write_str(y + 4, x + 4, "( ) Current Window", Color::Black, Color::LightGray);
        screen.write_str(y + 5, x + 4, "( ) Entire Program", Color::Black, Color::LightGray);

        Self::draw_button(screen, y + height - 2, x + 10, "OK", selected == 0);
        Self::draw_button(screen, y + height - 2, x + 25, "Cancel", selected == 1);

        // Cache layout for hit testing
        let layout_item = LayoutItem::vstack(vec![
            LayoutItem::leaf("title_bar").fixed_height(1),
            LayoutItem::spacer().height(Size::Flex(1)),
            LayoutItem::hstack(vec![
                LayoutItem::spacer().fixed_width(10),
                LayoutItem::leaf("ok_button").fixed_width(8),
                LayoutItem::spacer().fixed_width(7),
                LayoutItem::leaf("cancel_button").fixed_width(11),
                LayoutItem::spacer(),
            ]).fixed_height(1),
            LayoutItem::spacer().fixed_height(1),
        ]);
        let bounds = Rect::new(x, y, width, height);
        state.dialog_layout = Some(compute_layout(&layout_item, bounds));
    }

    fn draw_welcome(screen: &mut Screen, state: &mut AppState, _screen_width: u16, _screen_height: u16, selected: usize) {
        let x = state.dialog_x;
        let y = state.dialog_y;
        let width = state.dialog_width;
        let height = state.dialog_height;

        Self::draw_dialog_box_at(screen, x, y, width, height, "Welcome");

        // Welcome message matching real QBasic
        let center_x = x + width / 2;

        // Title line
        let title = "Welcome to MS-DOS QBasic";
        screen.write_str(y + 2, center_x - (title.len() as u16 / 2), title, Color::Black, Color::LightGray);

        // Copyright
        let copy1 = "Copyright (C) Microsoft Corporation, 1987-1992.";
        screen.write_str(y + 4, center_x - (copy1.len() as u16 / 2), copy1, Color::Black, Color::LightGray);

        // Options with angle brackets - highlighted based on selection
        let opt1 = "< Press Enter to see the Survival Guide >";
        let opt2 = "< Press ESC to clear this dialog box >";

        let (fg1, bg1) = if selected == 0 { (Color::White, Color::Black) } else { (Color::Black, Color::LightGray) };
        let (fg2, bg2) = if selected == 1 { (Color::White, Color::Black) } else { (Color::Black, Color::LightGray) };

        screen.write_str(y + 7, center_x - (opt1.len() as u16 / 2), opt1, fg1, bg1);
        screen.write_str(y + 9, center_x - (opt2.len() as u16 / 2), opt2, fg2, bg2);

        // Cache layout for hit testing
        let layout_item = LayoutItem::vstack(vec![
            LayoutItem::leaf("title_bar").fixed_height(1),
            LayoutItem::spacer().fixed_height(6),
            LayoutItem::hstack(vec![
                LayoutItem::spacer(),
                LayoutItem::leaf("start_button").fixed_width(opt1.len() as u16),
                LayoutItem::spacer(),
            ]).fixed_height(1),
            LayoutItem::spacer().fixed_height(1),
            LayoutItem::hstack(vec![
                LayoutItem::spacer(),
                LayoutItem::leaf("exit_button").fixed_width(opt2.len() as u16),
                LayoutItem::spacer(),
            ]).fixed_height(1),
            LayoutItem::spacer().height(Size::Flex(1)),
        ]);
        let bounds = Rect::new(x, y, width, height);
        state.dialog_layout = Some(compute_layout(&layout_item, bounds));
    }

    fn draw_new_sub(screen: &mut Screen, state: &mut AppState, _screen_width: u16, _screen_height: u16, selected: usize) {
        let x = state.dialog_x;
        let y = state.dialog_y;
        let width = state.dialog_width;
        let height = state.dialog_height;

        Self::draw_dialog_box_at(screen, x, y, width, height, "New SUB");

        screen.write_str(y + 2, x + 2, "SUB name:", Color::Black, Color::LightGray);
        screen.fill(y + 2, x + 13, width - 16, 1, ' ', Color::Black, Color::White);

        // Draw the input text
        let text = &state.dialog_find_text; // Reuse dialog_find_text for input
        let max_len = (width - 17) as usize;
        let display_text: String = text.chars().take(max_len).collect();
        screen.write_str(y + 2, x + 13, &display_text, Color::Black, Color::White);

        // Draw cursor
        let cursor_x = x + 13 + state.dialog_input_cursor.min(max_len) as u16;
        let cursor_char = text.chars().nth(state.dialog_input_cursor).unwrap_or(' ');
        screen.write_str(y + 2, cursor_x, &cursor_char.to_string(), Color::White, Color::Black);

        Self::draw_button(screen, y + height - 2, x + 8, "OK", selected == 0);
        Self::draw_button(screen, y + height - 2, x + 22, "Cancel", selected == 1);

        let layout_item = LayoutItem::vstack(vec![
            LayoutItem::leaf("title_bar").fixed_height(1),
            LayoutItem::spacer().height(Size::Flex(1)),
            LayoutItem::hstack(vec![
                LayoutItem::spacer().fixed_width(8),
                LayoutItem::leaf("ok_button").fixed_width(8),
                LayoutItem::spacer().fixed_width(6),
                LayoutItem::leaf("cancel_button").fixed_width(11),
                LayoutItem::spacer(),
            ]).fixed_height(1),
            LayoutItem::spacer().fixed_height(1),
        ]);
        let bounds = Rect::new(x, y, width, height);
        state.dialog_layout = Some(compute_layout(&layout_item, bounds));
    }

    fn draw_new_function(screen: &mut Screen, state: &mut AppState, _screen_width: u16, _screen_height: u16, selected: usize) {
        let x = state.dialog_x;
        let y = state.dialog_y;
        let width = state.dialog_width;
        let height = state.dialog_height;

        Self::draw_dialog_box_at(screen, x, y, width, height, "New FUNCTION");

        screen.write_str(y + 2, x + 2, "FUNCTION name:", Color::Black, Color::LightGray);
        screen.fill(y + 2, x + 18, width - 21, 1, ' ', Color::Black, Color::White);

        // Draw the input text
        let text = &state.dialog_find_text;
        let max_len = (width - 22) as usize;
        let display_text: String = text.chars().take(max_len).collect();
        screen.write_str(y + 2, x + 18, &display_text, Color::Black, Color::White);

        // Draw cursor
        let cursor_x = x + 18 + state.dialog_input_cursor.min(max_len) as u16;
        let cursor_char = text.chars().nth(state.dialog_input_cursor).unwrap_or(' ');
        screen.write_str(y + 2, cursor_x, &cursor_char.to_string(), Color::White, Color::Black);

        Self::draw_button(screen, y + height - 2, x + 8, "OK", selected == 0);
        Self::draw_button(screen, y + height - 2, x + 22, "Cancel", selected == 1);

        let layout_item = LayoutItem::vstack(vec![
            LayoutItem::leaf("title_bar").fixed_height(1),
            LayoutItem::spacer().height(Size::Flex(1)),
            LayoutItem::hstack(vec![
                LayoutItem::spacer().fixed_width(8),
                LayoutItem::leaf("ok_button").fixed_width(8),
                LayoutItem::spacer().fixed_width(6),
                LayoutItem::leaf("cancel_button").fixed_width(11),
                LayoutItem::spacer(),
            ]).fixed_height(1),
            LayoutItem::spacer().fixed_height(1),
        ]);
        let bounds = Rect::new(x, y, width, height);
        state.dialog_layout = Some(compute_layout(&layout_item, bounds));
    }

    fn draw_find_label(screen: &mut Screen, state: &mut AppState, _screen_width: u16, _screen_height: u16, selected: usize) {
        let x = state.dialog_x;
        let y = state.dialog_y;
        let width = state.dialog_width;
        let height = state.dialog_height;

        Self::draw_dialog_box_at(screen, x, y, width, height, "Find Label");

        screen.write_str(y + 2, x + 2, "Label:", Color::Black, Color::LightGray);
        screen.fill(y + 2, x + 10, width - 13, 1, ' ', Color::Black, Color::White);

        // Draw the input text
        let text = &state.dialog_find_text;
        let max_len = (width - 14) as usize;
        let display_text: String = text.chars().take(max_len).collect();
        screen.write_str(y + 2, x + 10, &display_text, Color::Black, Color::White);

        // Draw cursor
        let cursor_x = x + 10 + state.dialog_input_cursor.min(max_len) as u16;
        let cursor_char = text.chars().nth(state.dialog_input_cursor).unwrap_or(' ');
        screen.write_str(y + 2, cursor_x, &cursor_char.to_string(), Color::White, Color::Black);

        Self::draw_button(screen, y + height - 2, x + 8, "OK", selected == 0);
        Self::draw_button(screen, y + height - 2, x + 22, "Cancel", selected == 1);

        let layout_item = LayoutItem::vstack(vec![
            LayoutItem::leaf("title_bar").fixed_height(1),
            LayoutItem::spacer().height(Size::Flex(1)),
            LayoutItem::hstack(vec![
                LayoutItem::spacer().fixed_width(8),
                LayoutItem::leaf("ok_button").fixed_width(8),
                LayoutItem::spacer().fixed_width(6),
                LayoutItem::leaf("cancel_button").fixed_width(11),
                LayoutItem::spacer(),
            ]).fixed_height(1),
            LayoutItem::spacer().fixed_height(1),
        ]);
        let bounds = Rect::new(x, y, width, height);
        state.dialog_layout = Some(compute_layout(&layout_item, bounds));
    }

    fn draw_command_args(screen: &mut Screen, state: &mut AppState, _screen_width: u16, _screen_height: u16, selected: usize) {
        let x = state.dialog_x;
        let y = state.dialog_y;
        let width = state.dialog_width;
        let height = state.dialog_height;

        Self::draw_dialog_box_at(screen, x, y, width, height, "Modify COMMAND$");

        screen.write_str(y + 2, x + 2, "Command line:", Color::Black, Color::LightGray);
        screen.fill(y + 2, x + 17, width - 20, 1, ' ', Color::Black, Color::White);

        // Draw the input text (use command_args instead of dialog_find_text)
        let text = &state.command_args;
        let max_len = (width - 21) as usize;
        let display_text: String = text.chars().take(max_len).collect();
        screen.write_str(y + 2, x + 17, &display_text, Color::Black, Color::White);

        // Draw cursor
        let cursor_x = x + 17 + state.dialog_input_cursor.min(max_len) as u16;
        let cursor_char = text.chars().nth(state.dialog_input_cursor).unwrap_or(' ');
        screen.write_str(y + 2, cursor_x, &cursor_char.to_string(), Color::White, Color::Black);

        Self::draw_button(screen, y + height - 2, x + 12, "OK", selected == 0);
        Self::draw_button(screen, y + height - 2, x + 28, "Cancel", selected == 1);

        let layout_item = LayoutItem::vstack(vec![
            LayoutItem::leaf("title_bar").fixed_height(1),
            LayoutItem::spacer().height(Size::Flex(1)),
            LayoutItem::hstack(vec![
                LayoutItem::spacer().fixed_width(12),
                LayoutItem::leaf("ok_button").fixed_width(8),
                LayoutItem::spacer().fixed_width(8),
                LayoutItem::leaf("cancel_button").fixed_width(11),
                LayoutItem::spacer(),
            ]).fixed_height(1),
            LayoutItem::spacer().fixed_height(1),
        ]);
        let bounds = Rect::new(x, y, width, height);
        state.dialog_layout = Some(compute_layout(&layout_item, bounds));
    }

    fn draw_help_path(screen: &mut Screen, state: &mut AppState, _screen_width: u16, _screen_height: u16, selected: usize) {
        let x = state.dialog_x;
        let y = state.dialog_y;
        let width = state.dialog_width;
        let height = state.dialog_height;

        Self::draw_dialog_box_at(screen, x, y, width, height, "Help Path");

        screen.write_str(y + 2, x + 2, "Path:", Color::Black, Color::LightGray);
        screen.fill(y + 2, x + 9, width - 12, 1, ' ', Color::Black, Color::White);

        // Draw the input text
        let text = &state.help_path;
        let max_len = (width - 13) as usize;
        let display_text: String = text.chars().take(max_len).collect();
        screen.write_str(y + 2, x + 9, &display_text, Color::Black, Color::White);

        // Draw cursor
        let cursor_x = x + 9 + state.dialog_input_cursor.min(max_len) as u16;
        let cursor_char = text.chars().nth(state.dialog_input_cursor).unwrap_or(' ');
        screen.write_str(y + 2, cursor_x, &cursor_char.to_string(), Color::White, Color::Black);

        Self::draw_button(screen, y + height - 2, x + 12, "OK", selected == 0);
        Self::draw_button(screen, y + height - 2, x + 28, "Cancel", selected == 1);

        let layout_item = LayoutItem::vstack(vec![
            LayoutItem::leaf("title_bar").fixed_height(1),
            LayoutItem::spacer().height(Size::Flex(1)),
            LayoutItem::hstack(vec![
                LayoutItem::spacer().fixed_width(12),
                LayoutItem::leaf("ok_button").fixed_width(8),
                LayoutItem::spacer().fixed_width(8),
                LayoutItem::leaf("cancel_button").fixed_width(11),
                LayoutItem::spacer(),
            ]).fixed_height(1),
            LayoutItem::spacer().fixed_height(1),
        ]);
        let bounds = Rect::new(x, y, width, height);
        state.dialog_layout = Some(compute_layout(&layout_item, bounds));
    }

    fn draw_display_options(screen: &mut Screen, state: &mut AppState, _screen_width: u16, _screen_height: u16, selected: usize) {
        let x = state.dialog_x;
        let y = state.dialog_y;
        let width = state.dialog_width;
        let height = state.dialog_height;

        Self::draw_dialog_box_at(screen, x, y, width, height, "Display");

        // Tab Stops
        screen.write_str(y + 2, x + 2, "Tab Stops:", Color::Black, Color::LightGray);
        screen.fill(y + 2, x + 14, 5, 1, ' ', Color::Black, Color::White);
        let tab_str = state.tab_stops.to_string();
        screen.write_str(y + 2, x + 14, &tab_str, Color::Black, Color::White);

        // Show cursor on tab stops field if it's the active input field
        if state.dialog_input_field == 0 {
            let cursor_x = x + 14 + state.dialog_input_cursor.min(4) as u16;
            let cursor_char = tab_str.chars().nth(state.dialog_input_cursor).unwrap_or(' ');
            screen.write_str(y + 2, cursor_x, &cursor_char.to_string(), Color::White, Color::Black);
        }

        // Scroll Bars checkbox
        let scroll_mark = if state.show_scrollbars { "X" } else { " " };
        let (scroll_fg, scroll_bg) = if state.dialog_input_field == 1 {
            (Color::White, Color::Black)
        } else {
            (Color::Black, Color::LightGray)
        };
        screen.write_str(y + 4, x + 2, &format!("[{}] Scroll Bars", scroll_mark), scroll_fg, scroll_bg);

        // Color Scheme
        screen.write_str(y + 6, x + 2, "Color Scheme:", Color::Black, Color::LightGray);
        let schemes = ["Classic Blue", "Dark", "Light"];
        for (i, scheme) in schemes.iter().enumerate() {
            let mark = if state.color_scheme == i { "o" } else { " " };
            let (fg, bg) = if state.dialog_input_field == 2 + i {
                (Color::White, Color::Black)
            } else {
                (Color::Black, Color::LightGray)
            };
            screen.write_str(y + 7 + i as u16, x + 4, &format!("({}) {}", mark, scheme), fg, bg);
        }

        Self::draw_button(screen, y + height - 2, x + 12, "OK", selected == 0);
        Self::draw_button(screen, y + height - 2, x + 28, "Cancel", selected == 1);

        let layout_item = LayoutItem::vstack(vec![
            LayoutItem::leaf("title_bar").fixed_height(1),
            LayoutItem::spacer().height(Size::Flex(1)),
            LayoutItem::hstack(vec![
                LayoutItem::spacer().fixed_width(12),
                LayoutItem::leaf("ok_button").fixed_width(8),
                LayoutItem::spacer().fixed_width(8),
                LayoutItem::leaf("cancel_button").fixed_width(11),
                LayoutItem::spacer(),
            ]).fixed_height(1),
            LayoutItem::spacer().fixed_height(1),
        ]);
        let bounds = Rect::new(x, y, width, height);
        state.dialog_layout = Some(compute_layout(&layout_item, bounds));
    }
}

/// Get help text for a topic
fn get_help_text(topic: &str) -> &'static str {
    match topic.to_uppercase().as_str() {
        "PRINT" => r#"
PRINT Statement

Outputs data to the screen.

Syntax:
  PRINT [expression] [{,|;} expression]...

Examples:
  PRINT "Hello, World!"
  PRINT "Sum is"; 2 + 2
  PRINT A, B, C
"#,
        "FOR" => r#"
FOR...NEXT Statement

Repeats a group of statements a specified number of times.

Syntax:
  FOR counter = start TO end [STEP increment]
    [statements]
  NEXT [counter]

Example:
  FOR i = 1 TO 10
    PRINT i
  NEXT i
"#,
        "IF" => r#"
IF...THEN...ELSE Statement

Allows conditional execution of statements.

Syntax:
  IF condition THEN
    [statements]
  [ELSEIF condition THEN
    [statements]]
  [ELSE
    [statements]]
  END IF

Example:
  IF x > 10 THEN
    PRINT "Big"
  ELSE
    PRINT "Small"
  END IF
"#,
        _ => r#"
QBasic IDE Simulator - Help

Welcome to the QBasic IDE simulator!

Navigation:
  F1  - Help
  F2  - View SUBs
  F5  - Run program
  F8  - Step through code
  F9  - Toggle breakpoint
  F10 - Access menu

Use Alt+letter to access menus (Alt+F for File, etc.)

Press Escape or Enter to close this dialog.
"#,
    }
}
