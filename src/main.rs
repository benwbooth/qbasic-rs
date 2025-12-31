//! QBasic IDE Simulator
//!
//! A faithful simulation of the MS-DOS QBasic IDE written in Rust
//! using raw ANSI escape sequences (no external TUI libraries).

mod terminal;
mod screen;
mod input;
mod state;
mod ui;
mod basic;
mod help;

use std::io;
use terminal::{Terminal, Color};
use screen::Screen;
use input::InputEvent;
use state::{AppState, Focus, RunState, DialogType};
use ui::{MenuBar, Editor, StatusBar, ImmediateWindow, OutputWindow, Dialog, Rect, compute_layout, file_dialog_layout};
use ui::layout::main_screen_layout;
use ui::menubar::MenuAction;
use basic::{Lexer, Parser, Interpreter};

/// Main application
struct App {
    terminal: Terminal,
    screen: Screen,
    state: AppState,
    menubar: MenuBar,
    editor: Editor,
    immediate: ImmediateWindow,
    output: OutputWindow,
    interpreter: Interpreter,
    clipboard: Option<arboard::Clipboard>,
    /// Parsed program stored for resuming execution after NeedsInput
    current_program: Option<Vec<basic::parser::Stmt>>,
    /// Help system
    help: help::HelpSystem,
}

impl App {
    fn new() -> io::Result<Self> {
        let terminal = Terminal::new()?;
        let (width, height) = terminal.size();
        let screen = Screen::new(width, height);

        let mut help = help::HelpSystem::new();
        help.load_help_files();

        Ok(Self {
            terminal,
            screen,
            state: AppState::new(),
            menubar: MenuBar::new(),
            editor: Editor::new(),
            immediate: ImmediateWindow::new(),
            output: OutputWindow::new(),
            interpreter: Interpreter::new(),
            clipboard: arboard::Clipboard::new().ok(),
            current_program: None,
            help,
        })
    }

    fn run(&mut self) -> io::Result<()> {
        // Load a sample program
        self.editor.load(SAMPLE_PROGRAM);

        // Show welcome dialog on startup
        let (width, height) = self.screen.size();
        self.state.open_dialog_centered(DialogType::Welcome, width, height);

        loop {
            // Handle resize
            self.terminal.update_size();
            let (width, height) = self.terminal.size();
            if (width, height) != self.screen.size() {
                self.screen.resize(width, height);
                self.screen.invalidate();
                // Also resize graphics buffer if program is running
                if matches!(self.state.run_state, RunState::Running | RunState::WaitingForInput) {
                    self.interpreter.graphics.resize(width, height);
                }
            }

            // Draw
            self.draw();

            // Apply mouse cursor effect (orange box with inverted foreground)
            // But hide it when a BASIC program is running
            if !matches!(self.state.run_state, RunState::Running | RunState::WaitingForInput) {
                self.screen.apply_mouse_cursor(self.state.mouse_row, self.state.mouse_col);
            }

            // Flush to terminal
            self.screen.flush(&mut self.terminal)?;

            // Handle ALL available input events before next draw cycle
            // This ensures scroll events and other rapid inputs are processed smoothly
            let mut had_input = false;
            loop {
                let (maybe_key, raw_bytes) = self.terminal.read_key_raw()?;

                // If waiting for INKEY$ input, process key (or lack thereof) and continue execution
                if self.state.run_state == RunState::WaitingForInput {
                    if let Some(ref key) = maybe_key {
                        // Check for Ctrl+C or Ctrl+Break to stop program
                        if matches!(key, terminal::Key::Ctrl('c')) {
                            self.state.run_state = RunState::Finished;
                            self.state.set_status("Program stopped");
                            self.current_program = None;
                        } else if matches!(key, terminal::Key::Mouse(_)) {
                            // Ignore mouse events for INKEY$ - just continue execution
                            self.continue_after_input();
                        } else {
                            // Convert key to string for INKEY$
                            let key_str = if !raw_bytes.is_empty() {
                                // Use raw bytes for escape sequences (arrow keys, etc.)
                                String::from_utf8_lossy(&raw_bytes).to_string()
                            } else {
                                match key {
                                    terminal::Key::Char(c) => c.to_string(),
                                    terminal::Key::Enter => "\r".to_string(),
                                    terminal::Key::Escape => "\x1b".to_string(),
                                    terminal::Key::Tab => "\t".to_string(),
                                    _ => String::new(),
                                }
                            };

                            if !key_str.is_empty() {
                                self.interpreter.pending_key = Some(key_str);
                            }
                            // Continue execution (INKEY$ should return empty string if no key available)
                            self.continue_after_input();
                        }
                    } else {
                        // No key pressed - continue execution with empty INKEY$
                        self.continue_after_input();
                    }
                    break; // Only process one event when waiting for input
                } else if let Some(key) = maybe_key {
                    had_input = true;
                    // Normal input handling when not waiting for INKEY$
                    // Debug: show raw bytes in status bar
                    if !raw_bytes.is_empty() && raw_bytes[0] == 0x1b {
                        let hex: Vec<String> = raw_bytes.iter().map(|b| format!("{:02x}", b)).collect();
                        self.state.set_status(format!("Key: [{}]", hex.join(" ")));
                    }
                    let event = InputEvent::from(key);
                    if !self.handle_input(event) {
                        self.state.should_quit = true;
                        break;
                    }
                    // Continue reading more events if available
                } else {
                    // No more input available
                    break;
                }
            }

            if !had_input && !matches!(self.state.run_state, RunState::Running | RunState::WaitingForInput) {
                // No input this cycle and not running a program - sleep briefly to avoid 100% CPU
                std::thread::sleep(std::time::Duration::from_millis(10));
            }

            if self.state.should_quit {
                break;
            }
        }

        Ok(())
    }

    fn draw(&mut self) {
        let (width, height) = self.screen.size();

        // If output window is visible, draw it fullscreen
        if self.state.show_output {
            // Use graphics screen for running programs (uses LOCATE, COLOR, etc.)
            if matches!(self.state.run_state, RunState::Running | RunState::WaitingForInput | RunState::Finished) {
                self.output.draw_graphics_screen(&mut self.screen, &self.interpreter.graphics, &self.state);
            } else {
                self.output.draw_fullscreen(&mut self.screen, &self.state);
            }
            return;
        }

        // Compute main layout
        let main_layout_item = main_screen_layout(
            self.state.show_immediate,
            self.state.immediate_height,
            self.state.immediate_maximized,
            false, // output is never shown in split view anymore
            self.state.output_height,
        );
        let bounds = Rect::new(0, 0, width, height);
        let layout = compute_layout(&main_layout_item, bounds);

        // Store in state for mouse hit testing
        self.state.main_layout = Some(layout.clone());
        self.state.last_screen_size = (width, height);

        // Clear with blue background
        self.screen.clear_with(Color::Yellow, Color::Blue);

        // Draw menu bar using layout
        if let Some(menu_rect) = layout.get("menu_bar") {
            self.menubar.draw(&mut self.screen, &self.state, menu_rect);
        }

        // Draw editor using layout
        if let Some(editor_rect) = layout.get("editor") {
            self.editor.draw(&mut self.screen, &self.state, editor_rect);
        }

        // Draw immediate window if visible
        if self.state.show_immediate {
            if let Some(imm_rect) = layout.get("immediate") {
                self.immediate.draw(
                    &mut self.screen,
                    &self.state,
                    imm_rect,
                    self.state.focus == Focus::Immediate,
                );
            }
        }

        // Draw status bar using layout
        if let Some(status_rect) = layout.get("status_bar") {
            StatusBar::draw(
                &mut self.screen,
                &self.state,
                self.editor.cursor_line,
                self.editor.cursor_col,
                status_rect,
            );
        }

        // Draw menu dropdown (must be after editor so it appears on top)
        if self.state.menu_open {
            self.menubar.draw_dropdown(&mut self.screen, &self.state);
        }

        // Draw dialog if active
        if !matches!(self.state.dialog, DialogType::None) {
            if matches!(self.state.dialog, DialogType::Help(_)) {
                self.draw_help_dialog();
            } else {
                Dialog::draw(&mut self.screen, &mut self.state, width, height);
            }
        }

    }

