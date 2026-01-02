//! File Open/Save dialog
#![allow(dead_code)]
//!
//! A self-contained dialog that owns its state including ListView widgets
//! for files and directories.

use std::path::PathBuf;
use crate::input::InputEvent;
use crate::screen::Screen;
use crate::terminal::Color;
use crate::ui::layout::{ComputedLayout, compute_layout, file_dialog_layout};
use crate::ui::listview::{ListView, ListViewColors};
use crate::ui::scrollbar::ScrollbarColors;
use crate::ui::widget::{Widget, EventResult};
use crate::ui::modal::{ModalDialog, ModalResult, ModalAction};
use crate::ui::floating_window::FloatingWindow;

/// Whether dialog is for opening or saving
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileDialogMode {
    Open,
    Save,
}

/// Result of file dialog interaction
#[derive(Debug, Clone, PartialEq)]
pub enum FileDialogResult {
    /// No action yet
    None,
    /// User confirmed selection
    Ok(PathBuf),
    /// User cancelled
    Cancel,
    /// Request help
    Help,
}

/// Which field is currently focused
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileDialogField {
    Filename = 0,
    Directory = 1,
    Files = 2,
    Dirs = 3,
    OkButton = 4,
    CancelButton = 5,
    HelpButton = 6,
}

impl FileDialogField {
    fn next(self) -> Self {
        match self {
            Self::Filename => Self::Directory,
            Self::Directory => Self::Files,
            Self::Files => Self::Dirs,
            Self::Dirs => Self::OkButton,
            Self::OkButton => Self::CancelButton,
            Self::CancelButton => Self::HelpButton,
            Self::HelpButton => Self::Filename,
        }
    }

    fn prev(self) -> Self {
        match self {
            Self::Filename => Self::HelpButton,
            Self::Directory => Self::Filename,
            Self::Files => Self::Directory,
            Self::Dirs => Self::Files,
            Self::OkButton => Self::Dirs,
            Self::CancelButton => Self::OkButton,
            Self::HelpButton => Self::CancelButton,
        }
    }
}

/// File Open/Save dialog
pub struct FileDialog {
    /// Floating window (handles chrome, drag, resize)
    window: FloatingWindow,
    /// Dialog mode (Open or Save)
    mode: FileDialogMode,
    /// Current directory path
    current_path: PathBuf,
    /// Filename input field
    filename: String,
    /// Cursor position in filename field
    filename_cursor: usize,
    /// Files list widget
    files_list: ListView,
    /// Directories list widget
    dirs_list: ListView,
    /// Currently focused field
    focused_field: FileDialogField,
    /// Screen size for maximize support
    screen_size: (u16, u16),
}

impl FileDialog {
    /// Create a new file open dialog
    pub fn open(initial_path: Option<PathBuf>) -> Self {
        Self::new_with_mode("Open Program", FileDialogMode::Open, initial_path)
    }

    /// Create a new file save dialog
    pub fn save(initial_path: Option<PathBuf>) -> Self {
        Self::new_with_mode("Save Program As", FileDialogMode::Save, initial_path)
    }

    /// Create a new file dialog with specified mode
    fn new_with_mode(title: impl Into<String>, mode: FileDialogMode, initial_path: Option<PathBuf>) -> Self {
        let path = initial_path
            .unwrap_or_else(|| std::env::current_dir().unwrap_or_default());

        // Create list colors (inverted selection: lightgray on black)
        let list_colors = ListViewColors {
            item_fg: Color::Black,
            item_bg: Color::LightGray,
            selected_fg: Color::LightGray,
            selected_bg: Color::Black,
            border_fg: Color::Black,
            border_bg: Color::LightGray,
        };

        let scrollbar_colors = ScrollbarColors::default();

        let files_list = ListView::new("files")
            .with_colors(list_colors)
            .with_scrollbar_colors(scrollbar_colors)
            .with_border(true);

        let dirs_list = ListView::new("dirs")
            .with_colors(list_colors)
            .with_scrollbar_colors(scrollbar_colors)
            .with_border(true);

        let window = FloatingWindow::new(title)
            .with_size(60, 18)
            .with_min_size(40, 12);

        let mut dialog = Self {
            window,
            mode,
            current_path: path,
            filename: String::new(),
            filename_cursor: 0,
            files_list,
            dirs_list,
            focused_field: FileDialogField::Filename,
            screen_size: (80, 25),
        };

        dialog.refresh_lists();
        dialog
    }

