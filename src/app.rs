//! Main application module

use std::io;
use crate::terminal::{self, Terminal, Color};
use crate::screen::Screen;
use crate::input::{self, InputEvent};
use crate::state::{AppState, Focus, RunState, DialogType};
use crate::ui::{self, MenuBar, Editor, StatusBar, ImmediateWindow, OutputWindow, Dialog, Rect, compute_layout, tokenize_line, TokenKind, MenuClickResult, ImmediateClickResult, OutputClickResult, FileDialog, ModalDialog, ModalResult, ModalAction};
use crate::ui::scrollbar::{self, ScrollbarState, ScrollbarColors};
use crate::ui::dialog_manager::{DialogManager, DialogAction};
use crate::ui::editor_widgets::{handle_editor_click as widget_editor_click, EditorClickAction};
use crate::ui::scrollbar::ScrollAction;
use crate::ui::layout::main_screen_layout;
use crate::ui::menubar::MenuAction;
use crate::basic::{self, Lexer, Parser, Interpreter};
use crate::help;


/// Main application
pub struct App {
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
    /// Dialog widget manager
    dialog_manager: DialogManager,
    /// Modal dialog (captures all events when open)
    modal: Option<Box<dyn ModalDialog>>,
}

impl App {
    pub fn new() -> io::Result<Self> {
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
            dialog_manager: DialogManager::new(),
            modal: None,
        })
    }

    pub fn run(&mut self) -> io::Result<()> {
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
            // But hide it when a BASIC program is running or showing output
            if !self.state.show_output {
                self.screen.apply_mouse_cursor(self.state.mouse_row, self.state.mouse_col);
            }

            // Flush to terminal
            self.screen.flush(&mut self.terminal)?;

            // Handle ALL available input events before next draw cycle
            // This ensures scroll events and other rapid inputs are processed smoothly
            let mut had_input = false;
            loop {
                let (maybe_key, raw_bytes) = self.terminal.read_key_raw()?;

                // If waiting for input, handle differently depending on whether it's INPUT or INKEY$
                if self.state.run_state == RunState::WaitingForInput {
                    if self.interpreter.has_pending_input() {
                        // Waiting for INPUT statement - collect text input
                        if let Some(ref key) = maybe_key {
                            // Check for Ctrl+C or Ctrl+Break to stop program
                            if matches!(key, terminal::Key::Ctrl('c')) {
                                self.state.run_state = RunState::Finished;
                                self.state.set_status("Program stopped");
                                self.current_program = None;
                            } else if matches!(key, terminal::Key::Mouse(_)) {
                                // Ignore mouse events
                            } else {
                                match key {
                                    terminal::Key::Enter => {
                                        // Complete the INPUT and continue execution
                                        self.interpreter.complete_input();
                                        self.continue_after_input();
                                    }
                                    terminal::Key::Backspace => {
                                        self.interpreter.backspace_input();
                                    }
                                    terminal::Key::Char(c) => {
                                        self.interpreter.add_input_char(*c);
                                    }
                                    _ => {}
                                }
                            }
                        }
                        break; // Only process one event when waiting for INPUT
                    } else {
                        // Waiting for INKEY$ - process key (or lack thereof) and continue execution
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
                    }
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

        // Draw modal dialog if active (takes precedence)
        if let Some(ref modal) = self.modal {
            modal.draw(&mut self.screen);
        } else if !matches!(self.state.dialog, DialogType::None) {
            // Fall back to old dialog system for non-modal dialogs
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
            // Ignore all mouse events including movement
            if self.state.run_state == RunState::Finished {
                let is_keyboard = !matches!(event,
                    InputEvent::MouseClick { .. } |
                    InputEvent::MouseDrag { .. } |
                    InputEvent::MouseRelease { .. } |
                    InputEvent::MouseMove { .. } |
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

        // If help dialog is open, route scroll events and content-area clicks to it
        // But don't intercept if we're already dragging/resizing the dialog
        if self.state.focus == Focus::Dialog && matches!(self.state.dialog, DialogType::Help(_))
            && !self.state.dialog_dragging && !self.state.dialog_resizing
        {
            match &event {
                InputEvent::ScrollUp { .. } | InputEvent::ScrollDown { .. } => {
                    return self.handle_help_dialog_input(&event);
                }
                InputEvent::MouseClick { row, col } | InputEvent::MouseDrag { row, col } => {
                    // Route clicks in content, vscrollbar, or hscrollbar to help dialog handler
                    // Let title bar/resize clicks go to normal handler
                    if let Some(layout) = self.state.dialog_layout.as_ref() {
                        let in_content = layout.get("content").map_or(false, |r| r.contains(*row, *col));
                        let in_vscroll = layout.get("vscrollbar").map_or(false, |r| r.contains(*row, *col));
                        let in_hscroll = layout.get("hscrollbar").map_or(false, |r| r.contains(*row, *col));

                        if in_content || in_vscroll || in_hscroll {
                            return self.handle_help_dialog_input(&event);
                        }
                    }
                }
                _ => {}
            }
        }

        // If modal dialog is open, route all events to it
        if self.modal.is_some() {
            return self.handle_modal_event(&event);
        }

        // Handle mouse events
        if let InputEvent::MouseClick { row, col } = &event {
            return self.handle_mouse_click(*row, *col);
        }

        // Handle scroll wheel events - position-aware scrolling
        match &event {
            InputEvent::ScrollUp { row, col } | InputEvent::ScrollDown { row, col } |
            InputEvent::ScrollLeft { row, col } | InputEvent::ScrollRight { row, col } => {
                return self.handle_scroll_wheel(&event, *row, *col);
            }
            _ => {}
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

            // Handle editor selection drag using cached layout
            if self.editor.is_selecting {
                if let Some(ref layout) = self.state.main_layout {
                    if let Some(editor_rect) = layout.get("editor") {
                        // Content area is inside the border (offset by 2 for border + line numbers area)
                        let content_top = editor_rect.y + 2;
                        let content_left = editor_rect.x + 2;

                        // Check if drag is in editor content area
                        if *row >= content_top && *col >= content_left {
                            // Convert to editor coordinates
                            let editor_y = row.saturating_sub(content_top) as usize;
                            let editor_x = col.saturating_sub(content_left) as usize;

                            // Update cursor position
                            let target_line = self.editor.scroll_row + editor_y;
                            let target_col = self.editor.scroll_col + editor_x;

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
            }
        }

        // If dialog is open, route input directly to dialog handler
        if self.state.focus == Focus::Dialog {
            // Sync dialog manager with current state
            self.dialog_manager.sync_with_state(&self.state);

            // Handle help dialog specially
            if matches!(self.state.dialog, DialogType::Help(_)) {
                return self.handle_help_dialog_input(&event);
            }

            // Try widget-based dialog handling first for supported dialogs
            let dialog_action = self.dialog_manager.handle_event(&event, &self.state);

            // Sync widget state back to AppState
            self.dialog_manager.sync_to_state(&mut self.state);

            // Handle the action
            match dialog_action {
                DialogAction::Cancel => {
                    self.state.close_dialog();
                    self.dialog_manager.clear();
                    return true;
                }
                DialogAction::Find | DialogAction::FindNext => {
                    self.find_next();
                    return true;
                }
                DialogAction::Replace => {
                    // In Replace dialog, the "Replace" button does find & verify
                    self.find_and_verify();
                    return true;
                }
                DialogAction::ReplaceAll => {
                    self.replace_all();
                    return true;
                }
                DialogAction::GoToLine | DialogAction::Ok => {
                    self.execute_dialog_action();
                    return true;
                }
                DialogAction::ToggleCase => {
                    self.state.search_case_sensitive = !self.state.search_case_sensitive;
                    return true;
                }
                DialogAction::ToggleWholeWord => {
                    self.state.search_whole_word = !self.state.search_whole_word;
                    return true;
                }
                DialogAction::None => {
                    // Fall through to legacy handling
                }
                _ => {
                    // Other actions handled by legacy code
                }
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

        // Handle file dialogs specially - use the FileDialog widget
        match &dialog {
            DialogType::FileOpen => {
                let mut fd = FileDialog::open(None);
                fd.center(width, height);
                self.modal = Some(Box::new(fd));
                self.state.focus = Focus::Dialog;
                return;
            }
            DialogType::FileSave | DialogType::FileSaveAs => {
                let mut fd = FileDialog::save(None);
                fd.center(width, height);
                // Pre-fill filename if we have one
                if let Some(ref path) = self.state.file_path {
                    if let Some(name) = path.file_name() {
                        fd.set_filename(name.to_string_lossy().to_string());
                    }
                }
                self.modal = Some(Box::new(fd));
                self.state.focus = Focus::Dialog;
                return;
            }
            _ => {}
        }

        self.state.open_dialog_centered(dialog, width, height);
    }

    /// Handle events for modal dialogs
    fn handle_modal_event(&mut self, event: &InputEvent) -> bool {
        // Take the modal temporarily to avoid borrow issues
        let mut modal = match self.modal.take() {
            Some(m) => m,
            None => return true,
        };

        let result = modal.handle_event(event);

        match result {
            ModalResult::Continue => {
                // Put it back
                self.modal = Some(modal);
            }
            ModalResult::Close => {
                // Modal closed, focus returns to editor
                self.state.focus = Focus::Editor;
            }
            ModalResult::Action(action) => {
                // Handle the action
                self.handle_modal_action(action);
            }
        }

        true
    }

    /// Handle actions returned by modal dialogs
    fn handle_modal_action(&mut self, action: ModalAction) {
        match action {
            ModalAction::FileOpen(path) => {
                // Load the file
                match std::fs::read_to_string(&path) {
                    Ok(content) => {
                        self.editor.load(&content);
                        self.state.file_path = Some(path);
                        self.state.modified = false;
                        self.state.set_status("File loaded");
                    }
                    Err(e) => {
                        self.state.set_status(format!("Error loading file: {}", e));
                    }
                }
                self.state.focus = Focus::Editor;
            }
            ModalAction::FileSave(path) => {
                // Save the file
                let content = self.editor.content();
                match std::fs::write(&path, &content) {
                    Ok(()) => {
                        self.state.file_path = Some(path);
                        self.state.modified = false;
                        self.state.set_status("File saved");
                    }
                    Err(e) => {
                        self.state.set_status(format!("Error saving file: {}", e));
                    }
                }
                self.state.focus = Focus::Editor;
            }
            ModalAction::Help(topic) => {
                // Open help for the topic
                self.open_dialog(DialogType::Help(topic));
            }
            ModalAction::Confirm(confirmed) => {
                // Handle confirmation result
                if confirmed {
                    self.state.set_status("Confirmed");
                }
                self.state.focus = Focus::Editor;
            }
        }
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
            InputEvent::ShiftTab => {
                // Previous link
                let count = self.help.link_count();
                if count > 0 {
                    self.help.selected_link = if self.help.selected_link == 0 {
                        count - 1
                    } else {
                        self.help.selected_link - 1
                    };
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
            InputEvent::CursorLeft => {
                if self.help.scroll_col > 0 {
                    self.help.scroll_col -= 1;
                }
            }
            InputEvent::CursorRight => {
                self.help.scroll_col += 1;
            }
            InputEvent::PageUp => {
                self.help.scroll = self.help.scroll.saturating_sub(10);
            }
            InputEvent::PageDown => {
                self.help.scroll += 10;
            }
            InputEvent::Home => {
                self.help.scroll = 0;
                self.help.scroll_col = 0;
            }
            InputEvent::End => {
                // Scroll to end - we'll clamp in render
                self.help.scroll = usize::MAX / 2;
            }
            InputEvent::MouseClick { row, col, .. } | InputEvent::MouseDrag { row, col, .. } => {
                let layout = self.state.dialog_layout.clone();
                let is_drag = matches!(event, InputEvent::MouseDrag { .. });

                // If already dragging vertical scrollbar, just update position
                if self.state.vscroll_dragging {
                    if let Some(vscroll_rect) = layout.as_ref().and_then(|l| l.get("vscrollbar")) {
                        if let Some(content_rect) = layout.as_ref().and_then(|l| l.get("content")) {
                            let content_width = content_rect.width as usize;
                            let (lines, _, _, _) = self.help.render(content_width);
                            let vstate = ScrollbarState::new(self.help.scroll, lines.len(), 1);
                            let new_pos = scrollbar::drag_to_vscroll(
                                *row,
                                vscroll_rect.y,
                                vscroll_rect.y + vscroll_rect.height.saturating_sub(1),
                                &vstate,
                            );
                            self.help.scroll = new_pos;
                            return true;
                        }
                    }
                }

                // If already dragging horizontal scrollbar, just update position
                if self.state.hscroll_dragging {
                    if let Some(hscroll_rect) = layout.as_ref().and_then(|l| l.get("hscrollbar")) {
                        if let Some(content_rect) = layout.as_ref().and_then(|l| l.get("content")) {
                            let content_width = content_rect.width as usize;
                            let (_, _, _, max_width) = self.help.render(content_width);
                            let hstate = ScrollbarState::new(self.help.scroll_col, max_width, 1);
                            let new_pos = scrollbar::drag_to_hscroll(
                                *col,
                                hscroll_rect.x,
                                hscroll_rect.x + hscroll_rect.width.saturating_sub(1),
                                &hstate,
                            );
                            self.help.scroll_col = new_pos;
                            return true;
                        }
                    }
                }

                // Handle vertical scrollbar click
                if let Some(vscroll_rect) = layout.as_ref().and_then(|l| l.get("vscrollbar")) {
                    if vscroll_rect.contains(*row, *col) {
                        if let Some(content_rect) = layout.as_ref().and_then(|l| l.get("content")) {
                            let content_width = content_rect.width as usize;
                            let content_height = content_rect.height as usize;
                            let (lines, _, _, _) = self.help.render(content_width);
                            // Use visible_size=1 for scroll-past-end behavior
                            let vstate = ScrollbarState::new(self.help.scroll, lines.len(), 1);

                            let action = scrollbar::handle_vscroll_click(
                                *row,
                                vscroll_rect.y,
                                vscroll_rect.y + vscroll_rect.height.saturating_sub(1),
                                &vstate,
                                content_height,
                            );

                            match action {
                                scrollbar::ScrollAction::ScrollBack(n) => {
                                    self.help.scroll = self.help.scroll.saturating_sub(n);
                                }
                                scrollbar::ScrollAction::ScrollForward(n) => {
                                    self.help.scroll += n;
                                }
                                scrollbar::ScrollAction::PageBack => {
                                    if !is_drag {
                                        self.help.scroll = self.help.scroll.saturating_sub(content_height);
                                    }
                                }
                                scrollbar::ScrollAction::PageForward => {
                                    if !is_drag {
                                        self.help.scroll += content_height;
                                    }
                                }
                                scrollbar::ScrollAction::StartDrag | scrollbar::ScrollAction::SetPosition(_) => {
                                    // Start dragging - track state and position
                                    self.state.vscroll_dragging = true;
                                    let new_pos = scrollbar::drag_to_vscroll(
                                        *row,
                                        vscroll_rect.y,
                                        vscroll_rect.y + vscroll_rect.height.saturating_sub(1),
                                        &vstate,
                                    );
                                    self.help.scroll = new_pos;
                                }
                                scrollbar::ScrollAction::None => {}
                            }
                            return true;
                        }
                    }
                }

                // Handle horizontal scrollbar click
                if let Some(hscroll_rect) = layout.as_ref().and_then(|l| l.get("hscrollbar")) {
                    if hscroll_rect.contains(*row, *col) {
                        if let Some(content_rect) = layout.as_ref().and_then(|l| l.get("content")) {
                            let content_width = content_rect.width as usize;
                            let (_, _, _, max_width) = self.help.render(content_width);
                            // Use visible_size=1 for scroll-past-end behavior
                            let hstate = ScrollbarState::new(self.help.scroll_col, max_width, 1);

                            let action = scrollbar::handle_hscroll_click(
                                *col,
                                hscroll_rect.x,
                                hscroll_rect.x + hscroll_rect.width.saturating_sub(1),
                                &hstate,
                                content_width,
                            );

                            match action {
                                scrollbar::ScrollAction::ScrollBack(n) => {
                                    self.help.scroll_col = self.help.scroll_col.saturating_sub(n);
                                }
                                scrollbar::ScrollAction::ScrollForward(n) => {
                                    self.help.scroll_col += n;
                                }
                                scrollbar::ScrollAction::PageBack => {
                                    if !is_drag {
                                        self.help.scroll_col = self.help.scroll_col.saturating_sub(content_width);
                                    }
                                }
                                scrollbar::ScrollAction::PageForward => {
                                    if !is_drag {
                                        self.help.scroll_col += content_width;
                                    }
                                }
                                scrollbar::ScrollAction::StartDrag | scrollbar::ScrollAction::SetPosition(_) => {
                                    // Start dragging - track state and position
                                    self.state.hscroll_dragging = true;
                                    let new_pos = scrollbar::drag_to_hscroll(
                                        *col,
                                        hscroll_rect.x,
                                        hscroll_rect.x + hscroll_rect.width.saturating_sub(1),
                                        &hstate,
                                    );
                                    self.help.scroll_col = new_pos;
                                }
                                scrollbar::ScrollAction::None => {}
                            }
                            return true;
                        }
                    }
                }

                // Check if click is on a link (only for MouseClick, not drag)
                if matches!(event, InputEvent::MouseClick { .. }) {
                    if let Some(content_rect) = layout.as_ref().and_then(|l| l.get("content")) {
                        if content_rect.contains(*row, *col) {
                            let content_width = content_rect.width as usize;
                            let line_idx = self.help.scroll + (*row - content_rect.y) as usize;
                            let click_col = self.help.scroll_col + (*col - content_rect.x) as usize;

                            // Get links and check if click is on one
                            let (_, links, _, _) = self.help.render(content_width);

                            for link in &links {
                                if link.line == line_idx && click_col >= link.col_start && click_col < link.col_end {
                                    self.help.navigate_to(&link.target);
                                    return true;
                                }
                            }
                        }
                    }
                }
            }
            InputEvent::ScrollUp { .. } => {
                self.help.scroll = self.help.scroll.saturating_sub(3);
            }
            InputEvent::ScrollDown { .. } => {
                self.help.scroll += 3;
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

        // Draw maximize button (↓ = restore, ↑ = maximize)
        if let Some(rect) = layout.get("maximize") {
            let is_maximized = self.state.dialog_saved_bounds.is_some();
            let btn = if is_maximized { "[↓]" } else { "[↑]" };
            self.screen.write_str(rect.y, rect.x, btn, Color::LightGray, Color::Black);
        }

        // Get content area from layout
        if let Some(content_rect) = layout.get("content") {
            let content_width = content_rect.width as usize;
            let content_height = content_rect.height as usize;

            // Get rendered content (lines, links, styles, max_line_width)
            let (lines, links, styles, max_width) = self.help.render(content_width);

            // Clamp vertical scroll to valid range
            // Allow scrolling until only the last line is visible at the top
            let max_vscroll = lines.len().saturating_sub(1);
            if self.help.scroll > max_vscroll {
                self.help.scroll = max_vscroll;
            }

            // Clamp horizontal scroll to valid range
            // Allow scrolling until only the last column is visible at the left
            let max_hscroll = max_width.saturating_sub(1);
            if self.help.scroll_col > max_hscroll {
                self.help.scroll_col = max_hscroll;
            }

            let scroll = self.help.scroll;
            let scroll_col = self.help.scroll_col;
            let selected_link = self.help.selected_link;

            // Draw each visible line
            for (i, line) in lines.iter().skip(scroll).take(content_height).enumerate() {
                let row = content_rect.y + i as u16;
                let col = content_rect.x;
                let line_idx = scroll + i;

                // Find all links on this line
                let line_links: Vec<_> = links.iter().enumerate()
                    .filter(|(_, link)| link.line == line_idx)
                    .collect();

                // Find all styles on this line
                let line_styles: Vec<_> = styles.iter()
                    .filter(|s| s.line == line_idx)
                    .collect();

                // Check if this line is a code block (for syntax highlighting)
                let is_code_block = line_styles.iter().any(|s| s.style == help::TextStyle::CodeBlock);

                if is_code_block && line_links.is_empty() {
                    // Use syntax highlighting for code blocks
                    let tokens = tokenize_line(line);
                    let mut x = 0usize;

                    for token in tokens {
                        let token_fg = match token.kind {
                            TokenKind::Keyword => Color::White,
                            TokenKind::String => Color::LightMagenta,
                            TokenKind::Number => Color::LightCyan,
                            TokenKind::Comment => Color::LightGray,
                            TokenKind::Operator => Color::LightGreen,
                            TokenKind::Identifier => Color::Yellow,
                            TokenKind::Punctuation => Color::White,
                            TokenKind::Whitespace => Color::Yellow,
                        };

                        for ch in token.text.chars() {
                            if x >= scroll_col && x - scroll_col < content_width {
                                let screen_x = col + (x - scroll_col) as u16;
                                self.screen.set(row, screen_x, ch, token_fg, Color::Black);
                            }
                            x += 1;
                        }
                    }

                    // Fill remaining space with background
                    for screen_pos in x.saturating_sub(scroll_col)..content_width {
                        self.screen.set(row, col + screen_pos as u16, ' ', Color::Yellow, Color::Black);
                    }
                } else {
                    // Regular rendering with styles and links
                    let chars: Vec<char> = line.chars().collect();

                    for screen_pos in 0..content_width {
                        let char_pos = scroll_col + screen_pos;
                        let ch = chars.get(char_pos).copied().unwrap_or(' ');

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

                        // Check for style at this position
                        let mut style = None;
                        for s in &line_styles {
                            if char_pos >= s.col_start && char_pos < s.col_end {
                                style = Some(s.style);
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
                            match style {
                                Some(help::TextStyle::Code) => (Color::Yellow, Color::Black),
                                Some(help::TextStyle::CodeBlock) => (Color::Yellow, Color::Black),
                                Some(help::TextStyle::Bold) => (Color::White, Color::Black),
                                Some(help::TextStyle::Italic) => (Color::Cyan, Color::Black),
                                None => (Color::LightGray, Color::Black),
                            }
                        };

                        self.screen.set(row, col + screen_pos as u16, ch, fg, bg);
                    }
                }
            }

            // Draw vertical scrollbar
            // Use visible_size=1 to match scroll-past-end behavior (max_scroll = content - 1)
            if let Some(vscroll_rect) = layout.get("vscrollbar") {
                let vstate = ScrollbarState::new(scroll, lines.len(), 1);
                let colors = ScrollbarColors::dark();
                scrollbar::draw_vertical(
                    &mut self.screen,
                    vscroll_rect.x,
                    vscroll_rect.y,
                    vscroll_rect.y + vscroll_rect.height.saturating_sub(1),
                    &vstate,
                    &colors,
                );
            }

            // Draw horizontal scrollbar
            // Use visible_size=1 to match scroll-past-end behavior (max_scroll = content - 1)
            if let Some(hscroll_rect) = layout.get("hscrollbar") {
                let hstate = ScrollbarState::new(scroll_col, max_width, 1);
                let colors = ScrollbarColors::dark();
                scrollbar::draw_horizontal(
                    &mut self.screen,
                    hscroll_rect.y,
                    hscroll_rect.x,
                    hscroll_rect.x + hscroll_rect.width.saturating_sub(1),
                    &hstate,
                    &colors,
                );
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

    pub fn new_file(&mut self) {
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

    /// Handle scroll wheel events with position-aware scrolling
    fn handle_scroll_wheel(&mut self, event: &InputEvent, row: u16, col: u16) -> bool {
        let is_up = matches!(event, InputEvent::ScrollUp { .. });
        let is_down = matches!(event, InputEvent::ScrollDown { .. });
        let is_left = matches!(event, InputEvent::ScrollLeft { .. });
        let is_right = matches!(event, InputEvent::ScrollRight { .. });

        // Modal dialogs handle their own scrolling
        if self.modal.is_some() {
            return true; // Consume scroll in modal
        }

        // Use main layout for hit testing
        if let Some(ref layout) = self.state.main_layout {
            let hit_row = row.saturating_sub(1);
            let hit_col = col.saturating_sub(1);

            if let Some(hit_id) = layout.hit_test(hit_row, hit_col) {
                match hit_id.as_str() {
                    "editor" => {
                        if is_up {
                            self.editor.scroll_row = self.editor.scroll_row.saturating_sub(3);
                        } else if is_down {
                            let max_scroll = self.editor.buffer.line_count().saturating_sub(1);
                            self.editor.scroll_row = (self.editor.scroll_row + 3).min(max_scroll);
                        } else if is_left {
                            self.editor.scroll_col = self.editor.scroll_col.saturating_sub(6);
                        } else if is_right {
                            self.editor.scroll_col += 6;
                        }
                        return true;
                    }
                    "immediate" => {
                        // Could add immediate window scroll support here
                        return true;
                    }
                    "output" => {
                        if is_up {
                            self.output.scroll_up(3);
                        } else if is_down {
                            self.output.scroll_down(3);
                        }
                        return true;
                    }
                    _ => {}
                }
            }
        }

        // Default: scroll editor if focused
        if self.state.focus == Focus::Editor {
            if is_up {
                self.editor.scroll_row = self.editor.scroll_row.saturating_sub(3);
            } else if is_down {
                self.editor.scroll_row += 3;
            } else if is_left {
                self.editor.scroll_col = self.editor.scroll_col.saturating_sub(6);
            } else if is_right {
                self.editor.scroll_col += 6;
            }
        }

        true
    }

    fn handle_mouse_click(&mut self, row: u16, col: u16) -> bool {
        let (_width, _height) = self.screen.size();

        // If a dialog is open, handle dialog clicks
        if !matches!(self.state.dialog, DialogType::None) {
            let dialog_x = self.state.dialog_x;
            let dialog_y = self.state.dialog_y;
            let dialog_width = self.state.dialog_width;
            let dialog_height = self.state.dialog_height;

            // Use cached dialog layout for all hit testing
            let dialog_layout = self.state.dialog_layout.clone();
            if let Some(layout) = dialog_layout {
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
                            // Check for double-click to toggle maximize
                            let now = std::time::Instant::now();
                            let elapsed = now.duration_since(self.state.last_click_time);
                            let same_row = self.state.last_click_pos.0 == row;
                            let is_double_click = same_row && elapsed.as_millis() < 400;

                            self.state.last_click_time = now;
                            self.state.last_click_pos = (row, col);

                            if is_double_click {
                                // Toggle maximize
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
                            } else {
                                self.state.dialog_dragging = true;
                                self.state.dialog_drag_offset = (col - dialog_x, row - dialog_y);
                            }
                            return true;
                        }
                        "resize_handle" => {
                            self.state.dialog_resizing = true;
                            return true;
                        }
                        _ => {
                            // Handle button clicks
                            return self.handle_dialog_button_click(&hit_id);
                        }
                    }
                }
            }

            // Check if click is inside dialog bounds
            let dialog_bounds = Rect::new(dialog_x, dialog_y, dialog_width, dialog_height);
            if dialog_bounds.contains(row, col) {
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

            // If menu is open, check for clicks on dropdown first
            if self.state.menu_open {
                match self.menubar.handle_dropdown_click(row, col, &self.state) {
                    MenuClickResult::Execute(menu_idx, item_idx) => {
                        self.state.close_menu();
                        self.handle_menu_action(menu_idx, item_idx);
                        return true;
                    }
                    MenuClickResult::Absorbed => {
                        return true;
                    }
                    MenuClickResult::CloseMenu => {
                        self.state.close_menu();
                        // Fall through to handle click on underlying element
                    }
                    _ => {}
                }
            }

            // Check which main area was clicked
            if let Some(hit_id) = layout.hit_test(hit_row, hit_col) {
                match hit_id.as_str() {
                    "menu_bar" => {
                        if let Some(menu_bar_rect) = layout.get("menu_bar") {
                            match self.menubar.handle_bar_click(row, col, menu_bar_rect, &self.state) {
                                MenuClickResult::OpenMenu(i) => {
                                    self.state.menu_index = i;
                                    self.state.menu_item = 0;
                                    self.state.open_menu();
                                }
                                MenuClickResult::CloseMenu => {
                                    self.state.close_menu();
                                }
                                _ => {}
                            }
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
                            if let OutputClickResult::Close = self.output.handle_click(row, col, out_rect) {
                                self.state.show_output = false;
                            }
                        }
                        return true;
                    }
                    "immediate" => {
                        if let Some(imm_rect) = layout.get("immediate") {
                            match self.immediate.handle_click(row, col, imm_rect, self.state.immediate_maximized) {
                                ImmediateClickResult::ToggleMaximize => {
                                    self.state.immediate_maximized = !self.state.immediate_maximized;
                                }
                                ImmediateClickResult::StartResize => {
                                    self.state.immediate_resize_dragging = true;
                                }
                                ImmediateClickResult::Focus => {
                                    self.state.focus = Focus::Immediate;
                                }
                                ImmediateClickResult::None => {}
                            }
                        }
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

    /// Handle clicks within the editor area using widget-based hit testing
    fn handle_editor_click(&mut self, row: u16, col: u16, editor_rect: Rect) -> bool {
        let line_count = self.editor.buffer.line_count().max(1);
        let max_line_len = self.editor.buffer.max_line_length().max(1);

        let action = widget_editor_click(
            row, col, editor_rect,
            self.editor.scroll_row,
            self.editor.scroll_col,
            line_count,
            max_line_len,
            self.editor.visible_lines,
            self.editor.visible_cols,
        );

        match action {
            EditorClickAction::VScroll(scroll_action) => {
                let page_size = self.editor.visible_lines.max(1);
                let max_scroll = line_count.saturating_sub(1);

                match scroll_action {
                    ScrollAction::ScrollBack(n) => {
                        self.editor.scroll_row = self.editor.scroll_row.saturating_sub(n);
                    }
                    ScrollAction::ScrollForward(n) => {
                        self.editor.scroll_row = (self.editor.scroll_row + n).min(max_scroll);
                    }
                    ScrollAction::PageBack => {
                        self.editor.scroll_row = self.editor.scroll_row.saturating_sub(page_size);
                    }
                    ScrollAction::PageForward => {
                        self.editor.scroll_row = (self.editor.scroll_row + page_size).min(max_scroll);
                    }
                    ScrollAction::SetPosition(pos) => {
                        self.editor.scroll_row = pos.min(max_scroll);
                    }
                    _ => {}
                }
                true
            }
            EditorClickAction::StartVDrag => {
                self.state.vscroll_dragging = true;
                true
            }
            EditorClickAction::HScroll(scroll_action) => {
                let page_size = self.editor.visible_cols.max(1);
                let max_scroll = max_line_len.saturating_sub(1);

                match scroll_action {
                    ScrollAction::ScrollBack(n) => {
                        self.editor.scroll_col = self.editor.scroll_col.saturating_sub(n);
                    }
                    ScrollAction::ScrollForward(n) => {
                        self.editor.scroll_col = (self.editor.scroll_col + n).min(max_scroll);
                    }
                    ScrollAction::PageBack => {
                        self.editor.scroll_col = self.editor.scroll_col.saturating_sub(page_size);
                    }
                    ScrollAction::PageForward => {
                        self.editor.scroll_col = (self.editor.scroll_col + page_size).min(max_scroll);
                    }
                    ScrollAction::SetPosition(pos) => {
                        self.editor.scroll_col = pos.min(max_scroll);
                    }
                    _ => {}
                }
                true
            }
            EditorClickAction::StartHDrag => {
                self.state.hscroll_dragging = true;
                true
            }
            EditorClickAction::ContentClick { editor_y, editor_x } => {
                // Set cursor position
                let target_line = self.editor.scroll_row + editor_y;
                let target_col = self.editor.scroll_col + editor_x;

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
                    self.state.click_count = (self.state.click_count + 1).min(4);
                } else {
                    self.state.click_count = 1;
                }

                self.state.last_click_time = now;
                self.state.last_click_pos = (row, col);

                // Handle based on click count
                match self.state.click_count {
                    2 => {
                        self.editor.select_word();
                        self.editor.is_selecting = true;
                        self.state.selection_anchor = self.editor.get_selection_bounds();
                    }
                    3 => {
                        self.editor.select_line();
                        self.editor.is_selecting = true;
                        self.state.selection_anchor = self.editor.get_selection_bounds();
                    }
                    4 => {
                        self.editor.select_paragraph();
                        self.editor.is_selecting = true;
                        self.state.selection_anchor = self.editor.get_selection_bounds();
                    }
                    _ => {
                        self.editor.start_selection();
                        self.state.selection_anchor = None;
                    }
                }
                true
            }
            EditorClickAction::None => true, // Click on border - absorb it
        }
    }

    pub fn run_program(&mut self) {
        self.state.run_state = RunState::Running;
        self.state.set_status("Running...");

        // Clear output window and show it
        self.output.clear();
        self.state.show_output = true;
        self.screen.invalidate(); // Force full redraw

        // Clear terminal and screen buffer for fresh start
        let _ = self.terminal.clear();
        let _ = self.terminal.flush();
        self.screen.clear_with(crate::terminal::Color::White, crate::terminal::Color::Black);
        self.screen.invalidate();

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
                let stmt_count = program.len();
                match self.interpreter.execute_with_debug(&program) {
                    Ok(ExecutionResult::Completed) => {
                        // Show output in output window (black bg, white text)
                        for line in self.interpreter.take_output() {
                            self.output.add_output(&line);
                        }
                        self.state.set_status(format!("Program completed ({} stmts)", stmt_count));
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
const SAMPLE_PROGRAM: &str = r####"' ============================================
' Welcome to QBasic!
' ============================================
' This is a demo program showing various
' features of the QBasic interpreter.
' ============================================

CLS
PRINT "Hello, World!"
PRINT

' Simple counting loop
PRINT "Counting from 1 to 10:"
FOR i = 1 TO 10
    PRINT "  Count:"; i
NEXT i
PRINT

' Nested loops example
PRINT "Multiplication table (1-5):"
FOR i = 1 TO 5
    FOR j = 1 TO 5
        PRINT USING "###"; i * j;
    NEXT j
    PRINT
NEXT i
PRINT

' String manipulation
name$ = "QBasic"
PRINT "Welcome to "; name$; "!"
PRINT "String length:"; LEN(name$)
PRINT

' Simple math
a = 10
b = 3
PRINT "Math operations:"
PRINT "  10 + 3 ="; a + b
PRINT "  10 - 3 ="; a - b
PRINT "  10 * 3 ="; a * b
PRINT "  10 / 3 ="; a / b
PRINT "  10 MOD 3 ="; a MOD 3
PRINT

' ============================================
' Keyboard shortcuts:
' ============================================
' F5        - Run program
' F6        - Switch to Immediate window
' F1        - Help
' Ctrl+O    - Open file
' Ctrl+S    - Save file
' Alt+X     - Exit
' ============================================
"####;