    fn handle_input(&mut self, event: InputEvent) -> bool {
        // Debug: show raw bytes for unknown key sequences
        if let InputEvent::UnknownBytes(bytes) = &event {
            let hex: Vec<String> = bytes.iter().map(|b| format!("{:02x}", b)).collect();
            self.state.set_status(format!("Unknown key: [{}]", hex.join(" ")));
            return true;
        }

        // If output window is visible fullscreen
        if self.state.show_output {
            // While program is running, ignore input (could add INPUT support later)
            if self.state.run_state == RunState::Running {
                return true;
            }
            // Program finished - wait for a KEYBOARD key press to close output
            // Ignore mouse events
            if self.state.run_state == RunState::Finished {
                let is_keyboard = !matches!(event,
                    InputEvent::MouseClick { .. } |
                    InputEvent::MouseDrag { .. } |
                    InputEvent::MouseRelease { .. } |
                    InputEvent::ScrollUp { .. } |
                    InputEvent::ScrollDown { .. }
                );
                if is_keyboard {
                    self.state.show_output = false;
                    self.state.run_state = RunState::Editing;
                }
                return true;
            }
        }

        // Track mouse position for cursor rendering
        match &event {
            InputEvent::MouseClick { row, col } |
            InputEvent::MouseDrag { row, col } |
            InputEvent::MouseMove { row, col } |
            InputEvent::MouseRelease { row, col } |
            InputEvent::ScrollUp { row, col } |
            InputEvent::ScrollDown { row, col } |
            InputEvent::ScrollLeft { row, col } |
            InputEvent::ScrollRight { row, col } => {
                self.state.mouse_row = *row;
                self.state.mouse_col = *col;
            }
            _ => {}
        }

        // Handle mouse events
        if let InputEvent::MouseClick { row, col } = &event {
            return self.handle_mouse_click(*row, *col);
        }

        // Handle scroll wheel events
        if let InputEvent::ScrollUp { .. } = &event {
            if self.state.focus == Focus::Editor {
                self.editor.scroll_row = self.editor.scroll_row.saturating_sub(3);
            }
            return true;
        }
        if let InputEvent::ScrollDown { .. } = &event {
            if self.state.focus == Focus::Editor {
                self.editor.scroll_row += 3;
            }
            return true;
        }
        if let InputEvent::ScrollLeft { .. } = &event {
            if self.state.focus == Focus::Editor {
                self.editor.scroll_col = self.editor.scroll_col.saturating_sub(6);
            }
            return true;
        }
        if let InputEvent::ScrollRight { .. } = &event {
            if self.state.focus == Focus::Editor {
                self.editor.scroll_col += 6;
            }
            return true;
        }

        // Handle mouse release - stop dragging/resizing/selecting
        if let InputEvent::MouseRelease { .. } = &event {
            self.state.dialog_dragging = false;
            self.state.dialog_resizing = false;
            self.state.vscroll_dragging = false;
            self.state.hscroll_dragging = false;
            self.state.immediate_resize_dragging = false;
            self.editor.end_selection();
            return true;
        }

        // Handle mouse drag - move or resize dialog, or editor selection
        if let InputEvent::MouseDrag { row, col } = &event {
            let (screen_width, screen_height) = self.screen.size();

            // Handle resize
            if self.state.dialog_resizing && !matches!(self.state.dialog, DialogType::None) {
                let new_width = col.saturating_sub(self.state.dialog_x) + 1;
                let new_height = row.saturating_sub(self.state.dialog_y) + 1;

                // Minimum size and clamp to screen
                self.state.dialog_width = new_width.max(30).min(screen_width - self.state.dialog_x);
                self.state.dialog_height = new_height.max(10).min(screen_height - self.state.dialog_y);

                return true;
            }

            // Handle immediate window resize drag
            if self.state.immediate_resize_dragging {
                if let Some(layout) = &self.state.main_layout {
                    if let Some(imm_rect) = layout.get("immediate") {
                        // Calculate new height based on drag position
                        // Dragging up = bigger immediate window
                        let imm_bottom = imm_rect.y + imm_rect.height;
                        let new_height = imm_bottom.saturating_sub(*row);
                        // Clamp to reasonable bounds (3 to half screen)
                        let max_height = screen_height / 2;
                        self.state.immediate_height = new_height.max(3).min(max_height);
                    }
                }
                return true;
            }

            // Handle move
            if self.state.dialog_dragging && !matches!(self.state.dialog, DialogType::None) {
                let (offset_x, offset_y) = self.state.dialog_drag_offset;

                // Calculate new position, keeping dialog on screen
                let new_x = col.saturating_sub(offset_x);
                let new_y = row.saturating_sub(offset_y);

                // Clamp to screen bounds
                self.state.dialog_x = new_x.min(screen_width.saturating_sub(self.state.dialog_width));
                self.state.dialog_y = new_y.max(1).min(screen_height.saturating_sub(self.state.dialog_height));

                return true;
            }

            // Handle vertical scrollbar drag
            if self.state.vscroll_dragging {
                if let Some(layout) = &self.state.main_layout {
                    if let Some(editor_rect) = layout.get("editor") {
                        let editor_row = editor_rect.y + 1;
                        let editor_height = editor_rect.height;
                        let hscroll_row = editor_row + editor_height - 1;
                        let vscroll_start = editor_row + 1;
                        let vscroll_end = hscroll_row - 1;
                        let track_height = vscroll_end.saturating_sub(vscroll_start).saturating_sub(1) as usize;

                        if track_height > 1 {
                            let line_count = self.editor.buffer.line_count().max(1);
                            let max_scroll = line_count.saturating_sub(1);

                            // Map track position to scroll position (must match draw_scrollbars)
                            let track_pos = row.saturating_sub(vscroll_start + 1) as usize;
                            let new_scroll = if max_scroll > 0 {
                                (track_pos * max_scroll) / track_height.saturating_sub(1)
                            } else {
                                0
                            };
                            self.editor.scroll_row = new_scroll.min(max_scroll);
                        }
                        return true;
                    }
                }
            }

            // Handle horizontal scrollbar drag
            if self.state.hscroll_dragging {
                if let Some(layout) = &self.state.main_layout {
                    if let Some(editor_rect) = layout.get("editor") {
                        let editor_col = editor_rect.x + 1;
                        let editor_width = editor_rect.width;
                        let vscroll_col = editor_col + editor_width - 1;
                        let hscroll_start = editor_col + 1;
                        let hscroll_end = vscroll_col - 1;
                        let track_width = hscroll_end.saturating_sub(hscroll_start).saturating_sub(1) as usize;

                        if track_width > 1 {
                            let max_line_len = self.editor.buffer.max_line_length().max(1);
                            let max_scroll = max_line_len.saturating_sub(1);

                            // Map track position to scroll position (must match draw_scrollbars)
                            let track_pos = col.saturating_sub(hscroll_start + 1) as usize;
                            let new_scroll = if max_scroll > 0 {
                                (track_pos * max_scroll) / track_width.saturating_sub(1)
                            } else {
                                0
                            };
                            self.editor.scroll_col = new_scroll.min(max_scroll);
                        }
                        return true;
                    }
                }
            }

            // Handle editor selection drag
            if self.editor.is_selecting {
                let (width, height) = self.screen.size();
                let immediate_height = if self.state.show_immediate { self.state.immediate_height } else { 0 };
                let editor_row = 2u16;
                let editor_height = height.saturating_sub(2 + immediate_height + 1);

                // Check if drag is in or near editor content area
                if *row >= editor_row && *row < editor_row + editor_height && *col >= 1 && *col < width {
                    // Convert to editor coordinates
                    let editor_y = row.saturating_sub(editor_row + 1);
                    let editor_x = col.saturating_sub(2);

                    // Update cursor position
                    let target_line = self.editor.scroll_row + editor_y as usize;
                    let target_col = self.editor.scroll_col + editor_x as usize;

                    if target_line < self.editor.buffer.line_count() {
                        self.editor.cursor_line = target_line;
                        let line_len = self.editor.buffer.line(target_line).map(|l| l.len()).unwrap_or(0);
                        self.editor.cursor_col = target_col.min(line_len);
                    }

                    // Update selection based on click count and anchor
                    match (self.state.click_count, self.state.selection_anchor) {
                        (2, Some(anchor)) => {
                            // Double-click drag: extend by word
                            self.editor.extend_selection_by_word(anchor);
                        }
                        (3, Some(anchor)) => {
                            // Triple-click drag: extend by line
                            self.editor.extend_selection_by_line(anchor);
                        }
                        (4, Some(anchor)) => {
                            // Quadruple-click drag: extend by paragraph
                            self.editor.extend_selection_by_paragraph(anchor);
                        }
                        _ => {
                            // Single click drag: normal character-by-character selection
                            self.editor.update_selection();
                        }
                    }
                    return true;
                }
            }
        }

        // If dialog is open, route input directly to dialog handler
        if self.state.focus == Focus::Dialog {
            // Handle help dialog specially
            if matches!(self.state.dialog, DialogType::Help(_)) {
                return self.handle_help_dialog_input(&event);
            }

            // Check if this is a dialog that accepts input (text fields, lists, checkboxes)
            let is_input_dialog = matches!(
                self.state.dialog,
                DialogType::Find | DialogType::Replace | DialogType::GoToLine |
                DialogType::NewSub | DialogType::NewFunction | DialogType::FindLabel |
                DialogType::CommandArgs | DialogType::HelpPath | DialogType::DisplayOptions |
                DialogType::FileOpen | DialogType::FileSave | DialogType::FileSaveAs |
                DialogType::Print
            );

            if is_input_dialog {
                return self.handle_input_dialog(&event);
            }

            match event {
                InputEvent::Escape => {
                    self.state.close_dialog();
                }
                InputEvent::Enter => {
                    // Handle dialog-specific actions
                    match &self.state.dialog {
                        DialogType::Welcome => {
                            if self.state.dialog_button == 0 {
                                // Show Survival Guide
                                self.state.close_dialog();
                                self.open_dialog(DialogType::Help("Survival Guide".to_string()));
                            } else {
                                self.state.close_dialog();
                            }
                        }
                        _ => {
                            self.state.close_dialog();
                        }
                    }
                }
                InputEvent::Tab | InputEvent::CursorRight | InputEvent::CursorDown => {
                    if self.state.dialog_button_count > 0 {
                        self.state.dialog_button = (self.state.dialog_button + 1) % self.state.dialog_button_count;
                    }
                }
                InputEvent::CursorLeft | InputEvent::CursorUp => {
                    if self.state.dialog_button_count > 0 {
                        if self.state.dialog_button == 0 {
                            self.state.dialog_button = self.state.dialog_button_count - 1;
                        } else {
                            self.state.dialog_button -= 1;
                        }
                    }
                }
                _ => {}
            }
            return true;
        }

        // Global shortcuts (only when no dialog is open)
        match &event {
            InputEvent::AltX | InputEvent::CtrlQ => {
                self.state.should_quit = true;
                return true;
            }
            InputEvent::F10 => {
                if !self.state.menu_open {
                    self.state.open_menu();
                } else {
                    self.state.close_menu();
                }
                return true;
            }
            InputEvent::F1 => {
                self.open_dialog(DialogType::Help("General".to_string()));
                return true;
            }
            InputEvent::F4 => {
                // Toggle output window
                self.state.show_output = !self.state.show_output;
                return true;
            }
            InputEvent::F5 => {
                self.run_program();
                return true;
            }
            InputEvent::F2 => {
                self.show_subs_list();
                return true;
            }
            InputEvent::F6 => {
                self.state.toggle_focus();
                return true;
            }
            InputEvent::F3 => {
                self.repeat_find();
                return true;
            }
            // Search shortcuts
            InputEvent::CtrlF => {
                self.state.dialog_input_cursor = self.state.dialog_find_text.len();
                self.open_dialog(DialogType::Find);
                return true;
            }
            InputEvent::CtrlG => {
                self.state.dialog_goto_line.clear();
                self.state.dialog_input_cursor = 0;
                self.open_dialog(DialogType::GoToLine);
                return true;
            }
            // File shortcuts
            InputEvent::CtrlS => {
                self.save_file();
                return true;
            }
            InputEvent::CtrlO => {
                self.open_dialog(DialogType::FileOpen);
                return true;
            }
            InputEvent::CtrlN => {
                self.new_file();
                return true;
            }
            // Clipboard operations
            InputEvent::CtrlC => {
                self.clipboard_copy();
                return true;
            }
            InputEvent::CtrlX => {
                self.clipboard_cut();
                return true;
            }
            InputEvent::CtrlV => {
                self.clipboard_paste();
                return true;
            }
            // Undo/Redo
            InputEvent::CtrlZ => {
                if self.editor.undo() {
                    self.state.set_status("Undo");
                } else {
                    self.state.set_status("Nothing to undo");
                }
                return true;
            }
            InputEvent::CtrlY => {
                if self.editor.redo() {
                    self.state.set_status("Redo");
                } else {
                    self.state.set_status("Nothing to redo");
                }
                return true;
            }
            _ => {}
        }

        // Menu shortcuts open the menu
        if input::is_menu_trigger(&event) && !self.state.menu_open {
            if let Some(idx) = input::menu_index_from_alt(&event) {
                self.state.menu_index = idx;
            }
            self.state.open_menu();
            return true;
        }

        // Route to focused component
        match self.state.focus {
            Focus::Menu => {
                if let Some(action) = self.menubar.handle_input(&mut self.state, &event) {
                    match action {
                        MenuAction::Execute(menu_idx, item_idx) => {
                            self.handle_menu_action(menu_idx, item_idx);
                        }
                        MenuAction::Close | MenuAction::Navigate => {}
                    }
                }
            }
            Focus::Editor => {
                self.editor.handle_input(&event, &mut self.state);
            }
            Focus::Immediate => {
                if let Some(cmd) = self.immediate.handle_input(&event) {
                    self.execute_immediate(&cmd);
                }
                // Escape returns to editor
                if matches!(event, InputEvent::Escape) {
                    self.state.focus = Focus::Editor;
                }
            }
            Focus::Dialog => {
                // Dialog input is handled earlier in the function
            }
        }

        true
    }

    fn handle_menu_action(&mut self, menu_idx: usize, item_idx: usize) {
        match (menu_idx, item_idx) {
            // File menu
            (0, 0) => self.new_file(),
            (0, 1) => self.open_dialog(DialogType::FileOpen),
            (0, 2) => self.save_file(),
            (0, 3) => self.open_dialog(DialogType::FileSaveAs),
            (0, 5) => self.open_dialog(DialogType::Print),
            (0, 7) => self.state.should_quit = true,

            // Edit menu
            (1, 0) => { // Undo
                if self.editor.undo() {
                    self.state.set_status("Undo");
                }
            }
            (1, 2) => self.clipboard_cut(),
            (1, 3) => self.clipboard_copy(),
            (1, 4) => self.clipboard_paste(),
            (1, 5) => { // Clear - delete selection
                if self.editor.has_selection() {
                    self.editor.delete_selection();
                    self.state.set_modified(true);
                }
            }
            (1, 7) => { // New SUB
                self.state.dialog_find_text.clear();
                self.state.dialog_input_cursor = 0;
                self.open_dialog(DialogType::NewSub);
            }
            (1, 8) => { // New FUNCTION
                self.state.dialog_find_text.clear();
                self.state.dialog_input_cursor = 0;
                self.open_dialog(DialogType::NewFunction);
            }

            // View menu
            (2, 0) => { // SUBs (F2)
                self.show_subs_list();
            }
            (2, 1) => { // Next Statement
                if let Some(line) = self.state.current_line {
                    self.editor.go_to_line(line + 1);
                    self.state.set_status(format!("Next statement at line {}", line + 1));
                }
            }
            (2, 2) => { // Output screen (F4)
                self.state.show_output = !self.state.show_output;
                if self.state.show_output {
                    self.state.set_status("Output window shown");
                } else {
                    self.state.set_status("Output window hidden");
                }
            }
            (2, 4) => { // Included File
                self.state.set_status("No included files");
            }
            (2, 5) => { // Included Lines
                self.state.set_status("No included files");
            }

            // Search menu
            (3, 0) => self.open_dialog(DialogType::Find),
            (3, 1) => self.repeat_find(), // Repeat Last Find (F3)
            (3, 2) => self.open_dialog(DialogType::Replace),
            (3, 3) => { // Label
                self.state.dialog_find_text.clear();
                self.state.dialog_input_cursor = 0;
                self.open_dialog(DialogType::FindLabel);
            }

            // Run menu
            (4, 0) | (4, 2) => self.run_program(),
            (4, 1) => self.restart_program(),
            (4, 4) => { // Modify COMMAND$
                self.state.dialog_input_cursor = self.state.command_args.len();
                self.open_dialog(DialogType::CommandArgs);
            }

            // Debug menu
            (5, 0) => self.step_program(),
            (5, 1) => self.step_program(), // Procedure Step (same as step for now)
            (5, 3) => self.state.toggle_breakpoint(self.editor.cursor_line),
            (5, 4) => {
                self.state.breakpoints.clear();
                self.state.set_status("All breakpoints cleared");
            }
            (5, 6) => { // Set Next Statement
                if self.state.run_state == RunState::Paused {
                    // Set the next line to execute to the cursor position
                    self.state.current_line = Some(self.editor.cursor_line);
                    self.state.set_status(format!("Next statement set to line {}", self.editor.cursor_line + 1));
                } else {
                    self.state.set_status("Must be paused at breakpoint to set next statement");
                }
            }

            // Options menu
            (6, 0) => { // Display
                self.state.dialog_input_field = 0;
                self.state.dialog_input_cursor = self.state.tab_stops.to_string().len();
                self.open_dialog(DialogType::DisplayOptions);
            }
            (6, 1) => { // Help Path
                self.state.dialog_input_cursor = self.state.help_path.len();
                self.open_dialog(DialogType::HelpPath);
            }
            (6, 2) => { // Syntax Checking toggle
                self.state.syntax_checking = !self.state.syntax_checking;
                if self.state.syntax_checking {
                    self.state.set_status("Syntax checking enabled");
                    // Run syntax check immediately
                    self.check_syntax();
                } else {
                    self.state.set_status("Syntax checking disabled");
                    self.state.syntax_errors.clear();
                }
            }

            // Help menu
            (7, 0) => self.open_dialog(DialogType::Help("Index".to_string())),
            (7, 1) => self.open_dialog(DialogType::Help("Contents".to_string())),
            (7, 2) => { // Topic (F1)
                // Get word under cursor and show help for it
                self.show_help_for_word_under_cursor();
            }
            (7, 3) => self.open_dialog(DialogType::Help("Using Help".to_string())),
            (7, 5) => self.open_dialog(DialogType::About),

            _ => {}
        }
    }

