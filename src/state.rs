//! Application state management

use std::path::PathBuf;

/// Which window/component has focus
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Focus {
    Editor,
    Immediate,
    Menu,
    Dialog,
}

/// Application run state
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum RunState {
    Editing,
    Running,
    Paused,  // At breakpoint
    Stepping,
}

/// Active dialog type
#[derive(Clone, Debug)]
pub enum DialogType {
    None,
    FileOpen,
    FileSave,
    FileSaveAs,
    Find,
    Replace,
    GoToLine,
    Help(String),
    About,
    Message { title: String, text: String },
    Confirm { title: String, text: String },
    NewProgram,
    Print,
    Welcome,
}

/// Current editor mode
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum EditorMode {
    Insert,
    Overwrite,
}

/// Debug breakpoint
#[derive(Clone, Debug)]
pub struct Breakpoint {
    pub line: usize,
    pub enabled: bool,
}

/// Main application state
pub struct AppState {
    /// Currently focused component
    pub focus: Focus,

    /// Current run state
    pub run_state: RunState,

    /// Current file path (None if untitled)
    pub file_path: Option<PathBuf>,

    /// File modified flag
    pub modified: bool,

    /// Active dialog
    pub dialog: DialogType,

    // === Layout cache ===
    /// Main screen layout (menu_bar, editor, immediate, status_bar)
    pub main_layout: Option<crate::ui::layout::ComputedLayout>,

    /// Last screen size for detecting resize
    pub last_screen_size: (u16, u16),

    /// Selected button in dialog (0-indexed)
    pub dialog_button: usize,

    /// Number of buttons in current dialog
    pub dialog_button_count: usize,

    /// Menu bar open
    pub menu_open: bool,

    /// Currently selected menu (0-7)
    pub menu_index: usize,

    /// Currently selected menu item
    pub menu_item: usize,

    /// Editor insert/overwrite mode
    pub editor_mode: EditorMode,

    /// Show immediate window
    pub show_immediate: bool,

    /// Immediate window height (in lines)
    pub immediate_height: u16,

    /// Breakpoints
    pub breakpoints: Vec<Breakpoint>,

    /// Current execution line (when running/debugging)
    pub current_line: Option<usize>,

    /// Status message
    pub status_message: Option<String>,

    /// Should the application quit
    pub should_quit: bool,

    /// Last search string (for F3 repeat)
    pub last_search: String,

    /// Search case sensitive
    pub search_case_sensitive: bool,

    /// Search whole word
    pub search_whole_word: bool,

    /// Find dialog input text
    pub dialog_find_text: String,

    /// Replace dialog input text
    pub dialog_replace_text: String,

    /// Go to line input text
    pub dialog_goto_line: String,

    /// Which input field is focused in dialog (0=find, 1=replace)
    pub dialog_input_field: usize,

    /// Cursor position within dialog input field
    pub dialog_input_cursor: usize,

    /// Current path for file dialog
    pub dialog_path: std::path::PathBuf,

    /// Selected file index in file dialog
    pub dialog_file_index: usize,

    /// Selected dir index in file dialog
    pub dialog_dir_index: usize,

    /// Cached list of files for file dialog
    pub dialog_files: Vec<String>,

    /// Cached list of directories for file dialog
    pub dialog_dirs: Vec<String>,

    /// Dialog position (x, y) - top-left corner
    pub dialog_x: u16,
    pub dialog_y: u16,

    /// Dialog size (width, height)
    pub dialog_width: u16,
    pub dialog_height: u16,

    /// Whether dialog is being dragged
    pub dialog_dragging: bool,

    /// Drag offset from dialog corner
    pub dialog_drag_offset: (u16, u16),

    /// Whether dialog is being resized
    pub dialog_resizing: bool,

    /// Saved size for maximize/restore
    pub dialog_saved_bounds: Option<(u16, u16, u16, u16)>, // x, y, width, height

    /// Selected filename in file dialog
    pub dialog_filename: String,

    /// Last click time for double-click detection
    pub last_click_time: std::time::Instant,

    /// Last click position for double-click detection
    pub last_click_pos: (u16, u16),

    /// Computed dialog layout for hit testing
    pub dialog_layout: Option<crate::ui::layout::ComputedLayout>,

    /// Scrollbar dragging state
    pub vscroll_dragging: bool,
    pub hscroll_dragging: bool,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            focus: Focus::Editor,
            run_state: RunState::Editing,
            file_path: None,
            modified: false,
            dialog: DialogType::None,
            main_layout: None,
            last_screen_size: (0, 0),
            dialog_button: 0,
            dialog_button_count: 1,
            menu_open: false,
            menu_index: 0,
            menu_item: 0,
            editor_mode: EditorMode::Insert,
            show_immediate: true,
            immediate_height: 6,
            breakpoints: Vec::new(),
            current_line: None,
            status_message: None,
            should_quit: false,
            last_search: String::new(),
            search_case_sensitive: false,
            search_whole_word: false,
            dialog_find_text: String::new(),
            dialog_replace_text: String::new(),
            dialog_goto_line: String::new(),
            dialog_input_field: 0,
            dialog_input_cursor: 0,
            dialog_path: std::env::current_dir().unwrap_or_default(),
            dialog_file_index: 0,
            dialog_dir_index: 0,
            dialog_files: Vec::new(),
            dialog_dirs: Vec::new(),
            dialog_x: 0,
            dialog_y: 0,
            dialog_width: 0,
            dialog_height: 0,
            dialog_dragging: false,
            dialog_drag_offset: (0, 0),
            dialog_resizing: false,
            dialog_saved_bounds: None,
            dialog_filename: String::new(),
            last_click_time: std::time::Instant::now(),
            last_click_pos: (0, 0),
            dialog_layout: None,
            vscroll_dragging: false,
            hscroll_dragging: false,
        }
    }
}