    /// Center the dialog on screen
    pub fn center(&mut self, screen_width: u16, screen_height: u16) {
        self.screen_size = (screen_width, screen_height);
        self.window.center(screen_width, screen_height);
    }

    /// Refresh the files and directories lists from current_path
    pub fn refresh_lists(&mut self) {
        let mut files = Vec::new();
        let mut dirs = vec!["..".to_string()]; // Parent directory

        if let Ok(entries) = std::fs::read_dir(&self.current_path) {
            for entry in entries.filter_map(|e| e.ok()) {
                if let Ok(name) = entry.file_name().into_string() {
                    if entry.path().is_dir() {
                        dirs.push(name);
                    } else if name.to_lowercase().ends_with(".bas") {
                        files.push(name);
                    }
                }
            }
        }

        files.sort();
        dirs.sort();

        self.files_list.set_items(files);
        self.dirs_list.set_items(dirs);
    }

    /// Navigate to a directory
    pub fn navigate_to(&mut self, dir_name: &str) {
        let new_path = if dir_name == ".." {
            self.current_path.parent()
                .map(|p| p.to_path_buf())
                .unwrap_or_else(|| self.current_path.clone())
        } else {
            self.current_path.join(dir_name)
        };

        if new_path.is_dir() {
            self.current_path = new_path;
            self.refresh_lists();
        }
    }

    /// Get the currently selected file path
    pub fn selected_file(&self) -> Option<PathBuf> {
        if !self.filename.is_empty() {
            Some(self.current_path.join(&self.filename))
        } else if let Some(name) = self.files_list.selected_item() {
            Some(self.current_path.join(name))
        } else {
            None
        }
    }

    /// Get current path
    pub fn current_path(&self) -> &PathBuf {
        &self.current_path
    }

    /// Get filename
    pub fn filename(&self) -> &str {
        &self.filename
    }

    /// Set filename
    pub fn set_filename(&mut self, name: impl Into<String>) {
        self.filename = name.into();
        self.filename_cursor = self.filename.len();
    }