    fn open_dialog(&mut self, dialog: DialogType) {
        // Initialize help system if opening a help dialog
        if let DialogType::Help(ref topic) = dialog {
            self.help.navigate_to(topic);
        }
        let (width, height) = self.screen.size();
        self.state.open_dialog_centered(dialog, width, height);
    }

    fn handle_help_dialog_input(&mut self, event: &InputEvent) -> bool {
        match event {
            InputEvent::Escape => {
                self.state.close_dialog();
            }
            InputEvent::Enter => {
                // Follow selected link
                if let Some(link) = self.help.selected_link().cloned() {
                    self.help.navigate_to(&link.target);
                }
            }
            InputEvent::Backspace => {
                // Go back in history
                if !self.help.go_back() {
                    self.state.close_dialog();
                }
            }
            InputEvent::Tab => {
                // Next link
                let count = self.help.link_count();
                if count > 0 {
                    self.help.selected_link = (self.help.selected_link + 1) % count;
                }
            }
            InputEvent::CursorUp => {
                if self.help.scroll > 0 {
                    self.help.scroll -= 1;
                }
            }
            InputEvent::CursorDown => {
                self.help.scroll += 1;
            }
            InputEvent::PageUp => {
                self.help.scroll = self.help.scroll.saturating_sub(10);
            }
            InputEvent::PageDown => {
                self.help.scroll += 10;
            }
            InputEvent::Home => {
                self.help.scroll = 0;
            }
            InputEvent::End => {
                // Scroll to end - we'll clamp in render
                self.help.scroll = usize::MAX / 2;
            }
            InputEvent::MouseClick { row, col, .. } => {
                if let Some(content_rect) = self.state.dialog_layout.as_ref().and_then(|l| l.get("content")) {
                    let content_height = content_rect.height as usize;
                    let content_width = (content_rect.width.saturating_sub(1)) as usize;
                    let scrollbar_col = content_rect.x + content_rect.width - 1;

                    // Check if click is on the scrollbar
                    if *col == scrollbar_col && *row >= content_rect.y && *row < content_rect.y + content_rect.height {
                        // Calculate scroll position from click position
                        let (lines, _) = self.help.render(content_width);
                        let max_scroll = lines.len().saturating_sub(content_height);
                        if max_scroll > 0 && content_height > 0 {
                            let click_offset = (*row - content_rect.y) as usize;
                            self.help.scroll = (click_offset * max_scroll) / content_height.saturating_sub(1).max(1);
                        }
                        return true;
                    }

                    // Check if click is on a link
                    if *row >= content_rect.y && *row < content_rect.y + content_rect.height {
                        let line_idx = self.help.scroll + (*row - content_rect.y) as usize;
                        let click_col = (*col - content_rect.x) as usize;

                        // Get links and check if click is on one
                        let (_, links) = self.help.render(content_width);
                        for link in &links {
                            if link.line == line_idx && click_col >= link.col_start && click_col < link.col_end {
                                self.help.navigate_to(&link.target);
                                return true;
                            }
                        }
                    }
                }
            }
            InputEvent::MouseDrag { row, col, .. } => {
                // Handle scrollbar dragging
                if let Some(content_rect) = self.state.dialog_layout.as_ref().and_then(|l| l.get("content")) {
                    let content_height = content_rect.height as usize;
                    let content_width = (content_rect.width.saturating_sub(1)) as usize;
                    let scrollbar_col = content_rect.x + content_rect.width - 1;

                    if *col == scrollbar_col && *row >= content_rect.y && *row < content_rect.y + content_rect.height {
                        let (lines, _) = self.help.render(content_width);
                        let max_scroll = lines.len().saturating_sub(content_height);
                        if max_scroll > 0 && content_height > 0 {
                            let click_offset = (*row - content_rect.y) as usize;
                            self.help.scroll = (click_offset * max_scroll) / content_height.saturating_sub(1).max(1);
                        }
                        return true;
                    }
                }
            }
            InputEvent::ScrollUp { .. } => {
                if self.help.scroll > 0 {
                    self.help.scroll -= 1;
                }
            }
            InputEvent::ScrollDown { .. } => {
                self.help.scroll += 1;
            }
            _ => {}
        }
        true
    }

    fn draw_help_dialog(&mut self) {
        let x = self.state.dialog_x;
        let y = self.state.dialog_y;
        let width = self.state.dialog_width;
        let height = self.state.dialog_height;

        // Compute layout
        let bounds = Rect::new(x, y, width, height);
        let layout = compute_layout(&ui::layout::help_dialog_layout(), bounds);

        // Draw shadow and background - black background, light gray text (QBasic help style)
        self.screen.draw_shadow(y, x, width, height);
        self.screen.fill(y, x, width, height, ' ', Color::LightGray, Color::Black);
        self.screen.draw_box(y, x, width, height, Color::LightGray, Color::Black);

        // Get title from help system
        let title = self.help.current_document()
            .map(|d| d.title.clone())
            .unwrap_or_else(|| "Help".to_string());

        // Draw title bar (cyan on black like QBasic)
        if let Some(rect) = layout.get("title_bar") {
            let title_str = format!(" {} ", title);
            let title_x = rect.x + (rect.width.saturating_sub(title_str.len() as u16)) / 2;
            self.screen.write_str(rect.y, title_x, &title_str, Color::Cyan, Color::Black);
        }

        // Get content area from layout
        if let Some(content_rect) = layout.get("content") {
            let content_width = (content_rect.width.saturating_sub(1)) as usize; // Leave room for scrollbar
            let content_height = content_rect.height as usize;

            // Get rendered content
            let (lines, links) = self.help.render(content_width);

            // Clamp scroll to valid range
            let max_scroll = lines.len().saturating_sub(content_height);
            if self.help.scroll > max_scroll {
                self.help.scroll = max_scroll;
            }
            let scroll = self.help.scroll;
            let selected_link = self.help.selected_link;

            // Draw each visible line
            for (i, line) in lines.iter().skip(scroll).take(content_height).enumerate() {
                let row = content_rect.y + i as u16;
                let col = content_rect.x;
                let line_idx = scroll + i;

                // Collect characters from the line for safe indexing
                let chars: Vec<char> = line.chars().collect();

                // Find all links on this line
                let line_links: Vec<_> = links.iter().enumerate()
                    .filter(|(_, link)| link.line == line_idx)
                    .collect();

                // Draw character by character
                for (char_pos, ch) in chars.iter().enumerate() {
                    if char_pos >= content_width {
                        break;
                    }

                    // Check if this position is inside a link
                    let mut in_link = false;
                    let mut is_selected = false;

                    for (link_idx, link) in &line_links {
                        if char_pos >= link.col_start && char_pos < link.col_end {
                            in_link = true;
                            is_selected = *link_idx == selected_link;
                            break;
                        }
                    }

                    let (fg, bg) = if in_link {
                        if is_selected {
                            (Color::White, Color::Cyan) // Selected link
                        } else {
                            (Color::Green, Color::Black) // Normal link in green
                        }
                    } else {
                        (Color::LightGray, Color::Black) // Normal text
                    };

                    self.screen.set(row, col + char_pos as u16, *ch, fg, bg);
                }
            }

            // Draw vertical scrollbar
            let scrollbar_col = content_rect.x + content_rect.width - 1;
            if lines.len() > content_height && content_height > 0 {
                // Draw scrollbar track
                for i in 0..content_height {
                    let row = content_rect.y + i as u16;
                    self.screen.set(row, scrollbar_col, '░', Color::DarkGray, Color::Black);
                }

                // Calculate thumb position and size
                let thumb_size = ((content_height * content_height) / lines.len().max(1)).max(1).min(content_height);
                let thumb_pos = if max_scroll > 0 {
                    (scroll * (content_height - thumb_size)) / max_scroll
                } else {
                    0
                };

                // Draw thumb
                for i in 0..thumb_size {
                    let row = content_rect.y + (thumb_pos + i) as u16;
                    if row < content_rect.y + content_height as u16 {
                        self.screen.set(row, scrollbar_col, '█', Color::Cyan, Color::Black);
                    }
                }
            }
        }

        // Draw navigation bar
        if let Some(nav_rect) = layout.get("nav_bar") {
            let nav_hint = if self.help.link_count() > 0 {
                "Tab:Link  Enter:Follow  Backspace:Back  Esc:Close"
            } else {
                "Arrows:Scroll  Backspace:Back  Esc:Close"
            };
            self.screen.write_str(nav_rect.y, nav_rect.x, nav_hint, Color::Cyan, Color::Black);
        }

        // Cache layout for hit testing
        self.state.dialog_layout = Some(layout);
    }

    fn handle_file_dialog_ok(&mut self) {
        // Get the selected filename
        let filename = if !self.state.dialog_filename.is_empty() {
            self.state.dialog_filename.clone()
        } else if !self.state.dialog_files.is_empty() && self.state.dialog_file_index < self.state.dialog_files.len() {
            self.state.dialog_files[self.state.dialog_file_index].clone()
        } else {
            self.state.set_status("No file selected");
            return;
        };

        let full_path = self.state.dialog_path.join(&filename);

        match &self.state.dialog {
            DialogType::FileOpen => {
                // Load the file
                match std::fs::read_to_string(&full_path) {
                    Ok(content) => {
                        self.editor.load(&content);
                        self.state.file_path = Some(full_path);
                        self.state.modified = false;
                        self.state.set_status("File loaded");
                        self.state.close_dialog();
                    }
                    Err(e) => {
                        self.state.set_status(format!("Error loading file: {}", e));
                    }
                }
            }
            DialogType::FileSave | DialogType::FileSaveAs => {
                // Save the file
                let content = self.editor.content();
                match std::fs::write(&full_path, &content) {
                    Ok(()) => {
                        self.state.file_path = Some(full_path);
                        self.state.modified = false;
                        self.state.set_status("File saved");
                        self.state.close_dialog();
                    }
                    Err(e) => {
                        self.state.set_status(format!("Error saving file: {}", e));
                    }
                }
            }
            _ => {
                self.state.close_dialog();
            }
        }
    }

    fn new_file(&mut self) {
        if self.state.modified {
            self.open_dialog(DialogType::NewProgram);
        } else {
            self.editor.clear();
            self.state.file_path = None;
            self.state.modified = false;
        }
    }

    fn save_file(&mut self) {
        if self.state.file_path.is_none() {
            self.open_dialog(DialogType::FileSaveAs);
        } else {
            // Save to file
            if let Some(path) = &self.state.file_path {
                if let Err(e) = std::fs::write(path, self.editor.content()) {
                    self.state.set_status(format!("Error saving: {}", e));
                } else {
                    self.state.modified = false;
                    self.state.set_status("Saved");
                }
            }
        }
    }

