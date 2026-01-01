//! QBasic-style dialog boxes

use crate::screen::Screen;
use crate::terminal::Color;
use crate::state::{AppState, DialogType};
use super::layout::{Rect, LayoutItem, Size, compute_layout, file_dialog_layout, find_dialog_layout, replace_dialog_layout, goto_line_dialog_layout, print_dialog_layout, welcome_dialog_layout, simple_input_dialog_layout, display_options_dialog_layout};
use super::scrollbar::{ScrollbarState, ScrollbarColors, draw_vertical};

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

        // Title bar - gray background with black text, centered
        let title_str = format!(" {} ", title);
        let title_x = x + (width.saturating_sub(title_str.len() as u16)) / 2;
        screen.write_str(y, title_x, &title_str, Color::Black, Color::LightGray);
    }

    /// Legacy helper for dialogs that don't use state positioning yet
    #[allow(dead_code)]
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
            // Title bar with window controls
            LayoutItem::hstack(vec![
                LayoutItem::leaf("title_bar").width(Size::Flex(1)),
                LayoutItem::leaf("maximize").fixed_width(3),
                LayoutItem::leaf("close").fixed_width(3),
            ]).fixed_height(1),
            LayoutItem::spacer().height(Size::Flex(1)),
            LayoutItem::hstack(vec![
                LayoutItem::spacer(),
                LayoutItem::leaf("ok_button").fixed_width(btn_width + 4), // "< OK >"
                LayoutItem::spacer(),
            ]).fixed_height(1),
            LayoutItem::spacer().fixed_height(1),
            // Resize handle
            LayoutItem::hstack(vec![
                LayoutItem::spacer(),
                LayoutItem::leaf("resize_handle").fixed_width(2),
            ]).fixed_height(1),
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
            // Title bar with window controls
            LayoutItem::hstack(vec![
                LayoutItem::leaf("title_bar").width(Size::Flex(1)),
                LayoutItem::leaf("maximize").fixed_width(3),
                LayoutItem::leaf("close").fixed_width(3),
            ]).fixed_height(1),
            LayoutItem::spacer().height(Size::Flex(1)),
            LayoutItem::hstack(vec![
                LayoutItem::spacer(),
                LayoutItem::leaf("ok_button").fixed_width(10),
                LayoutItem::spacer(),
            ]).fixed_height(1),
            LayoutItem::spacer().fixed_height(1),
            // Resize handle
            LayoutItem::hstack(vec![
                LayoutItem::spacer(),
                LayoutItem::leaf("resize_handle").fixed_width(2),
            ]).fixed_height(1),
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
            // Title bar with window controls
            LayoutItem::hstack(vec![
                LayoutItem::leaf("title_bar").width(Size::Flex(1)),
                LayoutItem::leaf("maximize").fixed_width(3),
                LayoutItem::leaf("close").fixed_width(3),
            ]).fixed_height(1),
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
            // Resize handle
            LayoutItem::hstack(vec![
                LayoutItem::spacer(),
                LayoutItem::leaf("resize_handle").fixed_width(2),
            ]).fixed_height(1),
        ]);
        let bounds = Rect::new(x, y, width, height);
        state.dialog_layout = Some(compute_layout(&layout_item, bounds));
    }

    pub fn draw_file_dialog(screen: &mut Screen, state: &AppState, _screen_width: u16, _screen_height: u16, title: &str, _selected: usize) {
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

        // Draw title bar (centered title, gray background)
        if let Some(rect) = layout.get("title_bar") {
            let title_str = format!(" {} ", title);
            let title_x = rect.x + (rect.width.saturating_sub(title_str.len() as u16)) / 2;
            screen.write_str(rect.y, title_x, &title_str, Color::Black, Color::LightGray);
        }

        // Get current directory from state
        let cwd = state.dialog_path.to_string_lossy().to_string();

        // Current field: 0=filename, 1=directory, 2=files, 3=dirs, 4=OK, 5=Cancel, 6=Help
        let current_field = state.dialog_input_field;

        // File name field
        if let Some(label_rect) = layout.get("filename_label") {
            let (fg, bg) = if current_field == 0 { (Color::White, Color::Black) } else { (Color::Black, Color::LightGray) };
            screen.write_str(label_rect.y, label_rect.x, "File Name:", fg, bg);
        }
        if let Some(field_rect) = layout.get("filename_field") {
            let (fg, bg) = if current_field == 0 { (Color::Black, Color::Cyan) } else { (Color::Black, Color::LightGray) };
            screen.fill(field_rect.y, field_rect.x, field_rect.width, 1, ' ', fg, bg);
            let display = if state.dialog_filename.is_empty() { "*.BAS" } else { &state.dialog_filename };
            let truncated: String = display.chars().take(field_rect.width as usize).collect();
            screen.write_str(field_rect.y, field_rect.x, &truncated, fg, bg);

            // Draw cursor if focused
            if current_field == 0 {
                let cursor_x = field_rect.x + state.dialog_input_cursor.min(field_rect.width as usize - 1) as u16;
                let cursor_char = state.dialog_filename.chars().nth(state.dialog_input_cursor).unwrap_or(' ');
                screen.write_str(field_rect.y, cursor_x, &cursor_char.to_string(), Color::White, Color::Black);
            }
        }

        // Directory field (read-only, just shows focus)
        if let Some(label_rect) = layout.get("directory_label") {
            let (fg, bg) = if current_field == 1 { (Color::White, Color::Black) } else { (Color::Black, Color::LightGray) };
            screen.write_str(label_rect.y, label_rect.x, "Directory:", fg, bg);
        }
        if let Some(field_rect) = layout.get("directory_field") {
            let (fg, bg) = if current_field == 1 { (Color::Black, Color::Cyan) } else { (Color::Black, Color::LightGray) };
            screen.fill(field_rect.y, field_rect.x, field_rect.width, 1, ' ', fg, bg);
            let max_len = field_rect.width as usize;
            let dir_display = if cwd.len() > max_len { &cwd[cwd.len()-max_len..] } else { &cwd };
            screen.write_str(field_rect.y, field_rect.x, dir_display, fg, bg);
        }

        // Files label and list
        if let Some(rect) = layout.get("files_label") {
            let (fg, bg) = if current_field == 2 { (Color::White, Color::Black) } else { (Color::Black, Color::LightGray) };
            screen.write_str(rect.y, rect.x, "Files:", fg, bg);
        }
        if let Some(list_rect) = layout.get("files_list") {
            let box_fg = if current_field == 2 { Color::Black } else { Color::Black };
            let box_bg = if current_field == 2 { Color::Cyan } else { Color::LightGray };
            screen.draw_box(list_rect.y, list_rect.x, list_rect.width, list_rect.height, box_fg, box_bg);
            let max_items = list_rect.height.saturating_sub(2) as usize;
            let item_width = list_rect.width.saturating_sub(2) as usize;

            // Calculate scroll offset to keep selected item visible
            let scroll_offset = if state.dialog_file_index >= max_items {
                state.dialog_file_index - max_items + 1
            } else {
                0
            };

            for (i, file) in state.dialog_files.iter().skip(scroll_offset).take(max_items).enumerate() {
                let actual_index = scroll_offset + i;
                let is_selected = actual_index == state.dialog_file_index;
                let fg = if is_selected { Color::White } else { Color::Black };
                let bg = if is_selected { Color::Cyan } else { Color::LightGray };
                let display: String = if file.len() > item_width {
                    file[..item_width].to_string()
                } else {
                    format!("{:<width$}", file, width = item_width)
                };
                screen.write_str(list_rect.y + 1 + i as u16, list_rect.x + 1, &display, fg, bg);
            }

            // Draw scrollbar if there are more files than visible
            if state.dialog_files.len() > max_items {
                let scrollbar_state = ScrollbarState::new(
                    state.dialog_file_index,
                    state.dialog_files.len(),
                    max_items,
                );
                let colors = ScrollbarColors::default();
                let scrollbar_col = list_rect.x + list_rect.width - 1;
                draw_vertical(screen, scrollbar_col, list_rect.y + 1, list_rect.y + list_rect.height - 2, &scrollbar_state, &colors);
            }
        }

        // Directories label and list
        if let Some(rect) = layout.get("dirs_label") {
            let (fg, bg) = if current_field == 3 { (Color::White, Color::Black) } else { (Color::Black, Color::LightGray) };
            screen.write_str(rect.y, rect.x, "Dirs/Drives:", fg, bg);
        }
        if let Some(list_rect) = layout.get("dirs_list") {
            let box_fg = if current_field == 3 { Color::Black } else { Color::Black };
            let box_bg = if current_field == 3 { Color::Cyan } else { Color::LightGray };
            screen.draw_box(list_rect.y, list_rect.x, list_rect.width, list_rect.height, box_fg, box_bg);
            let max_items = list_rect.height.saturating_sub(2) as usize;
            let item_width = list_rect.width.saturating_sub(2) as usize;

            // Calculate scroll offset to keep selected item visible
            let scroll_offset = if state.dialog_dir_index >= max_items {
                state.dialog_dir_index - max_items + 1
            } else {
                0
            };

            for (i, dir) in state.dialog_dirs.iter().skip(scroll_offset).take(max_items).enumerate() {
                let actual_index = scroll_offset + i;
                let is_selected = actual_index == state.dialog_dir_index;
                let fg = if is_selected { Color::White } else { Color::Black };
                let bg = if is_selected { Color::Cyan } else { Color::LightGray };
                let display_name = format!("[{}]", dir);
                let display: String = if display_name.len() > item_width {
                    display_name[..item_width].to_string()
                } else {
                    format!("{:<width$}", display_name, width = item_width)
                };
                screen.write_str(list_rect.y + 1 + i as u16, list_rect.x + 1, &display, fg, bg);
            }

            // Draw scrollbar if there are more dirs than visible
            if state.dialog_dirs.len() > max_items {
                let scrollbar_state = ScrollbarState::new(
                    state.dialog_dir_index,
                    state.dialog_dirs.len(),
                    max_items,
                );
                let colors = ScrollbarColors::default();
                let scrollbar_col = list_rect.x + list_rect.width - 1;
                draw_vertical(screen, scrollbar_col, list_rect.y + 1, list_rect.y + list_rect.height - 2, &scrollbar_state, &colors);
            }
        }

        // Buttons - use current_field to determine selection
        if let Some(rect) = layout.get("ok_button") {
            Self::draw_button(screen, rect.y, rect.x, "OK", current_field == 4);
        }
        if let Some(rect) = layout.get("cancel_button") {
            Self::draw_button(screen, rect.y, rect.x, "Cancel", current_field == 5);
        }
        if let Some(rect) = layout.get("help_button") {
            Self::draw_button(screen, rect.y, rect.x, "Help", current_field == 6);
        }
    }

    pub fn draw_find_dialog(screen: &mut Screen, state: &AppState, _screen_width: u16, _screen_height: u16, _selected: usize) {
        let x = state.dialog_x;
        let y = state.dialog_y;
        let width = state.dialog_width;
        let height = state.dialog_height;

        // Compute layout
        let bounds = Rect::new(x, y, width, height);
        let layout = compute_layout(&find_dialog_layout(), bounds);

        // Draw dialog background and border
        screen.draw_shadow(y, x, width, height);
        screen.fill(y, x, width, height, ' ', Color::Black, Color::LightGray);
        screen.draw_box(y, x, width, height, Color::Black, Color::LightGray);

        // Title bar (blue with white text)
        if let Some(rect) = layout.get("title_bar") {
            // Title is just text on gray background, no fill needed
            let title = " Find ";
            let title_x = rect.x + (rect.width.saturating_sub(title.len() as u16)) / 2;
            screen.write_str(rect.y, title_x, title, Color::Black, Color::LightGray);
        }

        // Fields: 0=search, 1=case checkbox, 2=word checkbox, 3=Find, 4=Cancel, 5=Help
        let current_field = state.dialog_input_field;

        // Find label
        if let Some(rect) = layout.get("find_label") {
            let (fg, bg) = if current_field == 0 { (Color::White, Color::Black) } else { (Color::Black, Color::LightGray) };
            screen.write_str(rect.y, rect.x, "Find:", fg, bg);
        }

        // Find field
        if let Some(rect) = layout.get("find_field") {
            let (fg, bg) = if current_field == 0 { (Color::Black, Color::Cyan) } else { (Color::Black, Color::LightGray) };
            screen.fill(rect.y, rect.x, rect.width, 1, ' ', fg, bg);

            let text = &state.dialog_find_text;
            let max_chars = rect.width.saturating_sub(1) as usize;
            let display_text: String = text.chars().take(max_chars).collect();
            screen.write_str(rect.y, rect.x, &display_text, fg, bg);

            // Draw cursor if focused
            if current_field == 0 {
                let cursor_x = rect.x + state.dialog_input_cursor.min(max_chars) as u16;
                let cursor_char = text.chars().nth(state.dialog_input_cursor).unwrap_or(' ');
                screen.write_str(rect.y, cursor_x, &cursor_char.to_string(), Color::White, Color::Black);
            }
        }

        // Case checkbox
        if let Some(rect) = layout.get("case_checkbox") {
            let mark = if state.search_case_sensitive { "X" } else { " " };
            let (fg, bg) = if current_field == 1 { (Color::White, Color::Black) } else { (Color::Black, Color::LightGray) };
            screen.write_str(rect.y, rect.x, &format!("[{}] Match Case", mark), fg, bg);
        }

        // Whole word checkbox
        if let Some(rect) = layout.get("whole_checkbox") {
            let mark = if state.search_whole_word { "X" } else { " " };
            let (fg, bg) = if current_field == 2 { (Color::White, Color::Black) } else { (Color::Black, Color::LightGray) };
            screen.write_str(rect.y, rect.x, &format!("[{}] Whole Word", mark), fg, bg);
        }

        // Buttons
        if let Some(rect) = layout.get("ok_button") {
            Self::draw_button(screen, rect.y, rect.x, "Find", current_field == 3);
        }
        if let Some(rect) = layout.get("cancel_button") {
            Self::draw_button(screen, rect.y, rect.x, "Cancel", current_field == 4);
        }
        if let Some(rect) = layout.get("help_button") {
            Self::draw_button(screen, rect.y, rect.x, "Help", current_field == 5);
        }
    }

    pub fn draw_replace_dialog(screen: &mut Screen, state: &AppState, _screen_width: u16, _screen_height: u16, _selected: usize) {
        let x = state.dialog_x;
        let y = state.dialog_y;
        let width = state.dialog_width;
        let height = state.dialog_height;

        // Compute layout
        let bounds = Rect::new(x, y, width, height);
        let layout = compute_layout(&replace_dialog_layout(), bounds);

        // Draw dialog background and border
        screen.draw_shadow(y, x, width, height);
        screen.fill(y, x, width, height, ' ', Color::Black, Color::LightGray);
        screen.draw_box(y, x, width, height, Color::Black, Color::LightGray);

        // Title bar (blue with white text)
        if let Some(rect) = layout.get("title_bar") {
            // Title is just text on gray background, no fill needed
            let title = " Change ";
            let title_x = rect.x + (rect.width.saturating_sub(title.len() as u16)) / 2;
            screen.write_str(rect.y, title_x, title, Color::Black, Color::LightGray);
        }

        // Fields: 0=find, 1=replace, 2=case, 3=word, 4=Find&Verify, 5=ChangeAll, 6=Cancel
        let current_field = state.dialog_input_field;

        // Find label
        if let Some(rect) = layout.get("find_label") {
            let (fg, bg) = if current_field == 0 { (Color::White, Color::Black) } else { (Color::Black, Color::LightGray) };
            screen.write_str(rect.y, rect.x, "Find What:", fg, bg);
        }

        // Find field
        if let Some(rect) = layout.get("find_field") {
            let (fg, bg) = if current_field == 0 { (Color::Black, Color::Cyan) } else { (Color::Black, Color::LightGray) };
            screen.fill(rect.y, rect.x, rect.width, 1, ' ', fg, bg);

            let text = &state.dialog_find_text;
            let max_chars = rect.width.saturating_sub(1) as usize;
            let display_text: String = text.chars().take(max_chars).collect();
            screen.write_str(rect.y, rect.x, &display_text, fg, bg);

            if current_field == 0 {
                let cursor_x = rect.x + state.dialog_input_cursor.min(max_chars) as u16;
                let cursor_char = text.chars().nth(state.dialog_input_cursor).unwrap_or(' ');
                screen.write_str(rect.y, cursor_x, &cursor_char.to_string(), Color::White, Color::Black);
            }
        }

        // Replace label
        if let Some(rect) = layout.get("replace_label") {
            let (fg, bg) = if current_field == 1 { (Color::White, Color::Black) } else { (Color::Black, Color::LightGray) };
            screen.write_str(rect.y, rect.x, "Change To:", fg, bg);
        }

        // Replace field
        if let Some(rect) = layout.get("replace_field") {
            let (fg, bg) = if current_field == 1 { (Color::Black, Color::Cyan) } else { (Color::Black, Color::LightGray) };
            screen.fill(rect.y, rect.x, rect.width, 1, ' ', fg, bg);

            let text = &state.dialog_replace_text;
            let max_chars = rect.width.saturating_sub(1) as usize;
            let display_text: String = text.chars().take(max_chars).collect();
            screen.write_str(rect.y, rect.x, &display_text, fg, bg);

            if current_field == 1 {
                let cursor_x = rect.x + state.dialog_input_cursor.min(max_chars) as u16;
                let cursor_char = text.chars().nth(state.dialog_input_cursor).unwrap_or(' ');
                screen.write_str(rect.y, cursor_x, &cursor_char.to_string(), Color::White, Color::Black);
            }
        }

        // Case checkbox
        if let Some(rect) = layout.get("case_checkbox") {
            let mark = if state.search_case_sensitive { "X" } else { " " };
            let (fg, bg) = if current_field == 2 { (Color::White, Color::Black) } else { (Color::Black, Color::LightGray) };
            screen.write_str(rect.y, rect.x, &format!("[{}] Match Case", mark), fg, bg);
        }

        // Whole word checkbox
        if let Some(rect) = layout.get("whole_checkbox") {
            let mark = if state.search_whole_word { "X" } else { " " };
            let (fg, bg) = if current_field == 3 { (Color::White, Color::Black) } else { (Color::Black, Color::LightGray) };
            screen.write_str(rect.y, rect.x, &format!("[{}] Whole Word", mark), fg, bg);
        }

        // Buttons
        if let Some(rect) = layout.get("find_next_button") {
            Self::draw_button(screen, rect.y, rect.x, "Find Next", current_field == 4);
        }
        if let Some(rect) = layout.get("replace_button") {
            Self::draw_button(screen, rect.y, rect.x, "Replace", current_field == 5);
        }
        if let Some(rect) = layout.get("replace_all_button") {
            Self::draw_button(screen, rect.y, rect.x, "Replace All", current_field == 6);
        }
        if let Some(rect) = layout.get("cancel_button") {
            Self::draw_button(screen, rect.y, rect.x, "Cancel", current_field == 7);
        }
    }

    pub fn draw_goto_dialog(screen: &mut Screen, state: &AppState, _screen_width: u16, _screen_height: u16, _selected: usize) {
        let x = state.dialog_x;
        let y = state.dialog_y;
        let width = state.dialog_width;
        let height = state.dialog_height;

        // Compute layout
        let bounds = Rect::new(x, y, width, height);
        let layout = compute_layout(&goto_line_dialog_layout(), bounds);

        // Draw dialog background and border
        screen.draw_shadow(y, x, width, height);
        screen.fill(y, x, width, height, ' ', Color::Black, Color::LightGray);
        screen.draw_box(y, x, width, height, Color::Black, Color::LightGray);

        // Title bar (blue with white text)
        if let Some(rect) = layout.get("title_bar") {
            // Title is just text on gray background, no fill needed
            let title = " Go To Line ";
            let title_x = rect.x + (rect.width.saturating_sub(title.len() as u16)) / 2;
            screen.write_str(rect.y, title_x, title, Color::Black, Color::LightGray);
        }

        // Fields: 0=line number, 1=OK, 2=Cancel
        let current_field = state.dialog_input_field;

        // Line label
        if let Some(rect) = layout.get("line_label") {
            let (fg, bg) = if current_field == 0 { (Color::White, Color::Black) } else { (Color::Black, Color::LightGray) };
            screen.write_str(rect.y, rect.x, "Line number:", fg, bg);
        }

        // Line field
        if let Some(rect) = layout.get("line_field") {
            let (fg, bg) = if current_field == 0 { (Color::Black, Color::Cyan) } else { (Color::Black, Color::LightGray) };
            screen.fill(rect.y, rect.x, rect.width, 1, ' ', fg, bg);

            let text = &state.dialog_goto_line;
            let max_chars = rect.width.saturating_sub(1) as usize;
            let display_text: String = text.chars().take(max_chars).collect();
            screen.write_str(rect.y, rect.x, &display_text, fg, bg);

            if current_field == 0 {
                let cursor_x = rect.x + state.dialog_input_cursor.min(max_chars) as u16;
                let cursor_char = text.chars().nth(state.dialog_input_cursor).unwrap_or(' ');
                screen.write_str(rect.y, cursor_x, &cursor_char.to_string(), Color::White, Color::Black);
            }
        }

        // Buttons
        if let Some(rect) = layout.get("ok_button") {
            Self::draw_button(screen, rect.y, rect.x, "OK", current_field == 1);
        }
        if let Some(rect) = layout.get("cancel_button") {
            Self::draw_button(screen, rect.y, rect.x, "Cancel", current_field == 2);
        }
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
            // Title bar with window controls
            LayoutItem::hstack(vec![
                LayoutItem::leaf("title_bar").width(Size::Flex(1)),
                LayoutItem::leaf("maximize").fixed_width(3),
                LayoutItem::leaf("close").fixed_width(3),
            ]).fixed_height(1),
            LayoutItem::spacer().height(Size::Flex(1)),
            LayoutItem::hstack(vec![
                LayoutItem::spacer(),
                LayoutItem::leaf("close_button").fixed_width(11),
                LayoutItem::spacer(),
            ]).fixed_height(1),
            LayoutItem::spacer().fixed_height(1),
            // Resize handle
            LayoutItem::hstack(vec![
                LayoutItem::spacer(),
                LayoutItem::leaf("resize_handle").fixed_width(2),
            ]).fixed_height(1),
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
            // Title bar with window controls
            LayoutItem::hstack(vec![
                LayoutItem::leaf("title_bar").width(Size::Flex(1)),
                LayoutItem::leaf("maximize").fixed_width(3),
                LayoutItem::leaf("close").fixed_width(3),
            ]).fixed_height(1),
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
            // Resize handle
            LayoutItem::hstack(vec![
                LayoutItem::spacer(),
                LayoutItem::leaf("resize_handle").fixed_width(2),
            ]).fixed_height(1),
        ]);
        let bounds = Rect::new(x, y, width, height);
        state.dialog_layout = Some(compute_layout(&layout_item, bounds));
    }

    fn draw_print_dialog(screen: &mut Screen, state: &mut AppState, _screen_width: u16, _screen_height: u16, _selected: usize) {
        let x = state.dialog_x;
        let y = state.dialog_y;
        let width = state.dialog_width;
        let height = state.dialog_height;

        // Compute layout
        let bounds = Rect::new(x, y, width, height);
        let layout = compute_layout(&print_dialog_layout(), bounds);

        // Draw dialog background and border
        screen.draw_shadow(y, x, width, height);
        screen.fill(y, x, width, height, ' ', Color::Black, Color::LightGray);
        screen.draw_box(y, x, width, height, Color::Black, Color::LightGray);

        // Title bar (blue with white text)
        if let Some(rect) = layout.get("title_bar") {
            // Title is just text on gray background, no fill needed
            let title = " Print ";
            let title_x = rect.x + (rect.width.saturating_sub(title.len() as u16)) / 2;
            screen.write_str(rect.y, title_x, title, Color::Black, Color::LightGray);
        }

        // Fields: 0=selected text, 1=current window, 2=entire program, 3=OK, 4=Cancel
        let current_field = state.dialog_input_field;

        // Radio buttons
        if let Some(rect) = layout.get("option_selected") {
            let mark = if current_field == 0 { "o" } else { " " };
            let (fg, bg) = if current_field == 0 { (Color::White, Color::Black) } else { (Color::Black, Color::LightGray) };
            screen.write_str(rect.y, rect.x, &format!("({}) Selected Text Only", mark), fg, bg);
        }
        if let Some(rect) = layout.get("option_range") {
            let mark = if current_field == 1 { "o" } else { " " };
            let (fg, bg) = if current_field == 1 { (Color::White, Color::Black) } else { (Color::Black, Color::LightGray) };
            screen.write_str(rect.y, rect.x, &format!("({}) Current Window", mark), fg, bg);
        }

        // Buttons
        if let Some(rect) = layout.get("ok_button") {
            Self::draw_button(screen, rect.y, rect.x, "OK", current_field == 3);
        }
        if let Some(rect) = layout.get("cancel_button") {
            Self::draw_button(screen, rect.y, rect.x, "Cancel", current_field == 4);
        }

        // Cache layout for hit testing
        state.dialog_layout = Some(layout);
    }

    fn draw_welcome(screen: &mut Screen, state: &mut AppState, _screen_width: u16, _screen_height: u16, _selected: usize) {
        let x = state.dialog_x;
        let y = state.dialog_y;
        let width = state.dialog_width;
        let height = state.dialog_height;

        // Compute layout
        let bounds = Rect::new(x, y, width, height);
        let layout = compute_layout(&welcome_dialog_layout(), bounds);

        // Draw dialog background and border
        screen.draw_shadow(y, x, width, height);
        screen.fill(y, x, width, height, ' ', Color::Black, Color::LightGray);
        screen.draw_box(y, x, width, height, Color::Black, Color::LightGray);

        // Title bar (blue with white text)
        if let Some(rect) = layout.get("title_bar") {
            // Title is just text on gray background, no fill needed
            let title = " Welcome ";
            let title_x = rect.x + (rect.width.saturating_sub(title.len() as u16)) / 2;
            screen.write_str(rect.y, title_x, title, Color::Black, Color::LightGray);
        }

        // Buttons: 0=start button, 1=exit button
        let current_button = state.dialog_button;

        // Welcome text (centered in dialog)
        if let Some(rect) = layout.get("welcome_text") {
            let title = "Welcome to MS-DOS QBasic";
            let title_x = rect.x + (rect.width.saturating_sub(title.len() as u16)) / 2;
            screen.write_str(rect.y, title_x, title, Color::Black, Color::LightGray);
        }

        // Copyright text
        if let Some(rect) = layout.get("copyright") {
            let copy1 = "Copyright (C) Microsoft Corporation, 1987-1992.";
            let copy_x = rect.x + (rect.width.saturating_sub(copy1.len() as u16)) / 2;
            screen.write_str(rect.y, copy_x, copy1, Color::Black, Color::LightGray);
        }

        // Option buttons (now on separate lines)
        if let Some(rect) = layout.get("start_button") {
            let opt = "< Press Enter to see the Survival Guide >";
            let (fg, bg) = if current_button == 0 { (Color::White, Color::Black) } else { (Color::Black, Color::LightGray) };
            let opt_x = rect.x + (rect.width.saturating_sub(opt.len() as u16)) / 2;
            screen.write_str(rect.y, opt_x, opt, fg, bg);
        }
        if let Some(rect) = layout.get("exit_button") {
            let opt = "< Press ESC to clear this dialog box >";
            let (fg, bg) = if current_button == 1 { (Color::White, Color::Black) } else { (Color::Black, Color::LightGray) };
            let opt_x = rect.x + (rect.width.saturating_sub(opt.len() as u16)) / 2;
            screen.write_str(rect.y, opt_x, opt, fg, bg);
        }

        // Cache layout for hit testing
        state.dialog_layout = Some(layout);
    }

    /// Helper to draw simple input dialogs (NewSub, NewFunction, FindLabel, CommandArgs, HelpPath)
    fn draw_simple_input_dialog(screen: &mut Screen, state: &mut AppState, title: &str, label: &str, text: &str) {
        let x = state.dialog_x;
        let y = state.dialog_y;
        let width = state.dialog_width;
        let height = state.dialog_height;

        // Compute layout
        let bounds = Rect::new(x, y, width, height);
        let layout = compute_layout(&simple_input_dialog_layout(), bounds);

        // Draw dialog background and border
        screen.draw_shadow(y, x, width, height);
        screen.fill(y, x, width, height, ' ', Color::Black, Color::LightGray);
        screen.draw_box(y, x, width, height, Color::Black, Color::LightGray);

        // Title bar (blue with white text)
        if let Some(rect) = layout.get("title_bar") {
            // Title is just text on gray background, no fill needed
            let title_str = format!(" {} ", title);
            let title_x = rect.x + (rect.width.saturating_sub(title_str.len() as u16)) / 2;
            screen.write_str(rect.y, title_x, &title_str, Color::Black, Color::LightGray);
        }

        // Fields: 0=input, 1=OK, 2=Cancel
        let current_field = state.dialog_input_field;

        // Input label
        if let Some(rect) = layout.get("input_label") {
            let (fg, bg) = if current_field == 0 { (Color::White, Color::Black) } else { (Color::Black, Color::LightGray) };
            screen.write_str(rect.y, rect.x, label, fg, bg);
        }

        // Input field
        if let Some(rect) = layout.get("input_field") {
            let (fg, bg) = if current_field == 0 { (Color::Black, Color::Cyan) } else { (Color::Black, Color::LightGray) };
            screen.fill(rect.y, rect.x, rect.width, 1, ' ', fg, bg);

            let max_chars = rect.width.saturating_sub(1) as usize;
            let display_text: String = text.chars().take(max_chars).collect();
            screen.write_str(rect.y, rect.x, &display_text, fg, bg);

            if current_field == 0 {
                let cursor_x = rect.x + state.dialog_input_cursor.min(max_chars) as u16;
                let cursor_char = text.chars().nth(state.dialog_input_cursor).unwrap_or(' ');
                screen.write_str(rect.y, cursor_x, &cursor_char.to_string(), Color::White, Color::Black);
            }
        }

        // Buttons
        if let Some(rect) = layout.get("ok_button") {
            Self::draw_button(screen, rect.y, rect.x, "OK", current_field == 1);
        }
        if let Some(rect) = layout.get("cancel_button") {
            Self::draw_button(screen, rect.y, rect.x, "Cancel", current_field == 2);
        }

        // Cache layout for hit testing
        state.dialog_layout = Some(layout);
    }

    fn draw_new_sub(screen: &mut Screen, state: &mut AppState, _screen_width: u16, _screen_height: u16, _selected: usize) {
        Self::draw_simple_input_dialog(screen, state, "New SUB", "SUB name:", &state.dialog_find_text.clone());
    }

    fn draw_new_function(screen: &mut Screen, state: &mut AppState, _screen_width: u16, _screen_height: u16, _selected: usize) {
        Self::draw_simple_input_dialog(screen, state, "New FUNCTION", "FUNCTION:", &state.dialog_find_text.clone());
    }

    fn draw_find_label(screen: &mut Screen, state: &mut AppState, _screen_width: u16, _screen_height: u16, _selected: usize) {
        Self::draw_simple_input_dialog(screen, state, "Find Label", "Label:", &state.dialog_find_text.clone());
    }

    fn draw_command_args(screen: &mut Screen, state: &mut AppState, _screen_width: u16, _screen_height: u16, _selected: usize) {
        Self::draw_simple_input_dialog(screen, state, "Modify COMMAND$", "Command:", &state.command_args.clone());
    }

    fn draw_help_path(screen: &mut Screen, state: &mut AppState, _screen_width: u16, _screen_height: u16, _selected: usize) {
        Self::draw_simple_input_dialog(screen, state, "Help Path", "Path:", &state.help_path.clone());
    }

    fn draw_display_options(screen: &mut Screen, state: &mut AppState, _screen_width: u16, _screen_height: u16, _selected: usize) {
        let x = state.dialog_x;
        let y = state.dialog_y;
        let width = state.dialog_width;
        let height = state.dialog_height;

        // Compute layout
        let bounds = Rect::new(x, y, width, height);
        let layout = compute_layout(&display_options_dialog_layout(), bounds);

        // Draw dialog background and border
        screen.draw_shadow(y, x, width, height);
        screen.fill(y, x, width, height, ' ', Color::Black, Color::LightGray);
        screen.draw_box(y, x, width, height, Color::Black, Color::LightGray);

        // Title bar (blue with white text)
        if let Some(rect) = layout.get("title_bar") {
            // Title is just text on gray background, no fill needed
            let title = " Display ";
            let title_x = rect.x + (rect.width.saturating_sub(title.len() as u16)) / 2;
            screen.write_str(rect.y, title_x, title, Color::Black, Color::LightGray);
        }

        // Fields: 0=tabs, 1=scrollbars, 2=blue, 3=dark, 4=light, 5=OK, 6=Cancel
        let current_field = state.dialog_input_field;

        // Tab Stops
        if let Some(rect) = layout.get("tabs_label") {
            let (fg, bg) = if current_field == 0 { (Color::White, Color::Black) } else { (Color::Black, Color::LightGray) };
            screen.write_str(rect.y, rect.x, "Tab Stops:", fg, bg);
        }
        if let Some(rect) = layout.get("tabs_field") {
            let (fg, bg) = if current_field == 0 { (Color::Black, Color::Cyan) } else { (Color::Black, Color::LightGray) };
            screen.fill(rect.y, rect.x, rect.width, 1, ' ', fg, bg);
            let tab_str = state.tab_stops.to_string();
            screen.write_str(rect.y, rect.x, &tab_str, fg, bg);

            if current_field == 0 {
                let cursor_x = rect.x + state.dialog_input_cursor.min(rect.width as usize - 1) as u16;
                let cursor_char = tab_str.chars().nth(state.dialog_input_cursor).unwrap_or(' ');
                screen.write_str(rect.y, cursor_x, &cursor_char.to_string(), Color::White, Color::Black);
            }
        }

        // Scroll Bars checkbox
        if let Some(rect) = layout.get("scrollbars_checkbox") {
            let mark = if state.show_scrollbars { "X" } else { " " };
            let (fg, bg) = if current_field == 1 { (Color::White, Color::Black) } else { (Color::Black, Color::LightGray) };
            screen.write_str(rect.y, rect.x, &format!("[{}] Scroll Bars", mark), fg, bg);
        }

        // Color Scheme label
        if let Some(rect) = layout.get("scheme_label") {
            screen.write_str(rect.y, rect.x, "Color Scheme:", Color::Black, Color::LightGray);
        }

        // Color scheme radio buttons
        if let Some(rect) = layout.get("scheme_blue") {
            let mark = if state.color_scheme == 0 { "o" } else { " " };
            let (fg, bg) = if current_field == 2 { (Color::White, Color::Black) } else { (Color::Black, Color::LightGray) };
            screen.write_str(rect.y, rect.x, &format!("  ({}) Classic Blue", mark), fg, bg);
        }
        if let Some(rect) = layout.get("scheme_dark") {
            let mark = if state.color_scheme == 1 { "o" } else { " " };
            let (fg, bg) = if current_field == 3 { (Color::White, Color::Black) } else { (Color::Black, Color::LightGray) };
            screen.write_str(rect.y, rect.x, &format!("  ({}) Dark", mark), fg, bg);
        }
        if let Some(rect) = layout.get("scheme_light") {
            let mark = if state.color_scheme == 2 { "o" } else { " " };
            let (fg, bg) = if current_field == 4 { (Color::White, Color::Black) } else { (Color::Black, Color::LightGray) };
            screen.write_str(rect.y, rect.x, &format!("  ({}) Light", mark), fg, bg);
        }

        // Buttons
        if let Some(rect) = layout.get("ok_button") {
            Self::draw_button(screen, rect.y, rect.x, "OK", current_field == 5);
        }
        if let Some(rect) = layout.get("cancel_button") {
            Self::draw_button(screen, rect.y, rect.x, "Cancel", current_field == 6);
        }

        // Cache layout for hit testing
        state.dialog_layout = Some(layout);
    }
}

/// Get help text for a topic
fn get_help_text(topic: &str) -> &'static str {
    match topic.to_uppercase().as_str() {
        "SURVIVAL GUIDE" => r#"
                    QBasic Survival Guide

 GETTING AROUND
   Use menus:     Press Alt, then highlighted letter
   Shortcut keys: Press F1 for Help on any item

 EDITING KEYS
   Home/End       Move to start/end of line
   Ctrl+Home/End  Move to start/end of program
   Ctrl+Y         Delete current line
   Ctrl+C/V/X     Copy, Paste, Cut selected text

 RUNNING PROGRAMS
   F5             Run program
   Shift+F5       Restart program
   F8             Step through code one line at a time
   F9             Set/clear breakpoint on current line
   Ctrl+Break     Stop running program

 FILE OPERATIONS
   Ctrl+N         New program
   Ctrl+O         Open existing file
   Ctrl+S         Save current file
   Alt+F, X       Exit QBasic

 GETTING HELP
   F1             Help on current word or menu item
   Shift+F1       Help Index

 Press Escape to close this dialog.
"#,
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