    /// Draw the dialog
    pub fn draw(&self, screen: &mut Screen) {
        let bounds = self.window.bounds();
        let layout = compute_layout(&file_dialog_layout(), bounds);

        // Draw window chrome (shadow, border, title, resize handle)
        self.window.draw_chrome(screen);

        // Current directory display
        let cwd = self.current_path.to_string_lossy().to_string();

        // File name field
        if let Some(label_rect) = layout.get("filename_label") {
            let (fg, bg) = if self.focused_field == FileDialogField::Filename {
                (Color::White, Color::Black)
            } else {
                (Color::Black, Color::LightGray)
            };
            screen.write_str(label_rect.y, label_rect.x, "File Name:", fg, bg);
        }
        if let Some(field_rect) = layout.get("filename_field") {
            // Draw black border around input field
            screen.draw_box(field_rect.y - 1, field_rect.x - 1, field_rect.width + 2, 3, Color::Black, Color::LightGray);
            // Fill input field with light gray background
            screen.fill(field_rect.y, field_rect.x, field_rect.width, 1, ' ', Color::Black, Color::LightGray);
            let display = if self.filename.is_empty() { "*.BAS" } else { &self.filename };
            let truncated: String = display.chars().take(field_rect.width as usize).collect();
            screen.write_str(field_rect.y, field_rect.x, &truncated, Color::Black, Color::LightGray);

            // Draw cursor if focused
            if self.focused_field == FileDialogField::Filename {
                let cursor_x = field_rect.x + self.filename_cursor.min(field_rect.width as usize - 1) as u16;
                let cursor_char = self.filename.chars().nth(self.filename_cursor).unwrap_or(' ');
                screen.write_str(field_rect.y, cursor_x, &cursor_char.to_string(), Color::White, Color::Black);
            }
        }

        // Directory field (read-only)
        if let Some(label_rect) = layout.get("directory_label") {
            let (fg, bg) = if self.focused_field == FileDialogField::Directory {
                (Color::White, Color::Black)
            } else {
                (Color::Black, Color::LightGray)
            };
            screen.write_str(label_rect.y, label_rect.x, "Directory:", fg, bg);
        }
        if let Some(field_rect) = layout.get("directory_field") {
            // Directory field is read-only, use same colors as other fields
            screen.fill(field_rect.y, field_rect.x, field_rect.width, 1, ' ', Color::Black, Color::LightGray);
            let max_len = field_rect.width as usize;
            let dir_display = if cwd.len() > max_len { &cwd[cwd.len()-max_len..] } else { &cwd };
            screen.write_str(field_rect.y, field_rect.x, dir_display, Color::Black, Color::LightGray);
        }

        // Files label
        if let Some(rect) = layout.get("files_label") {
            let (fg, bg) = if self.focused_field == FileDialogField::Files {
                (Color::White, Color::Black)
            } else {
                (Color::Black, Color::LightGray)
            };
            screen.write_str(rect.y, rect.x, "Files:", fg, bg);
        }

        // Files list - use ListView widget
        if let Some(list_rect) = layout.get("files_list") {
            self.files_list.draw(screen, list_rect.clone());
        }

        // Dirs label
        if let Some(rect) = layout.get("dirs_label") {
            let (fg, bg) = if self.focused_field == FileDialogField::Dirs {
                (Color::White, Color::Black)
            } else {
                (Color::Black, Color::LightGray)
            };
            screen.write_str(rect.y, rect.x, "Dirs/Drives:", fg, bg);
        }

        // Dirs list - use ListView widget
        if let Some(list_rect) = layout.get("dirs_list") {
            self.dirs_list.draw(screen, list_rect.clone());
        }

        // Buttons
        if let Some(rect) = layout.get("ok_button") {
            Self::draw_button(screen, rect.y, rect.x, "OK", self.focused_field == FileDialogField::OkButton);
        }
        if let Some(rect) = layout.get("cancel_button") {
            Self::draw_button(screen, rect.y, rect.x, "Cancel", self.focused_field == FileDialogField::CancelButton);
        }
        if let Some(rect) = layout.get("help_button") {
            Self::draw_button(screen, rect.y, rect.x, "Help", self.focused_field == FileDialogField::HelpButton);
        }
    }

    fn draw_button(screen: &mut Screen, row: u16, col: u16, label: &str, selected: bool) {
        let fg = if selected { Color::White } else { Color::Black };
        let bg = if selected { Color::Black } else { Color::LightGray };
        let btn = format!("< {} >", label);
        screen.write_str(row, col, &btn, fg, bg);
    }