    fn handle_mouse_click(&mut self, row: u16, col: u16) -> bool {
        let (_width, _height) = self.screen.size();

        // If a dialog is open, handle dialog clicks
        if !matches!(self.state.dialog, DialogType::None) {
            let dialog_x = self.state.dialog_x;
            let dialog_y = self.state.dialog_y;
            let dialog_width = self.state.dialog_width;
            let dialog_height = self.state.dialog_height;

            // For file dialogs, use layout for all hit testing
            if matches!(self.state.dialog, DialogType::FileOpen | DialogType::FileSave | DialogType::FileSaveAs) {
                let bounds = Rect::new(dialog_x, dialog_y, dialog_width, dialog_height);
                let layout = compute_layout(&file_dialog_layout(), bounds);

                if let Some(hit_id) = layout.hit_test(row, col) {
                    match hit_id.as_str() {
                        "close" => {
                            self.state.close_dialog();
                            return true;
                        }
                        "maximize" => {
                            let (screen_width, screen_height) = self.screen.size();
                            if let Some((x, y, w, h)) = self.state.dialog_saved_bounds {
                                self.state.dialog_x = x;
                                self.state.dialog_y = y;
                                self.state.dialog_width = w;
                                self.state.dialog_height = h;
                                self.state.dialog_saved_bounds = None;
                            } else {
                                self.state.dialog_saved_bounds = Some((
                                    dialog_x, dialog_y, dialog_width, dialog_height,
                                ));
                                self.state.dialog_x = 1;
                                self.state.dialog_y = 2;
                                self.state.dialog_width = screen_width - 2;
                                self.state.dialog_height = screen_height - 3;
                            }
                            return true;
                        }
                        "title_bar" => {
                            self.state.dialog_dragging = true;
                            self.state.dialog_drag_offset = (col - dialog_x, row - dialog_y);
                            return true;
                        }
                        "resize_handle" => {
                            self.state.dialog_resizing = true;
                            return true;
                        }
                        "files_list" => {
                            if let Some(list_rect) = layout.get("files_list") {
                                if row > list_rect.y && row < list_rect.y + list_rect.height - 1 {
                                    let file_idx = (row - list_rect.y - 1) as usize;
                                    let max_items = list_rect.height.saturating_sub(2) as usize;
                                    if file_idx < self.state.dialog_files.len().min(max_items) {
                                        let now = std::time::Instant::now();
                                        let elapsed = now.duration_since(self.state.last_click_time);
                                        let same_pos = self.state.last_click_pos == (row, col);
                                        let is_double_click = same_pos && elapsed.as_millis() < 500;

                                        self.state.dialog_file_index = file_idx;
                                        self.state.dialog_filename = self.state.dialog_files[file_idx].clone();
                                        self.state.last_click_time = now;
                                        self.state.last_click_pos = (row, col);

                                        if is_double_click {
                                            self.handle_file_dialog_ok();
                                        }
                                    }
                                }
                            }
                            return true;
                        }
                        "dirs_list" => {
                            if let Some(list_rect) = layout.get("dirs_list") {
                                if row > list_rect.y && row < list_rect.y + list_rect.height - 1 {
                                    let dir_idx = (row - list_rect.y - 1) as usize;
                                    let max_items = list_rect.height.saturating_sub(2) as usize;
                                    if dir_idx < self.state.dialog_dirs.len().min(max_items) {
                                        let now = std::time::Instant::now();
                                        let elapsed = now.duration_since(self.state.last_click_time);
                                        let same_pos = self.state.last_click_pos == (row, col);
                                        let is_double_click = same_pos && elapsed.as_millis() < 500;

                                        self.state.dialog_dir_index = dir_idx;
                                        self.state.last_click_time = now;
                                        self.state.last_click_pos = (row, col);

                                        if is_double_click {
                                            let dir_name = self.state.dialog_dirs[dir_idx].clone();
                                            self.state.navigate_to_dir(&dir_name);
                                        }
                                    }
                                }
                            }
                            return true;
                        }
                        "ok_button" => {
                            self.handle_file_dialog_ok();
                            return true;
                        }
                        "cancel_button" => {
                            self.state.close_dialog();
                            return true;
                        }
                        "help_button" => {
                            self.state.set_status("File dialog help");
                            return true;
                        }
                        _ => {
                            return true;
                        }
                    }
                }

                // Check if click is inside dialog bounds
                if row >= dialog_y && row < dialog_y + dialog_height
                   && col >= dialog_x && col < dialog_x + dialog_width {
                    return true;
                }

                // Click outside dialog - close it
                self.state.close_dialog();
                return true;
            }

            // Non-file dialogs: use cached dialog layout for hit testing
            // Check window controls on title bar
            if row == dialog_y {
                // Close button [X] (right side)
                if col >= dialog_x + dialog_width - 4 && col < dialog_x + dialog_width - 1 {
                    self.state.close_dialog();
                    return true;
                }
                // Maximize button (next to close)
                if col >= dialog_x + dialog_width - 8 && col < dialog_x + dialog_width - 5 {
                    let (screen_width, screen_height) = self.screen.size();
                    if let Some((x, y, w, h)) = self.state.dialog_saved_bounds {
                        self.state.dialog_x = x;
                        self.state.dialog_y = y;
                        self.state.dialog_width = w;
                        self.state.dialog_height = h;
                        self.state.dialog_saved_bounds = None;
                    } else {
                        self.state.dialog_saved_bounds = Some((dialog_x, dialog_y, dialog_width, dialog_height));
                        self.state.dialog_x = 1;
                        self.state.dialog_y = 2;
                        self.state.dialog_width = screen_width - 2;
                        self.state.dialog_height = screen_height - 3;
                    }
                    return true;
                }
                // Title bar (for dragging) - everything else on title bar row
                if col >= dialog_x && col < dialog_x + dialog_width - 8 {
                    self.state.dialog_dragging = true;
                    self.state.dialog_drag_offset = (col - dialog_x, row - dialog_y);
                    return true;
                }
            }

            // Check resize handle (bottom-right corner)
            if row >= dialog_y + dialog_height - 2 && col >= dialog_x + dialog_width - 2 {
                self.state.dialog_resizing = true;
                return true;
            }

            // Use cached dialog layout for button hit testing
            let dialog_layout = self.state.dialog_layout.clone();
            if let Some(layout) = dialog_layout {
                if let Some(hit_id) = layout.hit_test(row, col) {
                    return self.handle_dialog_button_click(&hit_id);
                }
            }

            // Check if click is inside dialog bounds
            if row >= dialog_y && row < dialog_y + dialog_height
               && col >= dialog_x && col < dialog_x + dialog_width {
                // Click inside dialog but not on button - absorb it
                return true;
            }

            // Click outside dialog - close it
            self.state.close_dialog();
            return true;
        }

        // Use cached layout for hit testing (clone to avoid borrow issues)
        let layout = self.state.main_layout.clone();
        if let Some(layout) = layout {
            // Convert to 0-based for hit testing (layout uses 0-based coordinates)
            let hit_row = row.saturating_sub(1);
            let hit_col = col.saturating_sub(1);

            // If menu is open, check for clicks on menu items first
            if self.state.menu_open {
                let menu = &self.menubar.menus[self.state.menu_index];
                // Calculate menu position
                let mut menu_x = 1u16;
                for i in 0..self.state.menu_index {
                    menu_x += self.menubar.menus[i].title.len() as u16 + 3;
                }
                let menu_width = menu.width();
                let menu_height = menu.items.len() as u16 + 2;
                let menu_y = 2u16;

                // Check if click is inside dropdown (not on border)
                if row > menu_y && row < menu_y + menu_height - 1 && col > menu_x && col < menu_x + menu_width - 1 {
                    let item_idx = (row - menu_y - 1) as usize;
                    if item_idx < menu.items.len() && !menu.items[item_idx].separator {
                        let menu_idx = self.state.menu_index;
                        self.state.close_menu();
                        self.handle_menu_action(menu_idx, item_idx);
                    }
                    return true;
                }

                // Click on dropdown border - absorb but don't execute
                if row >= menu_y && row < menu_y + menu_height && col >= menu_x && col < menu_x + menu_width {
                    return true;
                }

                // Click outside menu - close it
                self.state.close_menu();
                // Fall through to handle click on underlying element
            }

            // Check which main area was clicked
            if let Some(hit_id) = layout.hit_test(hit_row, hit_col) {
                match hit_id.as_str() {
                    "menu_bar" => {
                        // Find which menu was clicked
                        let mut x = 2u16;
                        for (i, menu) in self.menubar.menus.iter().enumerate() {
                            let menu_end = x + menu.title.len() as u16 + 2;
                            if col >= x && col < menu_end {
                                self.state.menu_index = i;
                                self.state.menu_item = 0;
                                self.state.open_menu();
                                return true;
                            }
                            x = menu_end + 1;
                        }
                        // Clicked on menu bar but not on a menu - close if open
                        if self.state.menu_open {
                            self.state.close_menu();
                        }
                        return true;
                    }
                    "editor" => {
                        if let Some(editor_rect) = layout.get("editor") {
                            self.state.focus = Focus::Editor;
                            return self.handle_editor_click(row, col, editor_rect);
                        }
                    }
                    "output" => {
                        if let Some(out_rect) = layout.get("output") {
                            let out_row = out_rect.y + 1;
                            let out_col = out_rect.x + 1;
                            let out_width = out_rect.width;

                            // Check if click is on close button [X]
                            let close_x = out_col + out_width - 4;
                            if row == out_row && col >= close_x && col < close_x + 3 {
                                self.state.show_output = false;
                                return true;
                            }
                        }
                        return true;
                    }
                    "immediate" => {
                        if let Some(imm_rect) = layout.get("immediate") {
                            let imm_row = imm_rect.y + 1;
                            let imm_col = imm_rect.x + 1;
                            let imm_width = imm_rect.width;

                            // Check if click is on maximize/minimize button [↑] or [↓]
                            let button_x = imm_col + imm_width - 4;
                            if row == imm_row && col >= button_x && col < button_x + 3 {
                                self.state.immediate_maximized = !self.state.immediate_maximized;
                                return true;
                            }

                            // Check if click is on top border (for resize dragging)
                            if row == imm_row && !self.state.immediate_maximized {
                                self.state.immediate_resize_dragging = true;
                                return true;
                            }
                        }
                        self.state.focus = Focus::Immediate;
                        return true;
                    }
                    "status_bar" => {
                        // Click on status bar - absorb it
                        return true;
                    }
                    _ => {}
                }
            }
        }

        true
    }

    /// Handle dialog button clicks based on layout hit test result
    fn handle_dialog_button_click(&mut self, hit_id: &str) -> bool {
        let dialog_type = self.state.dialog.clone();
        match hit_id {
            // About dialog
            "ok_button" => {
                match dialog_type {
                    DialogType::About | DialogType::Message { .. } | DialogType::Help(_) => {
                        self.state.close_dialog();
                    }
                    DialogType::GoToLine => {
                        self.go_to_line();
                    }
                    DialogType::Find => {
                        self.find_and_verify();
                    }
                    DialogType::Print => {
                        self.state.set_status("Printing is not supported");
                        self.state.close_dialog();
                    }
                    _ => {}
                }
                return true;
            }
            "close_button" => {
                self.state.close_dialog();
                return true;
            }
            "cancel_button" => {
                self.state.close_dialog();
                return true;
            }
            "yes_button" => {
                if matches!(dialog_type, DialogType::NewProgram | DialogType::Confirm { .. }) {
                    self.save_file();
                    self.editor.clear();
                    self.state.file_path = None;
                    self.state.set_modified(false);
                    self.state.close_dialog();
                }
                return true;
            }
            "no_button" => {
                if matches!(dialog_type, DialogType::NewProgram | DialogType::Confirm { .. }) {
                    self.editor.clear();
                    self.state.file_path = None;
                    self.state.set_modified(false);
                    self.state.close_dialog();
                }
                return true;
            }
            "start_button" => {
                // Welcome dialog start button - show Survival Guide
                self.state.close_dialog();
                self.open_dialog(DialogType::Help("Survival Guide".to_string()));
                return true;
            }
            "exit_button" => {
                // Welcome dialog dismiss button - just close the dialog
                self.state.close_dialog();
                return true;
            }
            "find_next_button" => {
                self.find_and_verify();
                return true;
            }
            "replace_button" => {
                // Replace current occurrence
                if self.editor.has_selection() {
                    self.editor.replace_selection(&self.state.dialog_replace_text.clone());
                    self.state.set_modified(true);
                }
                return true;
            }
            "replace_all_button" => {
                // Replace all occurrences
                let search = self.state.dialog_find_text.clone();
                let replace = self.state.dialog_replace_text.clone();
                let count = self.editor.replace_all(
                    &search,
                    &replace,
                    self.state.search_case_sensitive,
                    self.state.search_whole_word,
                );
                self.state.set_status(format!("Replaced {} occurrences", count));
                if count > 0 {
                    self.state.set_modified(true);
                }
                return true;
            }
            _ => {
                // Any other hit inside dialog (spacer, content area, etc.) - absorb click
                return true;
            }
        }
    }

