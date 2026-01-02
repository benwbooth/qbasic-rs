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
    WaitingForInput,  // Program yielded for INKEY$ - waiting for keyboard input
    Paused,  // At breakpoint
    Stepping,
    Finished,  // Program completed, waiting for key press to return to editor
}

/// Active dialog type
#[derive(Clone, Debug)]
#[allow(dead_code)]
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
    NewSub,
    NewFunction,
    FindLabel,
    CommandArgs,
    HelpPath,
    DisplayOptions,
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

    /// Total number of focusable fields in dialog (inputs + checkboxes + buttons)
    pub dialog_field_count: usize,

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

    /// Immediate window maximized
    pub immediate_maximized: bool,

    /// Immediate window border being dragged for resize
    pub immediate_resize_dragging: bool,

    /// Show output window (for program execution)
    pub show_output: bool,

    /// Output window height (in lines)
    pub output_height: u16,

    /// Command line arguments for COMMAND$
    pub command_args: String,

    /// Help file path
    pub help_path: String,

    /// Syntax checking enabled
    pub syntax_checking: bool,

    /// Syntax errors (line number, error message)
    pub syntax_errors: Vec<(usize, String)>,

    /// Tab stop width
    pub tab_stops: usize,

    /// Show scrollbars
    pub show_scrollbars: bool,

    /// Color scheme (0=Classic Blue, 1=Dark, 2=Light)
    pub color_scheme: usize,

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

    /// Last click time for multi-click detection
    pub last_click_time: std::time::Instant,

    /// Last click position for multi-click detection
    pub last_click_pos: (u16, u16),

    /// Click count for multi-click detection (1=single, 2=double, 3=triple, 4=quadruple)
    pub click_count: u8,

    /// Anchor selection bounds for multi-click drag extension
    /// Stores ((start_line, start_col), (end_line, end_col)) of the initial selection
    pub selection_anchor: Option<((usize, usize), (usize, usize))>,

    /// Computed dialog layout for hit testing
    pub dialog_layout: Option<crate::ui::layout::ComputedLayout>,

    /// Scrollbar dragging state
    pub vscroll_dragging: bool,
    pub hscroll_dragging: bool,

    /// Mouse cursor position (for rendering orange box cursor)
    pub mouse_row: u16,
    pub mouse_col: u16,
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
            dialog_field_count: 1,
            menu_open: false,
            menu_index: 0,
            menu_item: 0,
            editor_mode: EditorMode::Insert,
            show_immediate: true,
            immediate_height: 6,
            immediate_maximized: false,
            immediate_resize_dragging: false,
            show_output: false,
            output_height: 10,
            command_args: String::new(),
            help_path: String::new(),
            syntax_checking: true,
            syntax_errors: Vec::new(),
            tab_stops: 8,
            show_scrollbars: true,
            color_scheme: 0,
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
            dialog_x: 0,
            dialog_y: 0,
            dialog_width: 0,
            dialog_height: 0,
            dialog_dragging: false,
            dialog_drag_offset: (0, 0),
            dialog_resizing: false,
            dialog_saved_bounds: None,
            last_click_time: std::time::Instant::now(),
            last_click_pos: (0, 0),
            click_count: 0,
            selection_anchor: None,
            dialog_layout: None,
            vscroll_dragging: false,
            hscroll_dragging: false,
            mouse_row: 0,
            mouse_col: 0,
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
    #[allow(dead_code)]
    pub fn open_dialog(&mut self, dialog: DialogType) {
        self.open_dialog_centered(dialog, 80, 25); // Default size if not specified
    }

    /// Open a dialog centered on screen
    pub fn open_dialog_centered(&mut self, dialog: DialogType, screen_width: u16, screen_height: u16) {
        // Set button count, field count, and default size based on dialog type
        // field_count = total focusable elements (inputs + checkboxes + buttons)
        let (button_count, field_count, width, height) = match &dialog {
            DialogType::None => (0, 0, 0, 0),
            DialogType::About => (1, 1, 50, 12),  // OK button only
            DialogType::Message { .. } => (1, 1, 50, 10),  // OK button only
            DialogType::Help(_) => (0, 0, 80, 25),  // Full screen help viewer
            DialogType::Confirm { .. } => (3, 3, 50, 10),  // Yes, No, Cancel
            DialogType::NewProgram => (3, 3, 40, 8),  // Yes, No, Cancel
            // FileOpen: filename(0), directory(1), files(2), dirs(3), OK(4), Cancel(5), Help(6)
            DialogType::FileOpen | DialogType::FileSave | DialogType::FileSaveAs => (3, 7, 60, 18),
            // Find: search(0), case(1), word(2), Find(3), Cancel(4), Help(5)
            DialogType::Find => (3, 6, 55, 10),
            // Replace: find(0), replace(1), case(2), word(3), FindNext(4), Replace(5), ReplaceAll(6), Cancel(7)
            DialogType::Replace => (4, 8, 55, 12),
            // GoToLine: line(0), OK(1), Cancel(2)
            DialogType::GoToLine => (2, 3, 40, 7),
            // Print: radio1(0), radio2(1), radio3(2), OK(3), Cancel(4)
            DialogType::Print => (2, 5, 50, 10),
            // Welcome: option1(0), option2(1)
            DialogType::Welcome => (2, 2, 54, 14),
            // Simple input dialogs: input(0), OK(1), Cancel(2)
            DialogType::NewSub => (2, 3, 45, 7),
            DialogType::NewFunction => (2, 3, 45, 7),
            DialogType::FindLabel => (2, 3, 45, 7),
            DialogType::CommandArgs => (2, 3, 55, 7),
            DialogType::HelpPath => (2, 3, 55, 7),
            // DisplayOptions: tabs(0), scrollbars(1), scheme1(2), scheme2(3), scheme3(4), OK(5), Cancel(6)
            DialogType::DisplayOptions => (2, 7, 50, 14),
        };

        self.dialog_button_count = button_count;
        self.dialog_field_count = field_count;
        self.dialog_button = 0;
        self.dialog_input_field = 0;
        self.dialog_width = width;
        self.dialog_height = height;
        self.dialog_x = (screen_width.saturating_sub(width)) / 2;
        self.dialog_y = (screen_height.saturating_sub(height)) / 2;
        self.dialog_dragging = false;

        self.dialog = dialog;
        self.focus = Focus::Dialog;
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
    #[allow(dead_code)]
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
