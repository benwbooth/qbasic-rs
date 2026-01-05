//! Main application module

use std::io;
use crate::terminal::{self, Terminal, Color};
use crate::screen::Screen;
use crate::input::{self, InputEvent};
use crate::state::{AppState, Focus, RunState};
use crate::ui::{Rect, compute_layout, ModalDialog, ModalResult, ModalAction, WidgetAction, Widgets};
use crate::ui::dialogs::{Dialogs, DialogContext, DialogResult, DialogController};
use crate::ui::layout::main_screen_layout;
use crate::basic::{self, Lexer, Parser, Interpreter};

/// Main application
pub struct App {
    terminal: Terminal,
    screen: Screen,
    state: AppState,
    /// All UI widgets
    widgets: Widgets,
    interpreter: Interpreter,
    clipboard: Option<arboard::Clipboard>,
    /// Parsed program stored for resuming execution after NeedsInput
    current_program: Option<Vec<basic::parser::Stmt>>,
    /// Modal dialog (captures all events when open)
    modal: Option<Box<dyn ModalDialog>>,
    /// All dialog instances
    dialogs: Dialogs,
}

impl App {
    pub fn new() -> io::Result<Self> {
        let terminal = Terminal::new()?;
        let (width, height) = terminal.size();
        let screen = Screen::new(width, height);

        Ok(Self {
            terminal,
            screen,
            state: AppState::new(),
            widgets: Widgets::new(),
            interpreter: Interpreter::new(),
            clipboard: arboard::Clipboard::new().ok(),
            current_program: None,
            modal: None,
            dialogs: Dialogs::new(width, height),
        })
    }