    /// Handle clicks within the editor area using layout-based hit testing
    fn handle_editor_click(&mut self, row: u16, col: u16, editor_rect: Rect) -> bool {
        // Convert editor rect to 1-based screen coordinates
        let editor_row = editor_rect.y + 1;
        let editor_col = editor_rect.x + 1;
        let editor_width = editor_rect.width;
        let editor_height = editor_rect.height;

        // Scrollbar positions (relative to editor area)
        let vscroll_col = editor_col + editor_width - 1;
        let hscroll_row = editor_row + editor_height - 1;

        // Vertical scrollbar (right edge of editor, inside the border)
        let vscroll_start = editor_row + 1;  // First row of scrollbar (up arrow)
        let vscroll_end = hscroll_row - 1;   // Last row of scrollbar (down arrow)

        // Debug: show click position and expected scrollbar positions
        self.state.set_status(format!(
            "click({},{}) vsb@c{} r{}-{} hsb@r{} c{}-{}",
            row, col, vscroll_col, vscroll_start, vscroll_end,
            hscroll_row, editor_col + 1, vscroll_col - 1
        ));

        if col == vscroll_col && row >= vscroll_start && row <= vscroll_end {
            // Up arrow (first row of scrollbar)
            if row == vscroll_start {
                self.editor.scroll_row = self.editor.scroll_row.saturating_sub(1);
                self.state.set_status(format!("VSCROLL UP: now at {}", self.editor.scroll_row));
                return true;
            }
            // Down arrow (last row of scrollbar)
            if row == vscroll_end {
                self.editor.scroll_row += 1;
                self.state.set_status(format!("VSCROLL DOWN: now at {}", self.editor.scroll_row));
                return true;
            }
            // Click on track (between arrows) - page up/down or drag thumb
            let track_height = vscroll_end.saturating_sub(vscroll_start).saturating_sub(1) as usize;
            if track_height > 1 {
                let page_size = self.editor.visible_lines.max(1);
                let line_count = self.editor.buffer.line_count().max(1);
                let max_scroll = line_count.saturating_sub(1);

                // Calculate current thumb position (must match draw_scrollbars)
                let thumb_pos = if line_count > 1 {
                    (self.editor.scroll_row.min(max_scroll) * track_height.saturating_sub(1)) / max_scroll
                } else {
                    0
                };
                let thumb_row = vscroll_start + 1 + thumb_pos as u16;

                if row == thumb_row {
                    // Click on thumb - start dragging
                    self.state.vscroll_dragging = true;
                } else if row < thumb_row {
                    // Click above thumb - page up
                    self.editor.scroll_row = self.editor.scroll_row.saturating_sub(page_size);
                    self.editor.cursor_line = self.editor.cursor_line.saturating_sub(page_size);
                    self.editor.clamp_cursor();
                } else {
                    // Click below thumb - page down
                    self.editor.scroll_row = (self.editor.scroll_row + page_size).min(max_scroll);
                    self.editor.cursor_line = (self.editor.cursor_line + page_size).min(max_scroll);
                    self.editor.clamp_cursor();
                }
                self.state.set_status(format!("VSCROLL: now at {}", self.editor.scroll_row));
            }
            return true;
        }

        // Horizontal scrollbar (bottom edge of editor, inside the border)
        let hscroll_start = editor_col + 1;  // First col of scrollbar (left arrow)
        let hscroll_end = vscroll_col - 1;   // Last col of scrollbar (right arrow)

        if row == hscroll_row && col >= hscroll_start && col <= hscroll_end {
            // Left arrow (first col of scrollbar)
            if col == hscroll_start {
                self.editor.scroll_col = self.editor.scroll_col.saturating_sub(1);
                self.state.set_status(format!("HSCROLL LEFT: now at {}", self.editor.scroll_col));
                return true;
            }
            // Right arrow (last col of scrollbar)
            if col == hscroll_end {
                self.editor.scroll_col += 1;
                self.state.set_status(format!("HSCROLL RIGHT: now at {}", self.editor.scroll_col));
                return true;
            }
            // Click on track (between arrows) - page left/right or drag thumb
            let track_width = hscroll_end.saturating_sub(hscroll_start).saturating_sub(1) as usize;
            if track_width > 1 {
                let page_size = self.editor.visible_cols.max(1);
                let max_line_len = self.editor.buffer.max_line_length().max(1);
                let max_scroll = max_line_len.saturating_sub(1);

                // Calculate current thumb position (must match draw_scrollbars)
                let thumb_pos = if max_line_len > 1 {
                    (self.editor.scroll_col.min(max_scroll) * track_width.saturating_sub(1)) / max_scroll
                } else {
                    0
                };
                let thumb_col = hscroll_start + 1 + thumb_pos as u16;

                if col == thumb_col {
                    // Click on thumb - start dragging
                    self.state.hscroll_dragging = true;
                } else if col < thumb_col {
                    // Click left of thumb - page left
                    self.editor.scroll_col = self.editor.scroll_col.saturating_sub(page_size);
                } else {
                    // Click right of thumb - page right
                    self.editor.scroll_col = (self.editor.scroll_col + page_size).min(max_scroll);
                }
                self.state.set_status(format!("HSCROLL: now at {}", self.editor.scroll_col));
            }
            return true;
        }

        // Click in editor content area (inside borders and scrollbars)
        let content_left = editor_col + 1;
        let content_right = vscroll_col - 1;
        let content_top = editor_row + 1;
        let content_bottom = hscroll_row - 1;

        if row >= content_top && row < content_bottom && col >= content_left && col < content_right {
            // Convert to editor coordinates
            let editor_y = row - content_top;
            let editor_x = col - content_left;

            // Set cursor position
            let target_line = self.editor.scroll_row + editor_y as usize;
            let target_col = self.editor.scroll_col + editor_x as usize;

            if target_line < self.editor.buffer.line_count() {
                self.editor.cursor_line = target_line;
                let line_len = self.editor.buffer.line(target_line).map(|l| l.len()).unwrap_or(0);
                self.editor.cursor_col = target_col.min(line_len);
            }

            // Multi-click detection
            let now = std::time::Instant::now();
            let elapsed = now.duration_since(self.state.last_click_time);
            let same_pos = self.state.last_click_pos == (row, col);
            let is_quick_click = elapsed.as_millis() < 400;

            if same_pos && is_quick_click {
                // Increment click count (max 4)
                self.state.click_count = (self.state.click_count + 1).min(4);
            } else {
                // Reset to single click
                self.state.click_count = 1;
            }

            self.state.last_click_time = now;
            self.state.last_click_pos = (row, col);

            // Handle based on click count
            match self.state.click_count {
                2 => {
                    // Double-click: select word
                    self.editor.select_word();
                    self.editor.is_selecting = true;
                    // Store anchor for drag extension
                    self.state.selection_anchor = self.editor.get_selection_bounds();
                }
                3 => {
                    // Triple-click: select line
                    self.editor.select_line();
                    self.editor.is_selecting = true;
                    // Store anchor for drag extension
                    self.state.selection_anchor = self.editor.get_selection_bounds();
                }
                4 => {
                    // Quadruple-click: select paragraph
                    self.editor.select_paragraph();
                    self.editor.is_selecting = true;
                    // Store anchor for drag extension
                    self.state.selection_anchor = self.editor.get_selection_bounds();
                }
                _ => {
                    // Single click: start selection
                    self.editor.start_selection();
                    self.state.selection_anchor = None;
                }
            }
            return true;
        }

        // Click on editor border or title bar - just absorb it
        true
    }

    fn run_program(&mut self) {
        self.state.run_state = RunState::Running;
        self.state.set_status("Running...");

        // Clear output window and show it
        self.output.clear();
        self.state.show_output = true;
        self.screen.invalidate(); // Force full redraw

        // Parse and execute
        let source = self.editor.content();
        let mut lexer = Lexer::new(&source);
        let tokens = lexer.tokenize();
        let mut parser = Parser::new(tokens);

        match parser.parse() {
            Ok(program) => {
                self.interpreter.reset();

                // Size graphics buffer to terminal size
                let (width, height) = self.terminal.size();
                self.interpreter.graphics.resize(width, height);
                self.interpreter.graphics.cls();

                // Pass breakpoints to interpreter
                let bp_lines: Vec<usize> = self.state.breakpoints
                    .iter()
                    .filter(|bp| bp.enabled)
                    .map(|bp| bp.line)
                    .collect();
                self.interpreter.set_breakpoints(&bp_lines);
                self.interpreter.set_step_mode(false);

                // Store program for potential resume after NeedsInput
                self.current_program = Some(program.clone());

                use crate::basic::interpreter::ExecutionResult;
                match self.interpreter.execute_with_debug(&program) {
                    Ok(ExecutionResult::Completed) => {
                        // Show output in output window (black bg, white text)
                        for line in self.interpreter.take_output() {
                            self.output.add_output(&line);
                        }
                        self.state.set_status("Program completed");
                        self.state.current_line = None;
                        self.state.run_state = RunState::Finished;
                        self.current_program = None;
                    }
                    Ok(ExecutionResult::Breakpoint(line)) => {
                        // Show output so far
                        for output_line in self.interpreter.take_output() {
                            self.output.add_output(&output_line);
                        }
                        self.state.current_line = Some(line);
                        self.state.run_state = RunState::Paused;
                        self.state.show_output = false; // Return to editor for breakpoint
                        self.state.set_status(format!("Breakpoint hit at line {}", line + 1));
                        self.editor.go_to_line(line + 1);
                    }
                    Ok(ExecutionResult::Stepped(line)) => {
                        self.state.current_line = Some(line);
                        self.state.run_state = RunState::Stepping;
                    }
                    Ok(ExecutionResult::NeedsInput) => {
                        // Show output so far
                        for output_line in self.interpreter.take_output() {
                            self.output.add_output(&output_line);
                        }
                        // Yield to allow UI update and keyboard input
                        self.state.run_state = RunState::WaitingForInput;
                    }
                    Err(e) => {
                        self.output.add_output(&format!("Runtime error: {}", e));
                        self.state.set_status(format!("Error: {}", e));
                        self.state.current_line = None;
                        self.state.run_state = RunState::Finished;
                        self.current_program = None;
                    }
                }
            }
            Err(e) => {
                self.output.add_output(&format!("Syntax error: {}", e));
                self.state.set_status(format!("Syntax error: {}", e));
                self.state.run_state = RunState::Finished;
            }
        }
    }

    fn restart_program(&mut self) {
        self.interpreter.reset();
        self.state.current_line = None;
        self.run_program();
    }

    fn step_program(&mut self) {
        let source = self.editor.content();
        let mut lexer = Lexer::new(&source);
        let tokens = lexer.tokenize();
        let mut parser = Parser::new(tokens);

        match parser.parse() {
            Ok(program) => {
                // If not already stepping, start fresh
                if self.state.run_state != RunState::Stepping && self.state.run_state != RunState::Paused {
                    self.interpreter.reset();
                }

                // Pass breakpoints to interpreter
                let bp_lines: Vec<usize> = self.state.breakpoints
                    .iter()
                    .filter(|bp| bp.enabled)
                    .map(|bp| bp.line)
                    .collect();
                self.interpreter.set_breakpoints(&bp_lines);
                self.interpreter.set_step_mode(true);

                // Store program for potential resume
                self.current_program = Some(program.clone());

                use crate::basic::interpreter::ExecutionResult;
                let result = if self.state.run_state == RunState::Stepping || self.state.run_state == RunState::Paused {
                    self.interpreter.continue_execution(&program)
                } else {
                    self.interpreter.execute_with_debug(&program)
                };

                match result {
                    Ok(ExecutionResult::Completed) => {
                        for line in self.interpreter.take_output() {
                            self.immediate.add_output(&line);
                        }
                        self.state.set_status("Program completed");
                        self.state.current_line = None;
                        self.state.run_state = RunState::Editing;
                        self.current_program = None;
                    }
                    Ok(ExecutionResult::Breakpoint(line)) | Ok(ExecutionResult::Stepped(line)) => {
                        for output_line in self.interpreter.take_output() {
                            self.immediate.add_output(&output_line);
                        }
                        self.state.current_line = Some(line);
                        self.state.run_state = RunState::Stepping;
                        self.state.set_status(format!("Step: line {} - F8 to continue", line + 1));
                        self.editor.go_to_line(line + 1);
                    }
                    Ok(ExecutionResult::NeedsInput) => {
                        for output_line in self.interpreter.take_output() {
                            self.immediate.add_output(&output_line);
                        }
                        self.state.run_state = RunState::WaitingForInput;
                    }
                    Err(e) => {
                        self.immediate.add_output(&format!("Runtime error: {}", e));
                        self.state.set_status(format!("Error: {}", e));
                        self.state.current_line = None;
                        self.state.run_state = RunState::Editing;
                        self.current_program = None;
                    }
                }
            }
            Err(e) => {
                self.immediate.add_output(&format!("Syntax error: {}", e));
                self.state.set_status(format!("Syntax error: {}", e));
            }
        }
    }