impl AppState {
    pub fn new() -> Self {
        Self::default()
    }

    /// Get the display title (filename or "Untitled")
    pub fn title(&self) -> String {
        match &self.file_path {
            Some(path) => {
                let name = path.file_name()
                    .and_then(|s| s.to_str())
                    .unwrap_or("Untitled");
                if self.modified {
                    format!("{}*", name)
                } else {
                    name.to_string()
                }
            }
            None => {
                if self.modified {
                    "Untitled*".to_string()
                } else {
                    "Untitled".to_string()
                }
            }
        }
    }

    /// Mark the document as modified
    pub fn set_modified(&mut self, modified: bool) {
        self.modified = modified;
    }

    /// Open a dialog with screen dimensions for centering
    pub fn open_dialog(&mut self, dialog: DialogType) {
        self.open_dialog_centered(dialog, 80, 25); // Default size if not specified
    }

    /// Open a dialog centered on screen
    pub fn open_dialog_centered(&mut self, dialog: DialogType, screen_width: u16, screen_height: u16) {
        // Set button count and default size based on dialog type
        let (button_count, width, height) = match &dialog {
            DialogType::None => (0, 0, 0),
            DialogType::About => (1, 50, 12),
            DialogType::Message { .. } => (1, 50, 10),
            DialogType::Help(_) => (1, 70, 20),
            DialogType::Confirm { .. } => (3, 50, 10),
            DialogType::NewProgram => (3, 40, 8),
            DialogType::FileOpen | DialogType::FileSave | DialogType::FileSaveAs => (3, 60, 18),
            DialogType::Find => (3, 55, 10),
            DialogType::Replace => (3, 55, 12),
            DialogType::GoToLine => (2, 40, 7),
            DialogType::Print => (2, 50, 10),
            DialogType::Welcome => (2, 54, 12),
        };

        self.dialog_button_count = button_count;
        self.dialog_button = 0;
        self.dialog_width = width;
        self.dialog_height = height;
        self.dialog_x = (screen_width.saturating_sub(width)) / 2;
        self.dialog_y = (screen_height.saturating_sub(height)) / 2;
        self.dialog_dragging = false;

        // Initialize file dialog state
        if matches!(dialog, DialogType::FileOpen | DialogType::FileSave | DialogType::FileSaveAs) {
            self.dialog_path = std::env::current_dir().unwrap_or_default();
            self.dialog_file_index = 0;
            self.dialog_dir_index = 0;
            self.refresh_file_dialog();
        }

        self.dialog = dialog;
        self.focus = Focus::Dialog;
    }

    /// Refresh the file dialog's file and directory lists
    pub fn refresh_file_dialog(&mut self) {
        self.dialog_files.clear();
        self.dialog_dirs.clear();

        // Add parent directory
        self.dialog_dirs.push("..".to_string());

        if let Ok(entries) = std::fs::read_dir(&self.dialog_path) {
            for entry in entries.filter_map(|e| e.ok()) {
                if let Ok(name) = entry.file_name().into_string() {
                    if entry.path().is_dir() {
                        self.dialog_dirs.push(name);
                    } else if name.to_lowercase().ends_with(".bas") {
                        self.dialog_files.push(name);
                    }
                }
            }
        }

        self.dialog_files.sort();
        self.dialog_dirs.sort();
    }

    /// Navigate to a directory in the file dialog
    pub fn navigate_to_dir(&mut self, dir_name: &str) {
        let new_path = if dir_name == ".." {
            self.dialog_path.parent()
                .map(|p| p.to_path_buf())
                .unwrap_or_else(|| self.dialog_path.clone())
        } else {
            self.dialog_path.join(dir_name)
        };

        if new_path.is_dir() {
            self.dialog_path = new_path;
            self.dialog_file_index = 0;
            self.dialog_dir_index = 0;
            self.refresh_file_dialog();
        }
    }

    /// Close the current dialog
    pub fn close_dialog(&mut self) {
        self.dialog = DialogType::None;
        self.focus = Focus::Editor;
    }

    /// Toggle breakpoint on a line
    pub fn toggle_breakpoint(&mut self, line: usize) {
        if let Some(idx) = self.breakpoints.iter().position(|b| b.line == line) {
            self.breakpoints.remove(idx);
        } else {
            self.breakpoints.push(Breakpoint { line, enabled: true });
        }
    }

    /// Check if a line has a breakpoint
    pub fn has_breakpoint(&self, line: usize) -> bool {
        self.breakpoints.iter().any(|b| b.line == line && b.enabled)
    }

    /// Set status message
    pub fn set_status(&mut self, msg: impl Into<String>) {
        self.status_message = Some(msg.into());
    }

    /// Clear status message
    pub fn clear_status(&mut self) {
        self.status_message = None;
    }

    /// Open menu bar
    pub fn open_menu(&mut self) {
        self.menu_open = true;
        self.focus = Focus::Menu;
    }

    /// Close menu bar
    pub fn close_menu(&mut self) {
        self.menu_open = false;
        self.focus = Focus::Editor;
    }

    /// Toggle between editor and immediate window
    pub fn toggle_focus(&mut self) {
        if self.focus == Focus::Editor && self.show_immediate {
            self.focus = Focus::Immediate;
        } else {
            self.focus = Focus::Editor;
        }
    }
}
