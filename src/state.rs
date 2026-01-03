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

    // === Layout cache ===
    /// Main screen layout (menu_bar, editor, immediate, status_bar)
    pub main_layout: Option<crate::ui::layout::ComputedLayout>,

    /// Last screen size for detecting resize
    pub last_screen_size: (u16, u16),

    /// Menu bar open
    pub menu_open: bool,

    /// Currently selected menu (0-7)
    pub menu_index: usize,

    /// Currently selected menu item
    pub menu_item: usize,

    /// Editor insert/overwrite mode
    pub editor_mode: EditorMode,

    /// Editor pane maximized
    pub editor_maximized: bool,

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
            // no dialog tracked in AppState
            main_layout: None,
            last_screen_size: (0, 0),
            menu_open: false,
            menu_index: 0,
            menu_item: 0,
            editor_mode: EditorMode::Insert,
            editor_maximized: false,
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

    /// Set focus to dialog
    pub fn focus_dialog(&mut self) {
        self.focus = Focus::Dialog;
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