    /// Continue execution after receiving keyboard input for INKEY$
    fn continue_after_input(&mut self) {
        let program = match &self.current_program {
            Some(p) => p.clone(),
            None => return,
        };

        use crate::basic::interpreter::ExecutionResult;
        match self.interpreter.continue_execution(&program) {
            Ok(ExecutionResult::Completed) => {
                for line in self.interpreter.take_output() {
                    self.output.add_output(&line);
                }
                self.state.set_status("Program completed");
                self.state.current_line = None;
                self.state.run_state = RunState::Finished;
                self.current_program = None;
            }
            Ok(ExecutionResult::Breakpoint(line)) => {
                for output_line in self.interpreter.take_output() {
                    self.output.add_output(&output_line);
                }
                self.state.current_line = Some(line);
                self.state.run_state = RunState::Paused;
                self.state.show_output = false;
                self.state.set_status(format!("Breakpoint hit at line {}", line + 1));
                self.editor.go_to_line(line + 1);
            }
            Ok(ExecutionResult::Stepped(line)) => {
                self.state.current_line = Some(line);
                self.state.run_state = RunState::Stepping;
            }
            Ok(ExecutionResult::NeedsInput) => {
                for output_line in self.interpreter.take_output() {
                    self.output.add_output(&output_line);
                }
                self.state.run_state = RunState::WaitingForInput;
            }
            Err(e) => {
                self.output.add_output(&format!("Runtime error: {}", e));
                self.state.set_status(format!("Error: {}", e));
                self.state.current_line = None;
                self.state.run_state = RunState::Finished;
                self.current_program = None;
            }
        }
    }

    fn execute_immediate(&mut self, cmd: &str) {
        // Echo the command
        self.immediate.add_output(&format!("? {}", cmd));

        // Try to parse and execute as expression or statement
        let source = cmd.trim();
        let mut lexer = Lexer::new(source);
        let tokens = lexer.tokenize();
        let mut parser = Parser::new(tokens);

        // Try as expression first
        if let Ok(expr) = parser.parse_expression() {
            match self.interpreter.eval_expr(&expr) {
                Ok(value) => {
                    self.immediate.add_output(&value.to_string());
                }
                Err(e) => {
                    self.immediate.add_output(&format!("Error: {}", e));
                }
            }
        } else {
            // Try as statement
            let mut lexer2 = Lexer::new(source);
            let tokens2 = lexer2.tokenize();
            let mut parser2 = Parser::new(tokens2);

            match parser2.parse() {
                Ok(stmts) => {
                    if let Err(e) = self.interpreter.execute(&stmts) {
                        self.immediate.add_output(&format!("Error: {}", e));
                    } else {
                        for line in self.interpreter.take_output() {
                            self.immediate.add_output(&line);
                        }
                    }
                }
                Err(e) => {
                    self.immediate.add_output(&format!("Error: {}", e));
                }
            }
        }
    }

    fn clipboard_copy(&mut self) {
        if let Some(text) = self.editor.get_selected_text() {
            if let Some(ref mut clipboard) = self.clipboard {
                let _ = clipboard.set_text(&text);
                self.state.set_status("Copied to clipboard");
                // Exit keyboard select mode after copy
                self.editor.keyboard_select_mode = false;
            }
        }
    }

    fn clipboard_cut(&mut self) {
        if let Some(text) = self.editor.get_selected_text() {
            if let Some(ref mut clipboard) = self.clipboard {
                if clipboard.set_text(&text).is_ok() {
                    self.editor.delete_selection();
                    self.state.set_modified(true);
                    self.state.set_status("Cut to clipboard");
                }
            }
        }
    }

    fn clipboard_paste(&mut self) {
        if let Some(ref mut clipboard) = self.clipboard {
            if let Ok(text) = clipboard.get_text() {
                // Delete selection first if any
                if self.editor.has_selection() {
                    self.editor.delete_selection();
                }
                self.editor.insert_text(&text);
                self.state.set_modified(true);
                self.state.set_status("Pasted from clipboard");
            }
        }
    }

    /// Handle input for dialogs that accept text input, checkboxes, or list navigation
    fn handle_input_dialog(&mut self, event: &InputEvent) -> bool {
        match event {
            InputEvent::Escape => {
                self.state.close_dialog();
            }
            InputEvent::Enter => {
                self.execute_dialog_action();
            }
            InputEvent::Tab => {
                self.dialog_next_field();
            }
            InputEvent::ShiftTab => {
                self.dialog_prev_field();
            }
            InputEvent::Char(' ') => {
                // Space toggles checkboxes/radios or inserts space in text fields
                if !self.dialog_toggle_checkbox() {
                    // Not on a checkbox/radio, treat as text input
                    self.dialog_insert_char(' ');
                }
            }
            InputEvent::Char(c) => {
                self.dialog_insert_char(*c);
            }
            InputEvent::Backspace => {
                self.dialog_backspace();
            }
            InputEvent::Delete => {
                self.dialog_delete();
            }
            InputEvent::CursorLeft => {
                // In text field: move cursor left
                // In list: do nothing (or could scroll)
                if self.is_current_field_text() {
                    if self.state.dialog_input_cursor > 0 {
                        self.state.dialog_input_cursor -= 1;
                    }
                }
            }
            InputEvent::CursorRight => {
                // In text field: move cursor right
                if self.is_current_field_text() {
                    let max_len = self.get_current_dialog_text().len();
                    if self.state.dialog_input_cursor < max_len {
                        self.state.dialog_input_cursor += 1;
                    }
                }
            }
            InputEvent::Home => {
                if self.is_current_field_text() {
                    self.state.dialog_input_cursor = 0;
                }
            }
            InputEvent::End => {
                if self.is_current_field_text() {
                    self.state.dialog_input_cursor = self.get_current_dialog_text().len();
                }
            }
            InputEvent::CursorUp => {
                self.dialog_cursor_up();
            }
            InputEvent::CursorDown => {
                self.dialog_cursor_down();
            }
            _ => {}
        }
        true
    }

    /// Move to next field in dialog (Tab)
    fn dialog_next_field(&mut self) {
        let field_count = self.state.dialog_field_count;
        if field_count > 0 {
            self.state.dialog_input_field = (self.state.dialog_input_field + 1) % field_count;
            self.sync_dialog_button_from_field();
            self.position_cursor_for_field();
        }
    }

    /// Move to previous field in dialog (Shift+Tab)
    fn dialog_prev_field(&mut self) {
        let field_count = self.state.dialog_field_count;
        if field_count > 0 {
            self.state.dialog_input_field = (self.state.dialog_input_field + field_count - 1) % field_count;
            self.sync_dialog_button_from_field();
            self.position_cursor_for_field();
        }
    }

    /// Sync dialog_button from dialog_input_field for button fields
    fn sync_dialog_button_from_field(&mut self) {
        match &self.state.dialog {
            // FileOpen: filename(0), directory(1), files(2), dirs(3), OK(4), Cancel(5), Help(6)
            DialogType::FileOpen | DialogType::FileSave | DialogType::FileSaveAs => {
                self.state.dialog_button = match self.state.dialog_input_field {
                    4 => 0, // OK
                    5 => 1, // Cancel
                    6 => 2, // Help
                    _ => self.state.dialog_button,
                };
            }
            // Find: search(0), case(1), word(2), Find(3), Cancel(4), Help(5)
            DialogType::Find => {
                self.state.dialog_button = match self.state.dialog_input_field {
                    3 => 0, // Find
                    4 => 1, // Cancel
                    5 => 2, // Help
                    _ => self.state.dialog_button,
                };
            }
            // Replace: find(0), replace(1), case(2), word(3), FindNext(4), Replace(5), ReplaceAll(6), Cancel(7)
            DialogType::Replace => {
                self.state.dialog_button = match self.state.dialog_input_field {
                    4 => 0, // FindNext
                    5 => 1, // Replace
                    6 => 2, // ReplaceAll
                    7 => 3, // Cancel
                    _ => self.state.dialog_button,
                };
            }
            // GoToLine: line(0), OK(1), Cancel(2)
            DialogType::GoToLine => {
                self.state.dialog_button = match self.state.dialog_input_field {
                    1 => 0, // OK
                    2 => 1, // Cancel
                    _ => self.state.dialog_button,
                };
            }
            // Print: radio1(0), radio2(1), radio3(2), OK(3), Cancel(4)
            DialogType::Print => {
                self.state.dialog_button = match self.state.dialog_input_field {
                    3 => 0, // OK
                    4 => 1, // Cancel
                    _ => self.state.dialog_button,
                };
            }
            // Simple input dialogs: input(0), OK(1), Cancel(2)
            DialogType::NewSub | DialogType::NewFunction | DialogType::FindLabel |
            DialogType::CommandArgs | DialogType::HelpPath => {
                self.state.dialog_button = match self.state.dialog_input_field {
                    1 => 0, // OK
                    2 => 1, // Cancel
                    _ => self.state.dialog_button,
                };
            }
            // DisplayOptions: tabs(0), scrollbars(1), scheme1(2), scheme2(3), scheme3(4), OK(5), Cancel(6)
            DialogType::DisplayOptions => {
                self.state.dialog_button = match self.state.dialog_input_field {
                    5 => 0, // OK
                    6 => 1, // Cancel
                    _ => self.state.dialog_button,
                };
            }
            _ => {}
        }
    }

    /// Position cursor at end of current text field
    fn position_cursor_for_field(&mut self) {
        match &self.state.dialog {
            DialogType::FileOpen | DialogType::FileSave | DialogType::FileSaveAs => {
                match self.state.dialog_input_field {
                    0 => self.state.dialog_input_cursor = self.state.dialog_filename.len(),
                    1 => self.state.dialog_input_cursor = 0, // Directory field not editable
                    _ => {}
                }
            }
            DialogType::Find => {
                if self.state.dialog_input_field == 0 {
                    self.state.dialog_input_cursor = self.state.dialog_find_text.len();
                }
            }
            DialogType::Replace => {
                match self.state.dialog_input_field {
                    0 => self.state.dialog_input_cursor = self.state.dialog_find_text.len(),
                    1 => self.state.dialog_input_cursor = self.state.dialog_replace_text.len(),
                    _ => {}
                }
            }
            DialogType::GoToLine => {
                if self.state.dialog_input_field == 0 {
                    self.state.dialog_input_cursor = self.state.dialog_goto_line.len();
                }
            }
            DialogType::NewSub | DialogType::NewFunction | DialogType::FindLabel => {
                if self.state.dialog_input_field == 0 {
                    self.state.dialog_input_cursor = self.state.dialog_find_text.len();
                }
            }
            DialogType::CommandArgs => {
                if self.state.dialog_input_field == 0 {
                    self.state.dialog_input_cursor = self.state.command_args.len();
                }
            }
            DialogType::HelpPath => {
                if self.state.dialog_input_field == 0 {
                    self.state.dialog_input_cursor = self.state.help_path.len();
                }
            }
            DialogType::DisplayOptions => {
                if self.state.dialog_input_field == 0 {
                    self.state.dialog_input_cursor = self.state.tab_stops.to_string().len();
                }
            }
            _ => {}
        }
    }

    /// Check if current field is a text input field
    fn is_current_field_text(&self) -> bool {
        match &self.state.dialog {
            DialogType::FileOpen | DialogType::FileSave | DialogType::FileSaveAs => {
                self.state.dialog_input_field == 0 // filename field only
            }
            DialogType::Find => self.state.dialog_input_field == 0,
            DialogType::Replace => self.state.dialog_input_field <= 1,
            DialogType::GoToLine => self.state.dialog_input_field == 0,
            DialogType::NewSub | DialogType::NewFunction | DialogType::FindLabel |
            DialogType::CommandArgs | DialogType::HelpPath => self.state.dialog_input_field == 0,
            DialogType::DisplayOptions => self.state.dialog_input_field == 0,
            _ => false,
        }
    }