    /// Handle input event (internal implementation)
    fn process_event(&mut self, event: &InputEvent) -> FileDialogResult {
        // Let FloatingWindow handle drag/resize/maximize first
        let (sw, sh) = self.screen_size;
        if self.window.handle_event_with_screen(event, sw, sh) {
            return FileDialogResult::None;
        }

        // Compute layout for hit testing
        let bounds = self.window.bounds();
        let layout = compute_layout(&file_dialog_layout(), bounds);

        // Handle escape
        if matches!(event, InputEvent::Escape) {
            return FileDialogResult::Cancel;
        }

        // Handle Tab to cycle fields
        if matches!(event, InputEvent::Tab) {
            self.update_list_focus(false);
            self.focused_field = self.focused_field.next();
            self.update_list_focus(true);
            return FileDialogResult::None;
        }
        if matches!(event, InputEvent::ShiftTab) {
            self.update_list_focus(false);
            self.focused_field = self.focused_field.prev();
            self.update_list_focus(true);
            return FileDialogResult::None;
        }

        // Handle Enter
        if matches!(event, InputEvent::Enter) {
            return self.handle_enter();
        }

        // Route to focused field
        match self.focused_field {
            FileDialogField::Filename => {
                self.handle_filename_event(event);
            }
            FileDialogField::Files => {
                if let Some(list_rect) = layout.get("files_list") {
                    let result = self.files_list.handle_event(event, list_rect.clone());
                    if let EventResult::Action(action) = result {
                        if action == "files_activate" {
                            // Double-click or Enter on file
                            if let Some(name) = self.files_list.selected_item() {
                                self.filename = name.to_string();
                                return FileDialogResult::Ok(self.current_path.join(&self.filename));
                            }
                        } else if action == "files_select" {
                            // Single click - update filename
                            if let Some(name) = self.files_list.selected_item() {
                                self.filename = name.to_string();
                                self.filename_cursor = self.filename.len();
                            }
                        }
                    }
                }
            }
            FileDialogField::Dirs => {
                if let Some(list_rect) = layout.get("dirs_list") {
                    let result = self.dirs_list.handle_event(event, list_rect.clone());
                    if let EventResult::Action(action) = result {
                        if action == "dirs_activate" {
                            // Double-click or Enter on directory - get name first to avoid borrow issue
                            let dir_name = self.dirs_list.selected_item().map(|s| s.to_string());
                            if let Some(name) = dir_name {
                                self.navigate_to(&name);
                            }
                        }
                    }
                }
            }
            _ => {}
        }

        // Handle mouse clicks on any area
        if let InputEvent::MouseClick { row, col } = event {
            return self.handle_mouse_click(*row, *col, &layout);
        }

        // Handle scroll wheel anywhere
        if let InputEvent::ScrollUp { row, col } | InputEvent::ScrollDown { row, col } = event {
            self.handle_scroll(event, *row, *col, &layout);
        }

        FileDialogResult::None
    }

    fn update_list_focus(&mut self, focused: bool) {
        match self.focused_field {
            FileDialogField::Files => self.files_list.set_focus(focused),
            FileDialogField::Dirs => self.dirs_list.set_focus(focused),
            _ => {}
        }
    }

    fn handle_enter(&mut self) -> FileDialogResult {
        match self.focused_field {
            FileDialogField::Filename | FileDialogField::OkButton => {
                if let Some(path) = self.selected_file() {
                    return FileDialogResult::Ok(path);
                }
            }
            FileDialogField::Files => {
                if let Some(name) = self.files_list.selected_item() {
                    self.filename = name.to_string();
                    return FileDialogResult::Ok(self.current_path.join(&self.filename));
                }
            }
            FileDialogField::Dirs => {
                // Get name first to avoid borrow issue
                let dir_name = self.dirs_list.selected_item().map(|s| s.to_string());
                if let Some(name) = dir_name {
                    self.navigate_to(&name);
                }
            }
            FileDialogField::CancelButton => {
                return FileDialogResult::Cancel;
            }
            FileDialogField::HelpButton => {
                return FileDialogResult::Help;
            }
            _ => {}
        }
        FileDialogResult::None
    }

    fn handle_filename_event(&mut self, event: &InputEvent) {
        match event {
            InputEvent::Char(c) => {
                self.filename.insert(self.filename_cursor, *c);
                self.filename_cursor += 1;
            }
            InputEvent::Backspace => {
                if self.filename_cursor > 0 {
                    self.filename_cursor -= 1;
                    self.filename.remove(self.filename_cursor);
                }
            }
            InputEvent::Delete => {
                if self.filename_cursor < self.filename.len() {
                    self.filename.remove(self.filename_cursor);
                }
            }
            InputEvent::CursorLeft => {
                if self.filename_cursor > 0 {
                    self.filename_cursor -= 1;
                }
            }
            InputEvent::CursorRight => {
                if self.filename_cursor < self.filename.len() {
                    self.filename_cursor += 1;
                }
            }
            InputEvent::Home => {
                self.filename_cursor = 0;
            }
            InputEvent::End => {
                self.filename_cursor = self.filename.len();
            }
            _ => {}
        }
    }