    pub fn run(&mut self) -> io::Result<()> {
        // Initialize dialog screen size before opening any dialogs
        let (width, height) = self.terminal.size();
        self.dialogs.set_screen_size(width, height);

        // Show welcome dialog on startup
        let mut ctx = DialogContext {
            editor: &mut self.widgets.editor,
            state: &mut self.state,
        };
        self.dialogs.welcome.open(&mut ctx);

        loop {
            // Handle resize
            self.terminal.update_size();
            let (width, height) = self.terminal.size();
            if (width, height) != self.screen.size() {
                self.screen.resize(width, height);
                self.screen.invalidate();
                // Also resize graphics buffer if program is running
                let (pixel_w, pixel_h) = self.terminal.pixel_size();
                if matches!(self.state.run_state, RunState::Running | RunState::WaitingForInput) {
                    self.interpreter.graphics_mut().resize_pixels(
                        width as u32, height as u32,
                        pixel_w as u32, pixel_h as u32
                    );
                }
                // Update character cell size for sixel positioning
                if pixel_w > 0 && pixel_h > 0 {
                    let char_w = pixel_w as u32 / width as u32;
                    let char_h = pixel_h as u32 / height as u32;
                    self.screen.set_char_size(char_w, char_h);
                }
                self.dialogs.set_screen_size(width, height);
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
                            // Check for Ctrl+C, Ctrl+Break, or Escape to stop program
                            if matches!(key, terminal::Key::Ctrl('c') | terminal::Key::Escape) {
                                self.state.show_output = false;
                                self.state.run_state = RunState::Editing;
                                self.state.set_status("Program stopped");
                                self.current_program = None;
                                self.interpreter.request_stop();
                                self.interpreter.clear_pending_input();
                                // Clear sixel mode and force full redraw of IDE
                                self.screen.clear_sixel();
                                self.screen.invalidate();
                                self.clear_terminal_graphics();
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
                            // Check for Ctrl+C, Ctrl+Break, or Escape to stop program
                            if matches!(key, terminal::Key::Ctrl('c') | terminal::Key::Escape) {
                                self.state.show_output = false;
                                self.state.run_state = RunState::Editing;
                                self.state.set_status("Program stopped");
                                self.current_program = None;
                                self.interpreter.request_stop();
                                // Clear sixel mode and force full redraw of IDE
                                self.screen.clear_sixel();
                                self.screen.invalidate();
                                self.clear_terminal_graphics();
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
                                    self.interpreter.set_pending_key(Some(key_str));
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

            // Continue execution if program is running
            if self.state.run_state == RunState::Running {
                if let Some(ref program) = self.current_program.clone() {
                    use crate::basic::interpreter::ExecutionResult;
                    match self.interpreter.continue_execution(&program) {
                        Ok(ExecutionResult::Running) => {
                            // Still running, will continue next iteration
                        }
                        Ok(ExecutionResult::Completed) => {
                            for line in self.interpreter.take_output() {
                                self.widgets.output.add_output(&line);
                            }
                            self.state.set_status("Program completed");
                            self.state.current_line = None;
                            self.state.run_state = RunState::Finished;
                            self.current_program = None;
                        }
                        Ok(ExecutionResult::Stopped) => {
                            for line in self.interpreter.take_output() {
                                self.widgets.output.add_output(&line);
                            }
                            self.state.set_status("Program stopped");
                            self.state.current_line = None;
                            self.state.run_state = RunState::Finished;
                            self.current_program = None;
                        }
                        Ok(ExecutionResult::NeedsInput) => {
                            for line in self.interpreter.take_output() {
                                self.widgets.output.add_output(&line);
                            }
                            self.state.run_state = RunState::WaitingForInput;
                        }
                        Ok(ExecutionResult::Breakpoint(line)) => {
                            for output_line in self.interpreter.take_output() {
                                self.widgets.output.add_output(&output_line);
                            }
                            self.state.current_line = Some(line);
                            self.state.run_state = RunState::Paused;
                            self.state.show_output = false;
                            self.state.set_status(format!("Breakpoint hit at line {}", line + 1));
                            self.widgets.editor.go_to_line(line + 1);
                        }
                        Ok(ExecutionResult::Stepped(line)) => {
                            self.state.current_line = Some(line);
                            self.state.run_state = RunState::Stepping;
                        }
                        Err(e) => {
                            self.state.show_output = false;
                            self.state.current_line = None;
                            self.state.run_state = RunState::Editing;
                            self.current_program = None;

                            self.dialogs.message.set_message("Runtime Error".to_string(), e.clone());
                            let mut ctx = DialogContext {
                                editor: &mut self.widgets.editor,
                                state: &mut self.state,
                            };
                            self.dialogs.message.open(&mut ctx);
                        }
                    }
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
                self.widgets.output.draw_graphics_screen(&mut self.screen, &mut self.interpreter.graphics_mut(), &self.state);
            } else {
                self.widgets.output.draw_fullscreen(&mut self.screen, &self.state);
            }
            return;
        }

        // Compute main layout
        let main_layout_item = main_screen_layout(
            self.state.show_immediate,
            self.state.immediate_height,
            self.state.immediate_maximized,
            self.state.editor_maximized,
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

        // Draw all widgets through the Widgets container
        self.widgets.draw(&mut self.screen, &self.state, &layout);

        // Draw menu dropdown (must be after widgets so it appears on top)
        if self.state.menu_open {
            self.widgets.menubar.draw_dropdown(&mut self.screen, &self.state);
        }

        // Draw modal dialog if active (takes precedence)
        if let Some(ref modal) = self.modal {
            modal.draw(&mut self.screen);
        } else {
            self.dialogs.draw(&mut self.screen, &self.state);
        }
    }

    fn handle_input(&mut self, event: InputEvent) -> bool {
        // Ignore unknown key sequences
        if let InputEvent::UnknownBytes(_) = &event {
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
                    // Clear sixel mode and force full redraw of IDE
                    self.screen.clear_sixel();
                    self.screen.invalidate();
                    self.clear_terminal_graphics();
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

        // If modal dialog is open, route all events to it
        if self.modal.is_some() {
            return self.handle_modal_event(&event);
        }

        if self.state.focus == Focus::Dialog && !self.dialogs.is_active() {
            self.state.focus = Focus::Editor;
        }

        if self.state.focus == Focus::Dialog {
            let mut ctx = DialogContext {
                editor: &mut self.widgets.editor,
                state: &mut self.state,
            };
            let result = self.dialogs.handle_event(&event, &mut ctx);

            if result == DialogResult::Closed {
                // Check if welcome dialog wants to open help before closing
                let should_open_help = self.dialogs.welcome.show_help_on_close
                    && self.dialogs.welcome.is_open();

                self.dialogs.close_active();
                self.state.focus = Focus::Editor;

                // Open help if welcome dialog requested it
                if should_open_help {
                    self.dialogs.help.set_topic("Index".to_string());
                    let mut ctx = DialogContext {
                        editor: &mut self.widgets.editor,
                        state: &mut self.state,
                    };
                    self.dialogs.help.open(&mut ctx);
                }
            }
            return true;
        }

        // Route all mouse events through widgets
        match &event {
            InputEvent::MouseClick { .. } => {
                return self.route_mouse_event(&event);
            }
            InputEvent::MouseDrag { row, .. } => {
                // Handle immediate window resize drag (app-level concern)
                if self.state.immediate_resize_dragging {
                    let (_screen_width, screen_height) = self.screen.size();
                    if let Some(layout) = &self.state.main_layout {
                        if let Some(imm_rect) = layout.get("immediate") {
                            let imm_bottom = imm_rect.y + imm_rect.height;
                            let new_height = imm_bottom.saturating_sub(*row);
                            let max_height = screen_height / 2;
                            self.state.immediate_height = new_height.max(3).min(max_height);
                        }
                    }
                    return true;
                }
                // Route other drags through widgets
                return self.route_mouse_event(&event);
            }
            InputEvent::MouseRelease { .. } => {
                self.state.immediate_resize_dragging = false;
                return self.route_mouse_event(&event);
            }
            InputEvent::ScrollUp { .. } | InputEvent::ScrollDown { .. } |
            InputEvent::ScrollLeft { .. } | InputEvent::ScrollRight { .. } => {
                return self.handle_scroll_wheel(&event);
            }
            _ => {}
        }

        // Global shortcuts (only when no dialog is open)
        match &event {
            InputEvent::Alt('x') | InputEvent::Ctrl('q') => {
                self.state.should_quit = true;
                return true;
            }
            InputEvent::F(10) => {
                if !self.state.menu_open {
                    self.state.open_menu();
                } else {
                    self.state.close_menu();
                }
                return true;
            }
            InputEvent::F(1) => {
                self.dialogs.help.set_topic("General".to_string());
                let mut ctx = DialogContext {
                    editor: &mut self.widgets.editor,
                    state: &mut self.state,
                };
                self.dialogs.help.open(&mut ctx);
                return true;
            }
            InputEvent::F(4) => {
                // Toggle output window
                self.state.show_output = !self.state.show_output;
                return true;
            }
            InputEvent::F(5) => {
                self.run_program();
                return true;
            }
            InputEvent::F(2) => {
                self.show_subs_list();
                return true;
            }
            InputEvent::F(6) => {
                self.state.toggle_focus();
                return true;
            }
            InputEvent::F(3) => {
                self.repeat_find();
                return true;
            }
            // Search shortcuts
            InputEvent::Ctrl('f') => {
                let mut ctx = DialogContext {
                    editor: &mut self.widgets.editor,
                    state: &mut self.state,
                };
                self.dialogs.find.open(&mut ctx);
                return true;
            }
            InputEvent::Ctrl('g') => {
                let mut ctx = DialogContext {
                    editor: &mut self.widgets.editor,
                    state: &mut self.state,
                };
                self.dialogs.goto.open(&mut ctx);
                return true;
            }
            // File shortcuts
            InputEvent::Ctrl('s') => {
                self.save_file();
                return true;
            }
            InputEvent::Ctrl('o') => {
                let mut ctx = DialogContext {
                    editor: &mut self.widgets.editor,
                    state: &mut self.state,
                };
                self.dialogs.file_open.open(&mut ctx);
                return true;
            }
            InputEvent::Ctrl('n') => {
                self.new_file();
                return true;
            }
            // Clipboard operations
            InputEvent::Ctrl('c') => {
                self.clipboard_copy();
                return true;
            }
            InputEvent::Ctrl('x') => {
                self.clipboard_cut();
                return true;
            }
            InputEvent::Ctrl('v') => {
                self.clipboard_paste();
                return true;
            }
            // Undo/Redo
            InputEvent::Ctrl('z') => {
                if self.widgets.editor.undo() {
                    self.state.set_status("Undo");
                } else {
                    self.state.set_status("Nothing to undo");
                }
                return true;
            }
            InputEvent::Ctrl('y') => {
                if self.widgets.editor.redo() {
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

        // Route keyboard events through widgets based on focus
        self.route_keyboard_event(&event);

        true
    }

    /// Route keyboard events to the focused widget
    fn route_keyboard_event(&mut self, event: &InputEvent) {
        let layout = self.state.main_layout.clone();
        let Some(layout) = layout else { return };

        let action = self.widgets.handle_keyboard_event(event, &mut self.state, &layout);
        self.handle_widget_action(action);
    }

    fn handle_menu_action(&mut self, menu_idx: usize, item_idx: usize) {
        match (menu_idx, item_idx) {
            // File menu
            (0, 0) => self.new_file(),
            (0, 1) => {
                let mut ctx = DialogContext {
                    editor: &mut self.widgets.editor,
                    state: &mut self.state,
                };
                self.dialogs.file_open.open(&mut ctx);
            }
            (0, 2) => self.save_file(),
            (0, 3) => {
                let mut ctx = DialogContext {
                    editor: &mut self.widgets.editor,
                    state: &mut self.state,
                };
                self.dialogs.file_save.open(&mut ctx);
            }
            (0, 5) => {
                let mut ctx = DialogContext {
                    editor: &mut self.widgets.editor,
                    state: &mut self.state,
                };
                self.dialogs.print.open(&mut ctx);
            }
            (0, 7) => self.state.should_quit = true,

            // Edit menu
            (1, 0) => { // Undo
                if self.widgets.editor.undo() {
                    self.state.set_status("Undo");
                }
            }
            (1, 2) => self.clipboard_cut(),
            (1, 3) => self.clipboard_copy(),
            (1, 4) => self.clipboard_paste(),
            (1, 5) => { // Clear - delete selection
                if self.widgets.editor.has_selection() {
                    self.widgets.editor.delete_selection();
                    self.state.set_modified(true);
                }
            }
            (1, 7) => {
                let mut ctx = DialogContext {
                    editor: &mut self.widgets.editor,
                    state: &mut self.state,
                };
                self.dialogs.new_sub.open(&mut ctx);
            }
            (1, 8) => {
                let mut ctx = DialogContext {
                    editor: &mut self.widgets.editor,
                    state: &mut self.state,
                };
                self.dialogs.new_function.open(&mut ctx);
            }

            // View menu
            (2, 0) => self.show_subs_list(),
            (2, 1) => { // Next Statement
                if let Some(line) = self.state.current_line {
                    self.widgets.editor.go_to_line(line + 1);
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
            (2, 4) => self.state.set_status("No included files"),
            (2, 5) => self.state.set_status("No included files"),

            // Search menu
            (3, 0) => {
                let mut ctx = DialogContext {
                    editor: &mut self.widgets.editor,
                    state: &mut self.state,
                };
                self.dialogs.find.open(&mut ctx);
            }
            (3, 1) => self.repeat_find(),
            (3, 2) => {
                let mut ctx = DialogContext {
                    editor: &mut self.widgets.editor,
                    state: &mut self.state,
                };
                self.dialogs.replace.open(&mut ctx);
            }
            (3, 3) => {
                let mut ctx = DialogContext {
                    editor: &mut self.widgets.editor,
                    state: &mut self.state,
                };
                self.dialogs.find_label.open(&mut ctx);
            }

            // Run menu
            (4, 0) | (4, 2) => self.run_program(),
            (4, 1) => self.restart_program(),
            (4, 4) => {
                let mut ctx = DialogContext {
                    editor: &mut self.widgets.editor,
                    state: &mut self.state,
                };
                self.dialogs.command_args.open(&mut ctx);
            }

            // Debug menu
            (5, 0) => self.step_program(),
            (5, 1) => self.step_program(),
            (5, 3) => self.state.toggle_breakpoint(self.widgets.editor.cursor_line),
            (5, 4) => {
                self.state.breakpoints.clear();
                self.state.set_status("All breakpoints cleared");
            }
            (5, 6) => { // Set Next Statement
                if self.state.run_state == RunState::Paused {
                    self.state.current_line = Some(self.widgets.editor.cursor_line);
                    self.state.set_status(format!("Next statement set to line {}", self.widgets.editor.cursor_line + 1));
                } else {
                    self.state.set_status("Must be paused at breakpoint to set next statement");
                }
            }

            // Options menu
            (6, 0) => {
                let mut ctx = DialogContext {
                    editor: &mut self.widgets.editor,
                    state: &mut self.state,
                };
                self.dialogs.display_options.open(&mut ctx);
            }
            (6, 1) => {
                let mut ctx = DialogContext {
                    editor: &mut self.widgets.editor,
                    state: &mut self.state,
                };
                self.dialogs.help_path.open(&mut ctx);
            }
            (6, 2) => { // Syntax Checking toggle
                self.state.syntax_checking = !self.state.syntax_checking;
                if self.state.syntax_checking {
                    self.state.set_status("Syntax checking enabled");
                    self.check_syntax();
                } else {
                    self.state.set_status("Syntax checking disabled");
                    self.state.syntax_errors.clear();
                }
            }

            // Help menu
            (7, 0) => {
                self.dialogs.help.set_topic("Index".to_string());
                let mut ctx = DialogContext {
                    editor: &mut self.widgets.editor,
                    state: &mut self.state,
                };
                self.dialogs.help.open(&mut ctx);
            }
            (7, 1) => {
                self.dialogs.help.set_topic("Contents".to_string());
                let mut ctx = DialogContext {
                    editor: &mut self.widgets.editor,
                    state: &mut self.state,
                };
                self.dialogs.help.open(&mut ctx);
            }
            (7, 2) => self.show_help_for_word_under_cursor(),
            (7, 3) => {
                self.dialogs.help.set_topic("Using Help".to_string());
                let mut ctx = DialogContext {
                    editor: &mut self.widgets.editor,
                    state: &mut self.state,
                };
                self.dialogs.help.open(&mut ctx);
            }
            (7, 5) => {
                let mut ctx = DialogContext {
                    editor: &mut self.widgets.editor,
                    state: &mut self.state,
                };
                self.dialogs.about.open(&mut ctx);
            }

            _ => {}
        }
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
                self.load_file_from_path(path);
                self.state.focus = Focus::Editor;
            }
            ModalAction::FileSave(path) => {
                self.save_file_to_path(path);
                self.state.focus = Focus::Editor;
            }
            ModalAction::Help(topic) => {
                self.dialogs.help.set_topic(topic);
                let mut ctx = DialogContext {
                    editor: &mut self.widgets.editor,
                    state: &mut self.state,
                };
                self.dialogs.help.open(&mut ctx);
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

    pub fn new_file(&mut self) {
        if self.state.modified {
            let mut ctx = DialogContext {
                editor: &mut self.widgets.editor,
                state: &mut self.state,
            };
            self.dialogs.new_program.open(&mut ctx);
        } else {
            self.widgets.editor.clear();
            self.state.file_path = None;
            self.state.modified = false;
        }
    }

    fn save_file(&mut self) {
        if self.state.file_path.is_none() {
            let mut ctx = DialogContext {
                editor: &mut self.widgets.editor,
                state: &mut self.state,
            };
            self.dialogs.file_save.open(&mut ctx);
        } else {
            // Save to file
            if let Some(path) = &self.state.file_path {
                if let Err(e) = std::fs::write(path, self.widgets.editor.content()) {
                    self.state.set_status(format!("Error saving: {}", e));
                } else {
                    self.state.modified = false;
                    self.state.set_status("Saved");
                }
            }
        }
    }

    pub fn load_file_from_path(&mut self, path: std::path::PathBuf) {
        match std::fs::read_to_string(&path) {
            Ok(content) => {
                self.widgets.editor.load(&content);
                self.state.file_path = Some(path);
                self.state.modified = false;
                self.state.set_status("File loaded");
            }
            Err(e) => {
                self.state
                    .set_status(format!("Error loading file: {}", e));
            }
        }
    }

    fn save_file_to_path(&mut self, path: std::path::PathBuf) {
        let content = self.widgets.editor.content();
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
    }

    /// Route scroll wheel events through widgets
    fn handle_scroll_wheel(&mut self, event: &InputEvent) -> bool {
        if self.modal.is_some() {
            return true;
        }

        let layout = self.state.main_layout.clone();
        let Some(layout) = layout else { return true };

        let action = self.widgets.handle_scroll(event, &mut self.state, &layout);
        self.handle_widget_action(action);
        true
    }

    /// Route mouse events through widgets
    fn route_mouse_event(&mut self, event: &InputEvent) -> bool {
        let layout = self.state.main_layout.clone();
        let Some(layout) = layout else { return true };

        let action = self.widgets.handle_mouse_event(event, &mut self.state, &layout);
        self.handle_widget_action(action);
        true
    }

    /// Handle a WidgetAction returned from a widget's handle_event
    /// Returns true if the action was consumed
    fn handle_widget_action(&mut self, action: WidgetAction) -> bool {
        match action {
            WidgetAction::Consumed => true,
            WidgetAction::Ignored => false,
            WidgetAction::SetFocus(focus) => {
                self.state.focus = focus;
                true
            }
            WidgetAction::MenuAction(menu_idx, item_idx) => {
                self.handle_menu_action(menu_idx, item_idx);
                true
            }
            WidgetAction::ExecuteCommand(cmd) => {
                self.execute_immediate(&cmd);
                true
            }
            WidgetAction::Toggle(name) => {
                match name {
                    "show_output" => self.state.show_output = !self.state.show_output,
                    "immediate_maximized" => self.state.immediate_maximized = !self.state.immediate_maximized,
                    "editor_maximized" => self.state.editor_maximized = !self.state.editor_maximized,
                    _ => {}
                }
                true
            }
            WidgetAction::StartDrag(name) => {
                match name {
                    "immediate_resize" => self.state.immediate_resize_dragging = true,
                    "vscroll" => self.state.vscroll_dragging = true,
                    "hscroll" => self.state.hscroll_dragging = true,
                    _ => {}
                }
                true
            }
        }
    }

    pub fn run_program(&mut self) {
        self.state.run_state = RunState::Running;
        self.state.set_status("Running...");

        // Clear output window and show it
        self.widgets.output.clear();
        self.state.show_output = true;
        self.screen.invalidate(); // Force full redraw

        // Clear terminal and screen buffer for fresh start
        let _ = self.terminal.clear();
        let _ = self.terminal.flush();
        self.screen.clear_with(crate::terminal::Color::White, crate::terminal::Color::Black);
        self.screen.invalidate();

        // Parse and execute
        let source = self.widgets.editor.content();
        let mut lexer = Lexer::new(&source);
        let tokens = lexer.tokenize();
        let mut parser = Parser::new(tokens);

        match parser.parse() {
            Ok(program) => {
                self.interpreter.reset();

                // Size graphics buffer to terminal size (using actual pixel dimensions)
                let (cols, rows) = self.terminal.size();
                let (pixel_w, pixel_h) = self.terminal.pixel_size();
                self.interpreter.graphics_mut().resize_pixels(
                    cols as u32, rows as u32,
                    pixel_w as u32, pixel_h as u32
                );
                // Enable graphics mode so PRINT/LOCATE/COLOR work with screen buffer
                self.interpreter.graphics_mut().mode = 12;
                self.interpreter.graphics_mut().cls();

                // Update character cell size for sixel positioning
                if pixel_w > 0 && pixel_h > 0 {
                    let char_w = pixel_w as u32 / cols as u32;
                    let char_h = pixel_h as u32 / rows as u32;
                    self.screen.set_char_size(char_w, char_h);
                }

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
                            self.widgets.output.add_output(&line);
                        }
                        self.state.set_status(format!("Program completed ({} stmts)", stmt_count));
                        self.state.current_line = None;
                        self.state.run_state = RunState::Finished;
                        self.current_program = None;
                    }
                    Ok(ExecutionResult::Stopped) => {
                        for line in self.interpreter.take_output() {
                            self.widgets.output.add_output(&line);
                        }
                        self.state.set_status("Program stopped");
                        self.state.current_line = None;
                        self.state.run_state = RunState::Finished;
                        self.current_program = None;
                    }
                    Ok(ExecutionResult::Breakpoint(line)) => {
                        // Show output so far
                        for output_line in self.interpreter.take_output() {
                            self.widgets.output.add_output(&output_line);
                        }
                        self.state.current_line = Some(line);
                        self.state.run_state = RunState::Paused;
                        self.state.show_output = false; // Return to editor for breakpoint
                        self.state.set_status(format!("Breakpoint hit at line {}", line + 1));
                        self.widgets.editor.go_to_line(line + 1);
                    }
                    Ok(ExecutionResult::Stepped(line)) => {
                        self.state.current_line = Some(line);
                        self.state.run_state = RunState::Stepping;
                    }
                    Ok(ExecutionResult::NeedsInput) => {
                        // Show output so far
                        for output_line in self.interpreter.take_output() {
                            self.widgets.output.add_output(&output_line);
                        }
                        // Yield to allow UI update and keyboard input
                        self.state.run_state = RunState::WaitingForInput;
                    }
                    Ok(ExecutionResult::Running) => {
                        // Still running, main loop will continue execution
                        // State is already Running
                    }
                    Err(e) => {
                        // Show runtime error in popup dialog
                        self.state.show_output = false;
                        self.state.current_line = None;
                        self.state.run_state = RunState::Editing;
                        self.current_program = None;

                        self.dialogs.message.set_message("Runtime Error".to_string(), e.clone());
                        let mut ctx = DialogContext {
                            editor: &mut self.widgets.editor,
                            state: &mut self.state,
                        };
                        self.dialogs.message.open(&mut ctx);
                    }
                }
            }
            Err(e) => {
                // Show syntax error in popup dialog
                self.state.show_output = false;
                self.state.run_state = RunState::Editing;

                self.dialogs.message.set_message("Syntax Error".to_string(), e.clone());
                let mut ctx = DialogContext {
                    editor: &mut self.widgets.editor,
                    state: &mut self.state,
                };
                self.dialogs.message.open(&mut ctx);
            }
        }
    }

    fn restart_program(&mut self) {
        self.interpreter.reset();
        self.state.current_line = None;
        self.run_program();
    }

    fn step_program(&mut self) {
        let source = self.widgets.editor.content();
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
                            self.widgets.output.add_output(&line);
                        }
                        self.state.set_status("Program completed");
                        self.state.current_line = None;
                        self.state.run_state = RunState::Editing;
                        self.current_program = None;
                    }
                    Ok(ExecutionResult::Stopped) => {
                        for line in self.interpreter.take_output() {
                            self.widgets.output.add_output(&line);
                        }
                        self.state.set_status("Program stopped");
                        self.state.current_line = None;
                        self.state.run_state = RunState::Editing;
                        self.current_program = None;
                    }
                    Ok(ExecutionResult::Breakpoint(line)) | Ok(ExecutionResult::Stepped(line)) => {
                        for output_line in self.interpreter.take_output() {
                            self.widgets.output.add_output(&output_line);
                        }
                        self.state.current_line = Some(line);
                        self.state.run_state = RunState::Stepping;
                        self.state.set_status(format!("Step: line {} - F8 to continue", line + 1));
                        self.widgets.editor.go_to_line(line + 1);
                    }
                    Ok(ExecutionResult::NeedsInput) => {
                        for output_line in self.interpreter.take_output() {
                            self.widgets.output.add_output(&output_line);
                        }
                        self.state.run_state = RunState::WaitingForInput;
                    }
                    Ok(ExecutionResult::Running) => {
                        self.state.run_state = RunState::Running;
                    }
                    Err(e) => {
                        self.state.show_output = false;
                        self.state.current_line = None;
                        self.state.run_state = RunState::Editing;
                        self.current_program = None;

                        self.dialogs.message.set_message("Runtime Error".to_string(), e.clone());
                        let mut ctx = DialogContext {
                            editor: &mut self.widgets.editor,
                            state: &mut self.state,
                        };
                        self.dialogs.message.open(&mut ctx);
                    }
                }
            }
            Err(e) => {
                self.state.show_output = false;
                self.state.run_state = RunState::Editing;

                self.dialogs.message.set_message("Syntax Error".to_string(), e.clone());
                let mut ctx = DialogContext {
                    editor: &mut self.widgets.editor,
                    state: &mut self.state,
                };
                self.dialogs.message.open(&mut ctx);
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
                    self.widgets.output.add_output(&line);
                }
                self.state.set_status("Program completed");
                self.state.current_line = None;
                self.state.run_state = RunState::Finished;
                self.current_program = None;
            }
            Ok(ExecutionResult::Stopped) => {
                for line in self.interpreter.take_output() {
                    self.widgets.output.add_output(&line);
                }
                self.state.set_status("Program stopped");
                self.state.current_line = None;
                self.state.run_state = RunState::Finished;
                self.current_program = None;
            }
            Ok(ExecutionResult::Breakpoint(line)) => {
                for output_line in self.interpreter.take_output() {
                    self.widgets.output.add_output(&output_line);
                }
                self.state.current_line = Some(line);
                self.state.run_state = RunState::Paused;
                self.state.show_output = false;
                self.state.set_status(format!("Breakpoint hit at line {}", line + 1));
                self.widgets.editor.go_to_line(line + 1);
            }
            Ok(ExecutionResult::Stepped(line)) => {
                self.state.current_line = Some(line);
                self.state.run_state = RunState::Stepping;
            }
            Ok(ExecutionResult::NeedsInput) => {
                for output_line in self.interpreter.take_output() {
                    self.widgets.output.add_output(&output_line);
                }
                self.state.run_state = RunState::WaitingForInput;
            }
            Ok(ExecutionResult::Running) => {
                self.state.run_state = RunState::Running;
            }
            Err(e) => {
                self.state.show_output = false;
                self.state.current_line = None;
                self.state.run_state = RunState::Editing;
                self.current_program = None;

                self.dialogs.message.set_message("Runtime Error".to_string(), e.clone());
                let mut ctx = DialogContext {
                    editor: &mut self.widgets.editor,
                    state: &mut self.state,
                };
                self.dialogs.message.open(&mut ctx);
            }
        }
    }

    fn execute_immediate(&mut self, cmd: &str) {
        // Try to parse and execute as expression or statement
        let source = cmd.trim();
        let mut lexer = Lexer::new(source);
        let tokens = lexer.tokenize();
        let mut parser = Parser::new(tokens);

        // Try as expression first
        if let Ok(expr) = parser.parse_expression() {
            match self.interpreter.eval_expr(&expr) {
                Ok(value) => {
                    self.widgets.output.add_output(&value.to_string());
                }
                Err(e) => {
                    self.widgets.output.add_output(&format!("Error: {}", e));
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
                        self.widgets.output.add_output(&format!("Error: {}", e));
                    } else {
                        for line in self.interpreter.take_output() {
                            self.widgets.output.add_output(&line);
                        }
                    }
                }
                Err(e) => {
                    self.widgets.output.add_output(&format!("Error: {}", e));
                }
            }
        }
    }

    fn clipboard_copy(&mut self) {
        if let Some(text) = self.widgets.editor.get_selected_text() {
            if let Some(ref mut clipboard) = self.clipboard {
                let _ = clipboard.set_text(&text);
                self.state.set_status("Copied to clipboard");
                // Exit keyboard select mode after copy
                self.widgets.editor.keyboard_select_mode = false;
            }
        }
    }

    fn clipboard_cut(&mut self) {
        if let Some(text) = self.widgets.editor.get_selected_text() {
            if let Some(ref mut clipboard) = self.clipboard {
                if clipboard.set_text(&text).is_ok() {
                    self.widgets.editor.delete_selection();
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
                if self.widgets.editor.has_selection() {
                    self.widgets.editor.delete_selection();
                }
                self.widgets.editor.insert_text(&text);
                self.state.set_modified(true);
                self.state.set_status("Pasted from clipboard");
            }
        }
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

        for (line_idx, line) in self.widgets.editor.buffer.lines.iter().enumerate() {
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
        let last_line = self.widgets.editor.buffer.line_count().saturating_sub(1);
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
            let mut ctx = DialogContext {
                editor: &mut self.widgets.editor,
                state: &mut self.state,
            };
            self.dialogs.find.open(&mut ctx);
            return;
        }

        if let Some((line, col)) = self.widgets.editor.find_text(&search, self.state.search_case_sensitive, self.state.search_whole_word) {
            self.widgets.editor.go_to_and_select(line, col, search.len());
            self.state.set_status(format!("Found at line {}", line + 1));
        } else {
            self.state.set_status("Match not found");
        }
    }

    /// Show list of SUBs and FUNCTIONs (F2)
    fn show_subs_list(&mut self) {
        // Parse the source to find SUB and FUNCTION definitions
        let source = self.widgets.editor.content();
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
        let topic = if let Some(line) = self.widgets.editor.buffer.line(self.widgets.editor.cursor_line) {
            let col = self.widgets.editor.cursor_col;
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
                    chars[start..end].iter().collect::<String>().to_uppercase()
                } else {
                    "General".to_string()
                }
            } else {
                "General".to_string()
            }
        } else {
            "General".to_string()
        };

        self.dialogs.help.set_topic(topic);
        let mut ctx = DialogContext {
            editor: &mut self.widgets.editor,
            state: &mut self.state,
        };
        self.dialogs.help.open(&mut ctx);
    }

    /// Clear terminal graphics (sixel) by filling the screen with spaces
    fn clear_terminal_graphics(&mut self) {
        let (width, height) = self.screen.size();
        // Clear screen and scrollback
        let _ = self.terminal.write_raw("\x1b[H\x1b[2J\x1b[3J");
        // Set default colors and fill screen with spaces to overwrite any sixel remnants
        let _ = self.terminal.write_raw("\x1b[0m");
        for _ in 0..height {
            for _ in 0..width {
                let _ = self.terminal.write_raw(" ");
            }
        }
        let _ = self.terminal.write_raw("\x1b[H");
        let _ = self.terminal.flush();
    }
}