    /// Toggle checkbox or radio button, returns true if toggled
    fn dialog_toggle_checkbox(&mut self) -> bool {
        match &self.state.dialog {
            DialogType::Find => {
                match self.state.dialog_input_field {
                    1 => { self.state.search_case_sensitive = !self.state.search_case_sensitive; true }
                    2 => { self.state.search_whole_word = !self.state.search_whole_word; true }
                    _ => false,
                }
            }
            DialogType::Replace => {
                match self.state.dialog_input_field {
                    2 => { self.state.search_case_sensitive = !self.state.search_case_sensitive; true }
                    3 => { self.state.search_whole_word = !self.state.search_whole_word; true }
                    _ => false,
                }
            }
            DialogType::Print => {
                // Radio buttons: 0, 1, 2 are the print options
                match self.state.dialog_input_field {
                    0..=2 => {
                        // Store selected print option (could add state field for this)
                        true
                    }
                    _ => false,
                }
            }
            DialogType::DisplayOptions => {
                match self.state.dialog_input_field {
                    1 => { self.state.show_scrollbars = !self.state.show_scrollbars; true }
                    2 => { self.state.color_scheme = 0; true }
                    3 => { self.state.color_scheme = 1; true }
                    4 => { self.state.color_scheme = 2; true }
                    _ => false,
                }
            }
            _ => false,
        }
    }

    /// Handle cursor up in dialog
    fn dialog_cursor_up(&mut self) {
        match &self.state.dialog {
            DialogType::FileOpen | DialogType::FileSave | DialogType::FileSaveAs => {
                match self.state.dialog_input_field {
                    2 => {
                        // Files list - move selection up
                        if self.state.dialog_file_index > 0 {
                            self.state.dialog_file_index -= 1;
                        }
                    }
                    3 => {
                        // Dirs list - move selection up
                        if self.state.dialog_dir_index > 0 {
                            self.state.dialog_dir_index -= 1;
                        }
                    }
                    _ => {}
                }
            }
            DialogType::Replace => {
                // Move between find and replace fields
                if self.state.dialog_input_field == 1 {
                    self.state.dialog_input_field = 0;
                    self.state.dialog_input_cursor = self.state.dialog_find_text.len().min(self.state.dialog_input_cursor);
                }
            }
            _ => {}
        }
    }

    /// Handle cursor down in dialog
    fn dialog_cursor_down(&mut self) {
        match &self.state.dialog {
            DialogType::FileOpen | DialogType::FileSave | DialogType::FileSaveAs => {
                match self.state.dialog_input_field {
                    2 => {
                        // Files list - move selection down
                        if self.state.dialog_file_index + 1 < self.state.dialog_files.len() {
                            self.state.dialog_file_index += 1;
                        }
                    }
                    3 => {
                        // Dirs list - move selection down
                        if self.state.dialog_dir_index + 1 < self.state.dialog_dirs.len() {
                            self.state.dialog_dir_index += 1;
                        }
                    }
                    _ => {}
                }
            }
            DialogType::Replace => {
                // Move between find and replace fields
                if self.state.dialog_input_field == 0 {
                    self.state.dialog_input_field = 1;
                    self.state.dialog_input_cursor = self.state.dialog_replace_text.len().min(self.state.dialog_input_cursor);
                }
            }
            _ => {}
        }
    }

    /// Get the current dialog text being edited
    fn get_current_dialog_text(&self) -> &str {
        match &self.state.dialog {
            DialogType::FileOpen | DialogType::FileSave | DialogType::FileSaveAs => {
                if self.state.dialog_input_field == 0 {
                    &self.state.dialog_filename
                } else {
                    "" // Directory not editable
                }
            }
            DialogType::Find => &self.state.dialog_find_text,
            DialogType::Replace => {
                if self.state.dialog_input_field == 0 {
                    &self.state.dialog_find_text
                } else if self.state.dialog_input_field == 1 {
                    &self.state.dialog_replace_text
                } else {
                    ""
                }
            }
            DialogType::GoToLine => &self.state.dialog_goto_line,
            DialogType::NewSub | DialogType::NewFunction | DialogType::FindLabel => &self.state.dialog_find_text,
            DialogType::CommandArgs => &self.state.command_args,
            DialogType::HelpPath => &self.state.help_path,
            DialogType::DisplayOptions => {
                // Tab stops field - handled specially
                ""
            }
            _ => "",
        }
    }

    /// Insert a character into the current dialog input
    fn dialog_insert_char(&mut self, c: char) {
        // Only insert if we're on a text input field
        if !self.is_current_field_text() {
            return;
        }

        let cursor = self.state.dialog_input_cursor;
        match &self.state.dialog {
            DialogType::FileOpen | DialogType::FileSave | DialogType::FileSaveAs => {
                if self.state.dialog_input_field == 0 {
                    self.state.dialog_filename.insert(cursor, c);
                    self.state.dialog_input_cursor += 1;
                }
            }
            DialogType::Find => {
                if self.state.dialog_input_field == 0 {
                    self.state.dialog_find_text.insert(cursor, c);
                    self.state.dialog_input_cursor += 1;
                }
            }
            DialogType::Replace => {
                if self.state.dialog_input_field == 0 {
                    self.state.dialog_find_text.insert(cursor, c);
                    self.state.dialog_input_cursor += 1;
                } else if self.state.dialog_input_field == 1 {
                    self.state.dialog_replace_text.insert(cursor, c);
                    self.state.dialog_input_cursor += 1;
                }
            }
            DialogType::GoToLine => {
                // Only allow digits
                if self.state.dialog_input_field == 0 && c.is_ascii_digit() {
                    self.state.dialog_goto_line.insert(cursor, c);
                    self.state.dialog_input_cursor += 1;
                }
            }
            DialogType::NewSub | DialogType::NewFunction | DialogType::FindLabel => {
                if self.state.dialog_input_field == 0 {
                    self.state.dialog_find_text.insert(cursor, c);
                    self.state.dialog_input_cursor += 1;
                }
            }
            DialogType::CommandArgs => {
                if self.state.dialog_input_field == 0 {
                    self.state.command_args.insert(cursor, c);
                    self.state.dialog_input_cursor += 1;
                }
            }
            DialogType::HelpPath => {
                if self.state.dialog_input_field == 0 {
                    self.state.help_path.insert(cursor, c);
                    self.state.dialog_input_cursor += 1;
                }
            }
            DialogType::DisplayOptions => {
                // Only allow digits in tab stops field
                if self.state.dialog_input_field == 0 && c.is_ascii_digit() {
                    let mut tab_str = self.state.tab_stops.to_string();
                    if cursor <= tab_str.len() {
                        tab_str.insert(cursor, c);
                        if let Ok(n) = tab_str.parse::<usize>() {
                            if n > 0 && n <= 16 {
                                self.state.tab_stops = n;
                                self.state.dialog_input_cursor += 1;
                            }
                        }
                    }
                }
            }
            _ => {}
        }
    }

    /// Handle backspace in dialog input
    fn dialog_backspace(&mut self) {
        // Only backspace if we're on a text input field
        if !self.is_current_field_text() {
            return;
        }
        if self.state.dialog_input_cursor == 0 {
            return;
        }
        let cursor = self.state.dialog_input_cursor - 1;
        match &self.state.dialog {
            DialogType::FileOpen | DialogType::FileSave | DialogType::FileSaveAs => {
                if self.state.dialog_input_field == 0 && cursor < self.state.dialog_filename.len() {
                    self.state.dialog_filename.remove(cursor);
                    self.state.dialog_input_cursor = cursor;
                }
            }
            DialogType::Find => {
                if self.state.dialog_input_field == 0 && cursor < self.state.dialog_find_text.len() {
                    self.state.dialog_find_text.remove(cursor);
                    self.state.dialog_input_cursor = cursor;
                }
            }
            DialogType::Replace => {
                if self.state.dialog_input_field == 0 && cursor < self.state.dialog_find_text.len() {
                    self.state.dialog_find_text.remove(cursor);
                    self.state.dialog_input_cursor = cursor;
                } else if self.state.dialog_input_field == 1 && cursor < self.state.dialog_replace_text.len() {
                    self.state.dialog_replace_text.remove(cursor);
                    self.state.dialog_input_cursor = cursor;
                }
            }
            DialogType::GoToLine => {
                if self.state.dialog_input_field == 0 && cursor < self.state.dialog_goto_line.len() {
                    self.state.dialog_goto_line.remove(cursor);
                    self.state.dialog_input_cursor = cursor;
                }
            }
            DialogType::NewSub | DialogType::NewFunction | DialogType::FindLabel => {
                if self.state.dialog_input_field == 0 && cursor < self.state.dialog_find_text.len() {
                    self.state.dialog_find_text.remove(cursor);
                    self.state.dialog_input_cursor = cursor;
                }
            }
            DialogType::CommandArgs => {
                if self.state.dialog_input_field == 0 && cursor < self.state.command_args.len() {
                    self.state.command_args.remove(cursor);
                    self.state.dialog_input_cursor = cursor;
                }
            }
            DialogType::HelpPath => {
                if self.state.dialog_input_field == 0 && cursor < self.state.help_path.len() {
                    self.state.help_path.remove(cursor);
                    self.state.dialog_input_cursor = cursor;
                }
            }
            DialogType::DisplayOptions => {
                if self.state.dialog_input_field == 0 {
                    let mut tab_str = self.state.tab_stops.to_string();
                    if cursor < tab_str.len() {
                        tab_str.remove(cursor);
                        if let Ok(n) = tab_str.parse::<usize>() {
                            if n > 0 {
                                self.state.tab_stops = n;
                                self.state.dialog_input_cursor = cursor;
                            }
                        }
                    }
                }
            }
            _ => {}
        }
    }

    /// Handle delete in dialog input
    fn dialog_delete(&mut self) {
        // Only delete if we're on a text input field
        if !self.is_current_field_text() {
            return;
        }
        let cursor = self.state.dialog_input_cursor;
        match &self.state.dialog {
            DialogType::FileOpen | DialogType::FileSave | DialogType::FileSaveAs => {
                if self.state.dialog_input_field == 0 && cursor < self.state.dialog_filename.len() {
                    self.state.dialog_filename.remove(cursor);
                }
            }
            DialogType::Find => {
                if self.state.dialog_input_field == 0 && cursor < self.state.dialog_find_text.len() {
                    self.state.dialog_find_text.remove(cursor);
                }
            }
            DialogType::Replace => {
                if self.state.dialog_input_field == 0 && cursor < self.state.dialog_find_text.len() {
                    self.state.dialog_find_text.remove(cursor);
                } else if self.state.dialog_input_field == 1 && cursor < self.state.dialog_replace_text.len() {
                    self.state.dialog_replace_text.remove(cursor);
                }
            }
            DialogType::GoToLine => {
                if self.state.dialog_input_field == 0 && cursor < self.state.dialog_goto_line.len() {
                    self.state.dialog_goto_line.remove(cursor);
                }
            }
            DialogType::NewSub | DialogType::NewFunction | DialogType::FindLabel => {
                if self.state.dialog_input_field == 0 && cursor < self.state.dialog_find_text.len() {
                    self.state.dialog_find_text.remove(cursor);
                }
            }
            DialogType::CommandArgs => {
                if self.state.dialog_input_field == 0 && cursor < self.state.command_args.len() {
                    self.state.command_args.remove(cursor);
                }
            }
            DialogType::HelpPath => {
                if self.state.dialog_input_field == 0 && cursor < self.state.help_path.len() {
                    self.state.help_path.remove(cursor);
                }
            }
            DialogType::DisplayOptions => {
                if self.state.dialog_input_field == 0 {
                    let mut tab_str = self.state.tab_stops.to_string();
                    if cursor < tab_str.len() {
                        tab_str.remove(cursor);
                        if let Ok(n) = tab_str.parse::<usize>() {
                            if n > 0 {
                                self.state.tab_stops = n;
                            }
                        }
                    }
                }
            }
            _ => {}
        }
    }