    fn handle_mouse_click(&mut self, row: u16, col: u16, layout: &ComputedLayout) -> FileDialogResult {
        // Check files list
        if let Some(rect) = layout.get("files_list") {
            if rect.contains(row, col) {
                self.focused_field = FileDialogField::Files;
                self.update_list_focus(true);
                let result = self.files_list.handle_event(&InputEvent::MouseClick { row, col }, rect.clone());
                if let EventResult::Action(action) = result {
                    if action == "files_activate" {
                        if let Some(name) = self.files_list.selected_item() {
                            self.filename = name.to_string();
                            return FileDialogResult::Ok(self.current_path.join(&self.filename));
                        }
                    } else if action == "files_select" {
                        if let Some(name) = self.files_list.selected_item() {
                            self.filename = name.to_string();
                            self.filename_cursor = self.filename.len();
                        }
                    }
                }
                return FileDialogResult::None;
            }
        }

        // Check dirs list
        if let Some(rect) = layout.get("dirs_list") {
            if rect.contains(row, col) {
                self.focused_field = FileDialogField::Dirs;
                self.update_list_focus(true);
                let result = self.dirs_list.handle_event(&InputEvent::MouseClick { row, col }, rect.clone());
                if let EventResult::Action(action) = result {
                    if action == "dirs_activate" {
                        // Get name first to avoid borrow issue
                        let dir_name = self.dirs_list.selected_item().map(|s| s.to_string());
                        if let Some(name) = dir_name {
                            self.navigate_to(&name);
                        }
                    }
                }
                return FileDialogResult::None;
            }
        }

        // Check filename field
        if let Some(rect) = layout.get("filename_field") {
            if rect.contains(row, col) {
                self.focused_field = FileDialogField::Filename;
                return FileDialogResult::None;
            }
        }

        // Check buttons
        if let Some(rect) = layout.get("ok_button") {
            if rect.contains(row, col) {
                self.focused_field = FileDialogField::OkButton;
                if let Some(path) = self.selected_file() {
                    return FileDialogResult::Ok(path);
                }
            }
        }
        if let Some(rect) = layout.get("cancel_button") {
            if rect.contains(row, col) {
                return FileDialogResult::Cancel;
            }
        }
        if let Some(rect) = layout.get("help_button") {
            if rect.contains(row, col) {
                return FileDialogResult::Help;
            }
        }

        FileDialogResult::None
    }

    fn handle_scroll(&mut self, event: &InputEvent, row: u16, col: u16, layout: &ComputedLayout) {
        // Check if scroll is over files list
        if let Some(rect) = layout.get("files_list") {
            if rect.contains(row, col) {
                self.files_list.handle_event(event, rect.clone());
                return;
            }
        }

        // Check if scroll is over dirs list
        if let Some(rect) = layout.get("dirs_list") {
            if rect.contains(row, col) {
                self.dirs_list.handle_event(event, rect.clone());
            }
        }
    }
}

/// Implement ModalDialog trait for FileDialog
impl ModalDialog for FileDialog {
    fn draw(&self, screen: &mut Screen) {
        // Call the existing draw method
        FileDialog::draw(self, screen);
    }

    fn handle_event(&mut self, event: &InputEvent) -> ModalResult {
        // Call the internal event handler and convert the result
        match self.process_event(event) {
            FileDialogResult::None => ModalResult::Continue,
            FileDialogResult::Cancel => ModalResult::Close,
            FileDialogResult::Help => {
                // Return help action - caller can decide what to do
                ModalResult::Action(ModalAction::Help("file_dialog".to_string()))
            }
            FileDialogResult::Ok(path) => {
                // Return the appropriate action based on mode
                match self.mode {
                    FileDialogMode::Open => ModalResult::Action(ModalAction::FileOpen(path)),
                    FileDialogMode::Save => ModalResult::Action(ModalAction::FileSave(path)),
                }
            }
        }
    }

    fn title(&self) -> &str {
        &self.window.title
    }
}