    /// Execute the dialog action (when Enter is pressed)
    fn execute_dialog_action(&mut self) {
        match self.state.dialog.clone() {
            DialogType::Find => {
                match self.state.dialog_button {
                    0 => self.find_next(), // Find
                    1 => self.state.close_dialog(), // Cancel
                    2 => {} // Help
                    _ => {}
                }
            }
            DialogType::Replace => {
                match self.state.dialog_button {
                    0 => self.find_and_verify(), // Find & Verify
                    1 => self.replace_all(), // Change All
                    2 => self.state.close_dialog(), // Cancel
                    _ => {}
                }
            }
            DialogType::GoToLine => {
                match self.state.dialog_button {
                    0 => self.go_to_line(), // OK
                    1 => self.state.close_dialog(), // Cancel
                    _ => {}
                }
            }
            DialogType::NewSub => {
                match self.state.dialog_button {
                    0 => self.insert_new_sub(), // OK
                    1 => self.state.close_dialog(), // Cancel
                    _ => {}
                }
            }
            DialogType::NewFunction => {
                match self.state.dialog_button {
                    0 => self.insert_new_function(), // OK
                    1 => self.state.close_dialog(), // Cancel
                    _ => {}
                }
            }
            DialogType::FindLabel => {
                match self.state.dialog_button {
                    0 => self.find_label(), // OK
                    1 => self.state.close_dialog(), // Cancel
                    _ => {}
                }
            }
            DialogType::CommandArgs => {
                match self.state.dialog_button {
                    0 => {
                        // OK - save the command args (already in state)
                        self.state.set_status(format!("COMMAND$ set to: {}", self.state.command_args));
                        self.state.close_dialog();
                    }
                    1 => self.state.close_dialog(), // Cancel
                    _ => {}
                }
            }
            DialogType::HelpPath => {
                match self.state.dialog_button {
                    0 => {
                        // OK - save the help path (already in state)
                        self.state.set_status(format!("Help path set to: {}", self.state.help_path));
                        self.state.close_dialog();
                    }
                    1 => self.state.close_dialog(), // Cancel
                    _ => {}
                }
            }
            DialogType::DisplayOptions => {
                match self.state.dialog_button {
                    0 => {
                        // OK - settings are already modified in place
                        self.state.set_status("Display options saved");
                        self.state.close_dialog();
                    }
                    1 => self.state.close_dialog(), // Cancel
                    _ => {}
                }
            }
            _ => {}
        }
    }

    /// Find next occurrence
    fn find_next(&mut self) {
        let search = self.state.dialog_find_text.clone();
        if search.is_empty() {
            self.state.set_status("No search text");
            return;
        }

        self.state.last_search = search.clone();

        if let Some((line, col)) = self.editor.find_text(&search, self.state.search_case_sensitive, self.state.search_whole_word) {
            self.editor.go_to_and_select(line, col, search.len());
            self.state.set_status(format!("Found at line {}", line + 1));
            self.state.close_dialog();
        } else {
            self.state.set_status("Match not found");
        }
    }

    /// Find and verify (for replace dialog)
    fn find_and_verify(&mut self) {
        let search = self.state.dialog_find_text.clone();
        if search.is_empty() {
            self.state.set_status("No search text");
            return;
        }

        self.state.last_search = search.clone();

        if let Some((line, col)) = self.editor.find_text(&search, self.state.search_case_sensitive, self.state.search_whole_word) {
            self.editor.go_to_and_select(line, col, search.len());
            self.state.set_status("Found - press Enter to replace or find next");
            // Keep dialog open so user can replace or skip
        } else {
            self.state.set_status("Match not found");
        }
    }

    /// Replace all occurrences
    fn replace_all(&mut self) {
        let search = self.state.dialog_find_text.clone();
        let replace = self.state.dialog_replace_text.clone();

        if search.is_empty() {
            self.state.set_status("No search text");
            return;
        }

        let count = self.editor.replace_all(&search, &replace, self.state.search_case_sensitive, self.state.search_whole_word);

        if count > 0 {
            self.state.set_modified(true);
            self.state.set_status(format!("Replaced {} occurrence(s)", count));
        } else {
            self.state.set_status("No matches found");
        }

        self.state.close_dialog();
    }

    /// Go to specific line
    fn go_to_line(&mut self) {
        if let Ok(line_num) = self.state.dialog_goto_line.parse::<usize>() {
            self.editor.go_to_line(line_num);
            self.state.set_status(format!("Jumped to line {}", line_num));
        } else {
            self.state.set_status("Invalid line number");
        }
        self.state.close_dialog();
    }

    /// Insert a new SUB at end of file
    fn insert_new_sub(&mut self) {
        let name = self.state.dialog_find_text.trim().to_string();
        if name.is_empty() {
            self.state.set_status("SUB name cannot be empty");
            return;
        }

        // Insert SUB block at end of file
        let sub_block = format!("\n\nSUB {}\n    \nEND SUB", name);
        let line_count = self.editor.buffer.line_count();

        // Go to end of file and insert the SUB block
        self.editor.go_to_line(line_count);
        if let Some(line) = self.editor.buffer.line(self.editor.cursor_line) {
            self.editor.cursor_col = line.len();
        }
        self.editor.insert_text(&sub_block);

        // Move cursor to inside the SUB (the blank line)
        self.editor.go_to_line(line_count + 2);
        self.editor.cursor_col = 4;

        self.state.set_modified(true);
        self.state.set_status(format!("Created SUB {}", name));
        self.state.close_dialog();
    }

    /// Insert a new FUNCTION at end of file
    fn insert_new_function(&mut self) {
        let name = self.state.dialog_find_text.trim().to_string();
        if name.is_empty() {
            self.state.set_status("FUNCTION name cannot be empty");
            return;
        }

        // Insert FUNCTION block at end of file
        let func_block = format!("\n\nFUNCTION {}\n    {} = 0\nEND FUNCTION", name, name);
        let line_count = self.editor.buffer.line_count();

        // Go to end of file and insert the FUNCTION block
        self.editor.go_to_line(line_count);
        if let Some(line) = self.editor.buffer.line(self.editor.cursor_line) {
            self.editor.cursor_col = line.len();
        }
        self.editor.insert_text(&func_block);

        // Move cursor to inside the FUNCTION
        self.editor.go_to_line(line_count + 2);
        self.editor.cursor_col = 4;

        self.state.set_modified(true);
        self.state.set_status(format!("Created FUNCTION {}", name));
        self.state.close_dialog();
    }

    /// Find a label in the source
    fn find_label(&mut self) {
        let label = self.state.dialog_find_text.trim();
        if label.is_empty() {
            self.state.set_status("Label name cannot be empty");
            return;
        }

        // Search for label: (like "10:" or "MyLabel:")
        let label_pattern = format!("{}:", label);

        for (line_idx, line) in self.editor.buffer.lines.iter().enumerate() {
            let trimmed = line.trim_start();
            if trimmed.starts_with(&label_pattern) || trimmed == label_pattern.trim_end_matches(':') {
                self.editor.go_to_line(line_idx + 1);
                self.state.set_status(format!("Found label at line {}", line_idx + 1));
                self.state.close_dialog();
                return;
            }
        }

        self.state.set_status(format!("Label '{}' not found", label));
    }

    /// Check syntax of current buffer
    fn check_syntax(&mut self) {
        self.state.syntax_errors.clear();

        // Simple syntax checking - check for unclosed blocks
        let mut open_fors = 0;
        let mut open_ifs = 0;
        let mut open_whiles = 0;
        let mut open_subs = 0;
        let mut open_functions = 0;

        for (line_idx, line) in self.editor.buffer.lines.iter().enumerate() {
            let upper = line.to_uppercase();
            let trimmed = upper.trim();

            // Track block starts
            if trimmed.starts_with("FOR ") && !trimmed.contains(" NEXT") {
                open_fors += 1;
            }
            if (trimmed.starts_with("IF ") || trimmed == "IF") && !trimmed.contains(" THEN ") && !trimmed.ends_with(" THEN") {
                // Multi-line IF
            } else if trimmed.starts_with("IF ") && (trimmed.contains(" THEN") && !trimmed.chars().filter(|&c| c != ' ').skip_while(|&c| c != 'N').take(4).collect::<String>().contains("THEN")) {
                // This is getting complex - simplified version
            }
            if trimmed == "IF" || (trimmed.starts_with("IF ") && trimmed.ends_with("THEN")) {
                open_ifs += 1;
            }
            if trimmed.starts_with("WHILE ") || trimmed == "WHILE" {
                open_whiles += 1;
            }
            if trimmed.starts_with("SUB ") {
                open_subs += 1;
            }
            if trimmed.starts_with("FUNCTION ") {
                open_functions += 1;
            }

            // Track block ends
            if trimmed == "NEXT" || trimmed.starts_with("NEXT ") {
                if open_fors > 0 {
                    open_fors -= 1;
                } else {
                    self.state.syntax_errors.push((line_idx, "NEXT without FOR".to_string()));
                }
            }
            if trimmed == "END IF" || trimmed == "ENDIF" {
                if open_ifs > 0 {
                    open_ifs -= 1;
                } else {
                    self.state.syntax_errors.push((line_idx, "END IF without IF".to_string()));
                }
            }
            if trimmed == "WEND" {
                if open_whiles > 0 {
                    open_whiles -= 1;
                } else {
                    self.state.syntax_errors.push((line_idx, "WEND without WHILE".to_string()));
                }
            }
            if trimmed == "END SUB" {
                if open_subs > 0 {
                    open_subs -= 1;
                } else {
                    self.state.syntax_errors.push((line_idx, "END SUB without SUB".to_string()));
                }
            }
            if trimmed == "END FUNCTION" {
                if open_functions > 0 {
                    open_functions -= 1;
                } else {
                    self.state.syntax_errors.push((line_idx, "END FUNCTION without FUNCTION".to_string()));
                }
            }
        }

        // Check for unclosed blocks at end
        let last_line = self.editor.buffer.line_count().saturating_sub(1);
        if open_fors > 0 {
            self.state.syntax_errors.push((last_line, format!("{} unclosed FOR loop(s)", open_fors)));
        }
        if open_ifs > 0 {
            self.state.syntax_errors.push((last_line, format!("{} unclosed IF statement(s)", open_ifs)));
        }
        if open_whiles > 0 {
            self.state.syntax_errors.push((last_line, format!("{} unclosed WHILE loop(s)", open_whiles)));
        }
        if open_subs > 0 {
            self.state.syntax_errors.push((last_line, format!("{} unclosed SUB(s)", open_subs)));
        }
        if open_functions > 0 {
            self.state.syntax_errors.push((last_line, format!("{} unclosed FUNCTION(s)", open_functions)));
        }
    }

    /// Repeat last search (F3)
    fn repeat_find(&mut self) {
        let search = self.state.last_search.clone();
        if search.is_empty() {
            // Open Find dialog if no previous search
            self.open_dialog(DialogType::Find);
            return;
        }

        if let Some((line, col)) = self.editor.find_text(&search, self.state.search_case_sensitive, self.state.search_whole_word) {
            self.editor.go_to_and_select(line, col, search.len());
            self.state.set_status(format!("Found at line {}", line + 1));
        } else {
            self.state.set_status("Match not found");
        }
    }

    /// Show list of SUBs and FUNCTIONs (F2)
    fn show_subs_list(&mut self) {
        // Parse the source to find SUB and FUNCTION definitions
        let source = self.editor.content();
        let mut subs: Vec<String> = Vec::new();

        for (line_num, line) in source.lines().enumerate() {
            let trimmed = line.trim().to_uppercase();
            if trimmed.starts_with("SUB ") || trimmed.starts_with("FUNCTION ") {
                // Extract the name
                let parts: Vec<&str> = line.trim().split_whitespace().collect();
                if parts.len() >= 2 {
                    let name = parts[1].split('(').next().unwrap_or(parts[1]);
                    subs.push(format!("{}  (line {})", name, line_num + 1));
                }
            }
        }

        if subs.is_empty() {
            self.state.set_status("No SUBs or FUNCTIONs found");
        } else {
            // For now, just show in status. A proper implementation would show a dialog.
            self.state.set_status(format!("Found: {}", subs.join(", ")));
        }
    }

    /// Show help for word under cursor
    fn show_help_for_word_under_cursor(&mut self) {
        // Get the word at cursor position
        if let Some(line) = self.editor.buffer.line(self.editor.cursor_line) {
            let col = self.editor.cursor_col;

            // Find word boundaries
            let chars: Vec<char> = line.chars().collect();
            if col < chars.len() {
                let mut start = col;
                let mut end = col;

                // Find start of word
                while start > 0 && (chars[start - 1].is_alphanumeric() || chars[start - 1] == '$' || chars[start - 1] == '%') {
                    start -= 1;
                }

                // Find end of word
                while end < chars.len() && (chars[end].is_alphanumeric() || chars[end] == '$' || chars[end] == '%') {
                    end += 1;
                }

                if start < end {
                    let word: String = chars[start..end].iter().collect();
                    self.open_dialog(DialogType::Help(word.to_uppercase()));
                    return;
                }
            }
        }

        // Fallback: show general help
        self.open_dialog(DialogType::Help("General".to_string()));
    }
}

/// Sample program to show on startup
const SAMPLE_PROGRAM: &str = r#"' Welcome to QBasic!
' This is a demo program

CLS
PRINT "Hello, World!"
PRINT

' Simple loop
FOR i = 1 TO 5
    PRINT "Counting:"; i
NEXT i

PRINT
PRINT "Press F5 to run this program"
PRINT "Press F6 to switch to Immediate window"
PRINT "Press Alt+X to exit"
"#;

fn main() -> io::Result<()> {
    let mut app = App::new()?;
    app.run()
}
