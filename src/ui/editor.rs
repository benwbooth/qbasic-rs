//! QBasic-style code editor

use crate::screen::Screen;
use crate::terminal::Color;
use crate::state::{AppState, EditorMode};
use super::layout::Rect;

/// BASIC keywords for syntax highlighting
const KEYWORDS: &[&str] = &[
    "ABS", "AND", "AS", "ASC", "ATN",
    "BEEP", "BLOAD", "BSAVE",
    "CALL", "CASE", "CDBL", "CHAIN", "CHDIR", "CHR$", "CINT", "CIRCLE", "CLEAR", "CLNG",
    "CLOSE", "CLS", "COLOR", "COM", "COMMON", "CONST", "COS", "CSNG", "CSRLIN", "CVD",
    "CVDMBF", "CVI", "CVL", "CVS", "CVSMBF",
    "DATA", "DATE$", "DECLARE", "DEF", "DEFDBL", "DEFINT", "DEFLNG", "DEFSNG", "DEFSTR",
    "DIM", "DO", "DOUBLE", "DRAW",
    "ELSE", "ELSEIF", "END", "ENVIRON", "ENVIRON$", "EOF", "EQV", "ERASE", "ERDEV",
    "ERDEV$", "ERL", "ERR", "ERROR", "EXIT", "EXP",
    "FIELD", "FILEATTR", "FILES", "FIX", "FOR", "FRE", "FREEFILE", "FUNCTION",
    "GET", "GOSUB", "GOTO",
    "HEX$",
    "IF", "IMP", "INKEY$", "INP", "INPUT", "INPUT$", "INSTR", "INT", "INTEGER", "IOCTL",
    "IOCTL$", "IS",
    "KEY", "KILL",
    "LBOUND", "LCASE$", "LEFT$", "LEN", "LET", "LINE", "LIST", "LOC", "LOCATE", "LOCK",
    "LOF", "LOG", "LONG", "LOOP", "LPOS", "LPRINT", "LSET", "LTRIM$",
    "MID$", "MKD$", "MKDIR", "MKDMBF$", "MKI$", "MKL$", "MKS$", "MKSMBF$", "MOD",
    "NAME", "NEXT", "NOT",
    "OCT$", "OFF", "ON", "OPEN", "OPTION", "OR", "OUT", "OUTPUT",
    "PAINT", "PALETTE", "PCOPY", "PEEK", "PEN", "PLAY", "PMAP", "POINT", "POKE", "POS",
    "PRESET", "PRINT", "PSET", "PUT",
    "RANDOM", "RANDOMIZE", "READ", "REDIM", "REM", "RESET", "RESTORE", "RESUME", "RETURN",
    "RIGHT$", "RMDIR", "RND", "RSET", "RTRIM$", "RUN",
    "SADD", "SCREEN", "SEEK", "SEG", "SELECT", "SETMEM", "SGN", "SHARED", "SHELL", "SIN",
    "SINGLE", "SLEEP", "SOUND", "SPACE$", "SPC", "SQR", "STATIC", "STEP", "STICK", "STOP",
    "STR$", "STRIG", "STRING", "STRING$", "SUB", "SWAP", "SYSTEM",
    "TAB", "TAN", "THEN", "TIME$", "TIMER", "TO", "TROFF", "TRON", "TYPE",
    "UBOUND", "UCASE$", "UEVENT", "UNLOCK", "UNTIL", "USING",
    "VAL", "VARPTR", "VARPTR$", "VARSEG", "VIEW",
    "WAIT", "WEND", "WHILE", "WIDTH", "WINDOW", "WRITE",
    "XOR",
];

/// Types of undoable actions
#[derive(Clone, Debug)]
pub enum UndoAction {
    /// Insert characters at position (line, col, text)
    Insert { line: usize, col: usize, text: String },
    /// Delete characters at position (line, col, text that was deleted)
    Delete { line: usize, col: usize, text: String },
    /// Delete a newline (joining two lines)
    JoinLines { line: usize, col: usize },
    /// Insert a newline (splitting a line)
    SplitLine { line: usize, col: usize },
}

/// Text buffer for the editor
pub struct TextBuffer {
    pub lines: Vec<String>,
}

impl TextBuffer {
    pub fn new() -> Self {
        Self {
            lines: vec![String::new()],
        }
    }

    pub fn from_string(text: &str) -> Self {
        let lines: Vec<String> = text.lines().map(|s| s.to_string()).collect();
        Self {
            lines: if lines.is_empty() { vec![String::new()] } else { lines },
        }
    }

    pub fn line_count(&self) -> usize {
        self.lines.len()
    }

    pub fn max_line_length(&self) -> usize {
        self.lines.iter().map(|l| l.len()).max().unwrap_or(0)
    }

    pub fn line(&self, n: usize) -> Option<&str> {
        self.lines.get(n).map(|s| s.as_str())
    }

    pub fn line_mut(&mut self, n: usize) -> Option<&mut String> {
        self.lines.get_mut(n)
    }

    pub fn insert_char(&mut self, line: usize, col: usize, ch: char) {
        if let Some(l) = self.lines.get_mut(line) {
            let col = col.min(l.len());
            l.insert(col, ch);
        }
    }

    pub fn delete_char(&mut self, line: usize, col: usize) {
        if let Some(l) = self.lines.get_mut(line) {
            if col < l.len() {
                l.remove(col);
            }
        }
    }

    pub fn backspace(&mut self, line: usize, col: usize) -> (usize, usize) {
        if col > 0 {
            self.delete_char(line, col - 1);
            (line, col - 1)
        } else if line > 0 {
            // Join with previous line
            let current = self.lines.remove(line);
            let prev_len = self.lines[line - 1].len();
            self.lines[line - 1].push_str(&current);
            (line - 1, prev_len)
        } else {
            (line, col)
        }
    }

    pub fn insert_newline(&mut self, line: usize, col: usize) -> (usize, usize) {
        if let Some(l) = self.lines.get_mut(line) {
            let col = col.min(l.len());
            let rest = l.split_off(col);
            self.lines.insert(line + 1, rest);
        }
        (line + 1, 0)
    }

    pub fn delete_line(&mut self, line: usize) {
        if self.lines.len() > 1 && line < self.lines.len() {
            self.lines.remove(line);
        }
    }

    pub fn to_string(&self) -> String {
        self.lines.join("\n")
    }

    pub fn clear(&mut self) {
        self.lines = vec![String::new()];
    }
}

impl Default for TextBuffer {
    fn default() -> Self {
        Self::new()
    }
}

/// The main editor component
pub struct Editor {
    pub buffer: TextBuffer,
    pub cursor_line: usize,
    pub cursor_col: usize,
    pub scroll_row: usize,
    pub scroll_col: usize,
    pub selection_start: Option<(usize, usize)>,  // (line, col)
    pub selection_end: Option<(usize, usize)>,    // (line, col)
    pub is_selecting: bool,  // True when mouse drag started in editor
    pub undo_stack: Vec<UndoAction>,
    pub redo_stack: Vec<UndoAction>,
}

impl Editor {
    pub fn new() -> Self {
        Self {
            buffer: TextBuffer::new(),
            cursor_line: 0,
            cursor_col: 0,
            scroll_row: 0,
            scroll_col: 0,
            selection_start: None,
            selection_end: None,
            is_selecting: false,
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
        }
    }

    /// Start a selection at the current cursor position
    pub fn start_selection(&mut self) {
        self.selection_start = Some((self.cursor_line, self.cursor_col));
        self.selection_end = None;
        self.is_selecting = true;
    }

    /// Update the selection end to the current cursor position
    pub fn update_selection(&mut self) {
        if self.is_selecting {
            self.selection_end = Some((self.cursor_line, self.cursor_col));
        }
    }

    /// End selection mode
    pub fn end_selection(&mut self) {
        self.is_selecting = false;
    }

    /// Clear the current selection
    pub fn clear_selection(&mut self) {
        self.selection_start = None;
        self.selection_end = None;
        self.is_selecting = false;
    }

    /// Check if a position is within the selection
    pub fn is_selected(&self, line: usize, col: usize) -> bool {
        let (start, end) = match (self.selection_start, self.selection_end) {
            (Some(s), Some(e)) => {
                // Normalize so start is before end
                if s.0 < e.0 || (s.0 == e.0 && s.1 <= e.1) {
                    (s, e)
                } else {
                    (e, s)
                }
            }
            _ => return false,
        };

        if line < start.0 || line > end.0 {
            return false;
        }

        if line == start.0 && line == end.0 {
            // Same line selection
            col >= start.1 && col < end.1
        } else if line == start.0 {
            // First line of multi-line selection
            col >= start.1
        } else if line == end.0 {
            // Last line of multi-line selection
            col < end.1
        } else {
            // Middle lines are fully selected
            true
        }
    }

    /// Check if there is an active selection
    pub fn has_selection(&self) -> bool {
        self.selection_start.is_some() && self.selection_end.is_some()
    }

    /// Get normalized selection bounds (start always before end)
    /// Returns ((start_line, start_col), (end_line, end_col))
    pub fn get_selection_bounds(&self) -> Option<((usize, usize), (usize, usize))> {
        match (self.selection_start, self.selection_end) {
            (Some(s), Some(e)) => {
                if s.0 < e.0 || (s.0 == e.0 && s.1 <= e.1) {
                    Some((s, e))
                } else {
                    Some((e, s))
                }
            }
            _ => None,
        }
    }

    /// Get the selected text as a String
    pub fn get_selected_text(&self) -> Option<String> {
        let ((start_line, start_col), (end_line, end_col)) = self.get_selection_bounds()?;

        if start_line == end_line {
            // Single line selection
            let line = self.buffer.line(start_line)?;
            let start = start_col.min(line.len());
            let end = end_col.min(line.len());
            Some(line[start..end].to_string())
        } else {
            // Multi-line selection
            let mut result = String::new();

            // First line (from start_col to end)
            if let Some(first_line) = self.buffer.line(start_line) {
                let start = start_col.min(first_line.len());
                result.push_str(&first_line[start..]);
                result.push('\n');
            }

            // Middle lines (full lines)
            for line_num in (start_line + 1)..end_line {
                if let Some(line) = self.buffer.line(line_num) {
                    result.push_str(line);
                    result.push('\n');
                }
            }

            // Last line (from start to end_col)
            if let Some(last_line) = self.buffer.line(end_line) {
                let end = end_col.min(last_line.len());
                result.push_str(&last_line[..end]);
            }

            Some(result)
        }
    }

    /// Delete the selected text and position cursor at selection start
    /// Returns true if selection was deleted
    pub fn delete_selection(&mut self) -> bool {
        let bounds = match self.get_selection_bounds() {
            Some(b) => b,
            None => return false,
        };

        let ((start_line, start_col), (end_line, end_col)) = bounds;

        if start_line == end_line {
            // Single line deletion
            if let Some(line) = self.buffer.lines.get_mut(start_line) {
                let start = start_col.min(line.len());
                let end = end_col.min(line.len());
                line.drain(start..end);
            }
        } else {
            // Multi-line deletion
            // Get the part of first line before selection
            let prefix = self.buffer.line(start_line)
                .map(|l| l[..start_col.min(l.len())].to_string())
                .unwrap_or_default();

            // Get the part of last line after selection
            let suffix = self.buffer.line(end_line)
                .map(|l| l[end_col.min(l.len())..].to_string())
                .unwrap_or_default();

            // Remove lines from end to start+1 (in reverse order)
            for _ in (start_line + 1)..=end_line {
                if start_line + 1 < self.buffer.lines.len() {
                    self.buffer.lines.remove(start_line + 1);
                }
            }

            // Combine prefix and suffix on the start line
            if let Some(line) = self.buffer.lines.get_mut(start_line) {
                *line = prefix + &suffix;
            }
        }

        // Position cursor at start of deleted selection
        self.cursor_line = start_line;
        self.cursor_col = start_col;

        // Clear the selection
        self.clear_selection();

        true
    }

    /// Select all text in the buffer
    pub fn select_all(&mut self) {
        self.selection_start = Some((0, 0));
        let last_line = self.buffer.line_count().saturating_sub(1);
        let last_col = self.buffer.line(last_line).map(|l| l.len()).unwrap_or(0);
        self.selection_end = Some((last_line, last_col));
        self.is_selecting = false;
    }

    /// Insert text at cursor position (handles multi-line text for paste)
    pub fn insert_text(&mut self, text: &str) {
        for ch in text.chars() {
            if ch == '\n' {
                let (new_line, new_col) = self.buffer.insert_newline(self.cursor_line, self.cursor_col);
                self.cursor_line = new_line;
                self.cursor_col = new_col;
            } else if ch != '\r' {  // Skip carriage returns
                self.buffer.insert_char(self.cursor_line, self.cursor_col, ch);
                self.cursor_col += 1;
            }
        }
    }

    /// Record an action for undo
    fn record_undo(&mut self, action: UndoAction) {
        self.undo_stack.push(action);
        // Clear redo stack when new action is performed
        self.redo_stack.clear();
    }

    /// Undo the last action
    pub fn undo(&mut self) -> bool {
        if let Some(action) = self.undo_stack.pop() {
            match &action {
                UndoAction::Insert { line, col, text } => {
                    // To undo an insert, delete the text
                    let line = *line;
                    let col = *col;
                    for _ in 0..text.len() {
                        self.buffer.delete_char(line, col);
                    }
                    self.cursor_line = line;
                    self.cursor_col = col;
                }
                UndoAction::Delete { line, col, text } => {
                    // To undo a delete, insert the text back
                    let line = *line;
                    let mut insert_col = *col;
                    for ch in text.chars() {
                        self.buffer.insert_char(line, insert_col, ch);
                        insert_col += 1;
                    }
                    self.cursor_line = line;
                    self.cursor_col = insert_col;
                }
                UndoAction::SplitLine { line, col } => {
                    // To undo a split, join the lines
                    let line = *line;
                    let col = *col;
                    if line + 1 < self.buffer.line_count() {
                        let next_line = self.buffer.lines.remove(line + 1);
                        self.buffer.lines[line].push_str(&next_line);
                    }
                    self.cursor_line = line;
                    self.cursor_col = col;
                }
                UndoAction::JoinLines { line, col } => {
                    // To undo a join, split the line
                    let line = *line;
                    let col = *col;
                    if let Some(l) = self.buffer.lines.get_mut(line) {
                        let rest = l.split_off(col);
                        self.buffer.lines.insert(line + 1, rest);
                    }
                    self.cursor_line = line;
                    self.cursor_col = col;
                }
            }
            self.redo_stack.push(action);
            self.clear_selection();
            true
        } else {
            false
        }
    }

    /// Redo the last undone action
    pub fn redo(&mut self) -> bool {
        if let Some(action) = self.redo_stack.pop() {
            match &action {
                UndoAction::Insert { line, col, text } => {
                    // To redo an insert, insert the text again
                    let line = *line;
                    let mut col = *col;
                    for ch in text.chars() {
                        self.buffer.insert_char(line, col, ch);
                        col += 1;
                    }
                    self.cursor_line = line;
                    self.cursor_col = col;
                }
                UndoAction::Delete { line, col, text } => {
                    // To redo a delete, delete the text again
                    let line = *line;
                    let col = *col;
                    for _ in 0..text.len() {
                        self.buffer.delete_char(line, col);
                    }
                    self.cursor_line = line;
                    self.cursor_col = col;
                }
                UndoAction::SplitLine { line, col } => {
                    // To redo a split, split the line again
                    let line = *line;
                    let col = *col;
                    if let Some(l) = self.buffer.lines.get_mut(line) {
                        let rest = l.split_off(col);
                        self.buffer.lines.insert(line + 1, rest);
                    }
                    self.cursor_line = line + 1;
                    self.cursor_col = 0;
                }
                UndoAction::JoinLines { line, col } => {
                    // To redo a join, join the lines again
                    let line = *line;
                    let col = *col;
                    if line + 1 < self.buffer.line_count() {
                        let next_line = self.buffer.lines.remove(line + 1);
                        self.buffer.lines[line].push_str(&next_line);
                    }
                    self.cursor_line = line;
                    self.cursor_col = col;
                }
            }
            self.undo_stack.push(action);
            self.clear_selection();
            true
        } else {
            false
        }
    }

    /// Draw the editor
    pub fn draw(&self, screen: &mut Screen, state: &AppState, bounds: Rect) {
        let row = bounds.y + 1; // Convert 0-based to 1-based
        let col = bounds.x + 1;
        let width = bounds.width;
        let height = bounds.height;

        // Draw editor background
        for r in 0..height {
            for c in 0..width {
                screen.set(row + r, col + c, ' ', Color::Yellow, Color::Blue);
            }
        }

        // Draw border with title
        self.draw_border(screen, state, bounds);

        // Draw content area (inside border)
        let content_row = row + 1;
        let content_col = col + 1;
        let content_width = width.saturating_sub(2);
        let content_height = height.saturating_sub(2);

        // Draw lines
        for r in 0..content_height as usize {
            let line_num = self.scroll_row + r;
            let screen_row = content_row + r as u16;

            if let Some(line) = self.buffer.line(line_num) {
                // Draw the line with syntax highlighting
                self.draw_line(screen, screen_row, content_col, content_width, line, state, line_num);
            }
        }

        // Update cursor position (only if cursor is visible in current scroll view)
        if self.cursor_line >= self.scroll_row && self.cursor_col >= self.scroll_col {
            let cursor_screen_row = content_row + (self.cursor_line - self.scroll_row) as u16;
            let cursor_screen_col = content_col + (self.cursor_col - self.scroll_col) as u16;

            if cursor_screen_row >= content_row
                && cursor_screen_row < content_row + content_height
                && cursor_screen_col >= content_col
                && cursor_screen_col < content_col + content_width
            {
                screen.set_cursor(cursor_screen_row, cursor_screen_col);
                screen.set_cursor_visible(true);
            }
        }
    }

    fn draw_border(&self, screen: &mut Screen, state: &AppState, bounds: Rect) {
        let row = bounds.y + 1;
        let col = bounds.x + 1;
        let width = bounds.width;
        let height = bounds.height;

        // Draw border
        screen.draw_box(row, col, width, height, Color::White, Color::Blue);

        // Draw title bar with filename - inverted colors (blue on white)
        let title = format!(" {} ", state.title());
        let title_x = col + (width.saturating_sub(title.len() as u16)) / 2;
        screen.write_str(row, title_x, &title, Color::Blue, Color::White);

        // Draw scroll bars
        self.draw_scrollbars(screen, bounds);
    }

    fn draw_scrollbars(&self, screen: &mut Screen, bounds: Rect) {
        let row = bounds.y + 1;
        let col = bounds.x + 1;
        let width = bounds.width;
        let height = bounds.height;

        // Vertical scrollbar on right edge (inside border)
        let scrollbar_col = col + width - 1;
        let scrollbar_height = height.saturating_sub(2);

        // Draw scrollbar track
        for r in 1..=scrollbar_height {
            screen.set(row + r, scrollbar_col, '░', Color::LightGray, Color::Blue);
        }

        // Draw up/down arrows
        screen.set(row + 1, scrollbar_col, '↑', Color::Black, Color::LightGray);
        screen.set(row + height - 2, scrollbar_col, '↓', Color::Black, Color::LightGray);

        // Draw scrollbar thumb
        // Classic scrollbar: can scroll until last line is at top of screen
        let line_count = self.buffer.line_count().max(1);
        if scrollbar_height > 4 {
            let track_size = scrollbar_height.saturating_sub(2) as usize;
            // thumb_pos maps scroll_row (0..line_count-1) to track position (0..track_size-1)
            let thumb_pos = if line_count > 1 {
                (self.scroll_row.min(line_count - 1) * track_size.saturating_sub(1)) / (line_count - 1)
            } else {
                0
            };
            let thumb_row = row + 2 + thumb_pos as u16;
            screen.set(thumb_row, scrollbar_col, '█', Color::White, Color::Blue);
        }

        // Horizontal scrollbar on bottom edge (inside border)
        let scrollbar_row = row + height - 1;
        let scrollbar_width = width.saturating_sub(2);

        // Draw scrollbar track
        for c in 1..scrollbar_width {
            screen.set(scrollbar_row, col + c, '░', Color::LightGray, Color::Blue);
        }

        // Draw left/right arrows
        screen.set(scrollbar_row, col + 1, '←', Color::Black, Color::LightGray);
        screen.set(scrollbar_row, col + width - 2, '→', Color::Black, Color::LightGray);

        // Draw horizontal scrollbar thumb
        // Classic scrollbar: can scroll until end of longest line is at left of screen
        let max_line_len = self.buffer.max_line_length().max(1);
        if scrollbar_width > 4 {
            let track_size = scrollbar_width.saturating_sub(2) as usize;
            // thumb_pos maps scroll_col (0..max_line_len-1) to track position (0..track_size-1)
            let thumb_pos = if max_line_len > 1 {
                (self.scroll_col.min(max_line_len - 1) * track_size.saturating_sub(1)) / (max_line_len - 1)
            } else {
                0
            };
            let thumb_col = col + 2 + thumb_pos as u16;
            screen.set(scrollbar_row, thumb_col, '█', Color::White, Color::Blue);
        }
    }

    fn draw_line(&self, screen: &mut Screen, row: u16, col: u16, width: u16, line: &str, state: &AppState, line_num: usize) {
        // Check for breakpoint
        let has_bp = state.has_breakpoint(line_num);
        let is_current = state.current_line == Some(line_num);

        // Background color
        let normal_bg = if is_current {
            Color::Cyan
        } else if has_bp {
            Color::Red
        } else {
            Color::Blue
        };

        // Clear line (check selection for each character position)
        for c in 0..width {
            let char_col = self.scroll_col + c as usize;
            let (fg, bg) = if self.is_selected(line_num, char_col) {
                (Color::Black, Color::White)  // Inverted for selection
            } else {
                (Color::Yellow, normal_bg)
            };
            screen.set(row, col + c, ' ', fg, bg);
        }

        // Tokenize and draw with syntax highlighting
        let tokens = tokenize_line(line);
        let mut x = 0usize;

        for token in tokens {
            if x >= self.scroll_col + width as usize {
                break;
            }

            let normal_fg = match token.kind {
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
                if x >= self.scroll_col && x - self.scroll_col < width as usize {
                    let screen_x = col + (x - self.scroll_col) as u16;
                    let (fg, bg) = if self.is_selected(line_num, x) {
                        (Color::Black, Color::White)  // Inverted for selection
                    } else {
                        (normal_fg, normal_bg)
                    };
                    screen.set(row, screen_x, ch, fg, bg);
                }
                x += 1;
            }
        }
    }

    /// Handle input for the editor
    pub fn handle_input(&mut self, event: &crate::input::InputEvent, state: &mut AppState) -> bool {
        use crate::input::InputEvent;

        match event {
            InputEvent::Char(c) => {
                // If there's a selection, delete it first (typing replaces selection)
                if self.has_selection() {
                    self.delete_selection();
                    state.set_modified(true);
                } else if state.editor_mode == EditorMode::Overwrite {
                    // Record deleted char for undo
                    if let Some(line) = self.buffer.line(self.cursor_line) {
                        if self.cursor_col < line.len() {
                            let deleted_char = line.chars().nth(self.cursor_col).unwrap();
                            self.record_undo(UndoAction::Delete {
                                line: self.cursor_line,
                                col: self.cursor_col,
                                text: deleted_char.to_string(),
                            });
                        }
                    }
                    self.buffer.delete_char(self.cursor_line, self.cursor_col);
                }
                // Record insert for undo
                self.record_undo(UndoAction::Insert {
                    line: self.cursor_line,
                    col: self.cursor_col,
                    text: c.to_string(),
                });
                self.buffer.insert_char(self.cursor_line, self.cursor_col, *c);
                self.cursor_col += 1;
                state.set_modified(true);
                true
            }
            InputEvent::Enter => {
                // If there's a selection, delete it first
                if self.has_selection() {
                    self.delete_selection();
                    state.set_modified(true);
                }
                // Record split for undo
                self.record_undo(UndoAction::SplitLine {
                    line: self.cursor_line,
                    col: self.cursor_col,
                });
                let (new_line, new_col) = self.buffer.insert_newline(self.cursor_line, self.cursor_col);
                self.cursor_line = new_line;
                self.cursor_col = new_col;
                state.set_modified(true);
                true
            }
            InputEvent::Backspace => {
                // If there's a selection, delete it instead of single char
                if self.has_selection() {
                    self.delete_selection();
                    state.set_modified(true);
                    return true;
                }
                if self.cursor_col > 0 {
                    // Record delete for undo
                    if let Some(line) = self.buffer.line(self.cursor_line) {
                        if self.cursor_col <= line.len() {
                            let deleted_char = line.chars().nth(self.cursor_col - 1).unwrap_or(' ');
                            self.record_undo(UndoAction::Delete {
                                line: self.cursor_line,
                                col: self.cursor_col - 1,
                                text: deleted_char.to_string(),
                            });
                        }
                    }
                } else if self.cursor_line > 0 {
                    // Recording join for undo
                    let prev_line_len = self.buffer.line(self.cursor_line - 1).map(|l| l.len()).unwrap_or(0);
                    self.record_undo(UndoAction::JoinLines {
                        line: self.cursor_line - 1,
                        col: prev_line_len,
                    });
                }
                let (new_line, new_col) = self.buffer.backspace(self.cursor_line, self.cursor_col);
                self.cursor_line = new_line;
                self.cursor_col = new_col;
                state.set_modified(true);
                true
            }
            InputEvent::Delete => {
                // If there's a selection, delete it instead of single char
                if self.has_selection() {
                    self.delete_selection();
                    state.set_modified(true);
                    return true;
                }
                let line_len = self.buffer.line(self.cursor_line).map(|l| l.len()).unwrap_or(0);
                if self.cursor_col < line_len {
                    // Record delete for undo
                    if let Some(line) = self.buffer.line(self.cursor_line) {
                        let deleted_char = line.chars().nth(self.cursor_col).unwrap_or(' ');
                        self.record_undo(UndoAction::Delete {
                            line: self.cursor_line,
                            col: self.cursor_col,
                            text: deleted_char.to_string(),
                        });
                    }
                    self.buffer.delete_char(self.cursor_line, self.cursor_col);
                } else if self.cursor_line + 1 < self.buffer.line_count() {
                    // Record join for undo
                    self.record_undo(UndoAction::JoinLines {
                        line: self.cursor_line,
                        col: self.cursor_col,
                    });
                    let next_line = self.buffer.lines.remove(self.cursor_line + 1);
                    self.buffer.lines[self.cursor_line].push_str(&next_line);
                }
                state.set_modified(true);
                true
            }
            InputEvent::Tab => {
                // If there's a selection, delete it first
                if self.has_selection() {
                    self.delete_selection();
                    state.set_modified(true);
                }
                // Record tab as insert of 8 spaces
                self.record_undo(UndoAction::Insert {
                    line: self.cursor_line,
                    col: self.cursor_col,
                    text: "        ".to_string(), // 8 spaces
                });
                // Insert spaces (QBasic used 8-space tabs)
                for _ in 0..8 {
                    self.buffer.insert_char(self.cursor_line, self.cursor_col, ' ');
                    self.cursor_col += 1;
                }
                state.set_modified(true);
                true
            }
            InputEvent::CursorUp => {
                self.clear_selection();
                if self.cursor_line > 0 {
                    self.cursor_line -= 1;
                    self.clamp_cursor();
                }
                true
            }
            InputEvent::CursorDown => {
                self.clear_selection();
                if self.cursor_line + 1 < self.buffer.line_count() {
                    self.cursor_line += 1;
                    self.clamp_cursor();
                }
                true
            }
            InputEvent::CursorLeft => {
                self.clear_selection();
                if self.cursor_col > 0 {
                    self.cursor_col -= 1;
                } else if self.cursor_line > 0 {
                    self.cursor_line -= 1;
                    self.cursor_col = self.buffer.line(self.cursor_line).map(|l| l.len()).unwrap_or(0);
                }
                true
            }
            InputEvent::CursorRight => {
                self.clear_selection();
                let line_len = self.buffer.line(self.cursor_line).map(|l| l.len()).unwrap_or(0);
                if self.cursor_col < line_len {
                    self.cursor_col += 1;
                } else if self.cursor_line + 1 < self.buffer.line_count() {
                    self.cursor_line += 1;
                    self.cursor_col = 0;
                }
                true
            }
            InputEvent::Home => {
                self.clear_selection();
                self.cursor_col = 0;
                true
            }
            InputEvent::End => {
                self.clear_selection();
                self.cursor_col = self.buffer.line(self.cursor_line).map(|l| l.len()).unwrap_or(0);
                true
            }
            InputEvent::PageUp => {
                self.clear_selection();
                self.cursor_line = self.cursor_line.saturating_sub(20);
                self.clamp_cursor();
                true
            }
            InputEvent::PageDown => {
                self.clear_selection();
                self.cursor_line = (self.cursor_line + 20).min(self.buffer.line_count().saturating_sub(1));
                self.clamp_cursor();
                true
            }
            // Shift+Arrow keys for keyboard selection
            InputEvent::ShiftUp => {
                // Start or extend selection
                if !self.has_selection() {
                    self.selection_start = Some((self.cursor_line, self.cursor_col));
                }
                if self.cursor_line > 0 {
                    self.cursor_line -= 1;
                    self.clamp_cursor();
                }
                self.selection_end = Some((self.cursor_line, self.cursor_col));
                true
            }
            InputEvent::ShiftDown => {
                if !self.has_selection() {
                    self.selection_start = Some((self.cursor_line, self.cursor_col));
                }
                if self.cursor_line + 1 < self.buffer.line_count() {
                    self.cursor_line += 1;
                    self.clamp_cursor();
                }
                self.selection_end = Some((self.cursor_line, self.cursor_col));
                true
            }
            InputEvent::ShiftLeft => {
                if !self.has_selection() {
                    self.selection_start = Some((self.cursor_line, self.cursor_col));
                }
                if self.cursor_col > 0 {
                    self.cursor_col -= 1;
                } else if self.cursor_line > 0 {
                    self.cursor_line -= 1;
                    self.cursor_col = self.buffer.line(self.cursor_line).map(|l| l.len()).unwrap_or(0);
                }
                self.selection_end = Some((self.cursor_line, self.cursor_col));
                true
            }
            InputEvent::ShiftRight => {
                if !self.has_selection() {
                    self.selection_start = Some((self.cursor_line, self.cursor_col));
                }
                let line_len = self.buffer.line(self.cursor_line).map(|l| l.len()).unwrap_or(0);
                if self.cursor_col < line_len {
                    self.cursor_col += 1;
                } else if self.cursor_line + 1 < self.buffer.line_count() {
                    self.cursor_line += 1;
                    self.cursor_col = 0;
                }
                self.selection_end = Some((self.cursor_line, self.cursor_col));
                true
            }
            InputEvent::ShiftHome => {
                if !self.has_selection() {
                    self.selection_start = Some((self.cursor_line, self.cursor_col));
                }
                self.cursor_col = 0;
                self.selection_end = Some((self.cursor_line, self.cursor_col));
                true
            }
            InputEvent::ShiftEnd => {
                if !self.has_selection() {
                    self.selection_start = Some((self.cursor_line, self.cursor_col));
                }
                self.cursor_col = self.buffer.line(self.cursor_line).map(|l| l.len()).unwrap_or(0);
                self.selection_end = Some((self.cursor_line, self.cursor_col));
                true
            }
            InputEvent::ShiftSpace => {
                // Shift+Space extends selection like Shift+Right
                if !self.has_selection() {
                    self.selection_start = Some((self.cursor_line, self.cursor_col));
                }
                let line_len = self.buffer.line(self.cursor_line).map(|l| l.len()).unwrap_or(0);
                if self.cursor_col < line_len {
                    self.cursor_col += 1;
                } else if self.cursor_line + 1 < self.buffer.line_count() {
                    self.cursor_line += 1;
                    self.cursor_col = 0;
                }
                self.selection_end = Some((self.cursor_line, self.cursor_col));
                true
            }
            InputEvent::CtrlA => {
                self.select_all();
                true
            }
            InputEvent::Insert => {
                state.editor_mode = match state.editor_mode {
                    EditorMode::Insert => EditorMode::Overwrite,
                    EditorMode::Overwrite => EditorMode::Insert,
                };
                true
            }
            InputEvent::F9 => {
                state.toggle_breakpoint(self.cursor_line);
                true
            }
            InputEvent::ScrollUp { .. } => {
                // Scroll up 3 lines
                self.scroll_row = self.scroll_row.saturating_sub(3);
                true
            }
            InputEvent::ScrollDown { .. } => {
                // Scroll down 3 lines
                self.scroll_row += 3;
                true
            }
            _ => false,
        }
    }

    fn clamp_cursor(&mut self) {
        let line_len = self.buffer.line(self.cursor_line).map(|l| l.len()).unwrap_or(0);
        self.cursor_col = self.cursor_col.min(line_len);
    }

    /// Adjust scroll position to keep cursor visible
    pub fn ensure_cursor_visible(&mut self, visible_lines: usize, visible_cols: usize) {
        // Vertical scrolling
        if self.cursor_line < self.scroll_row {
            self.scroll_row = self.cursor_line;
        } else if self.cursor_line >= self.scroll_row + visible_lines {
            self.scroll_row = self.cursor_line - visible_lines + 1;
        }

        // Horizontal scrolling
        if self.cursor_col < self.scroll_col {
            self.scroll_col = self.cursor_col;
        } else if self.cursor_col >= self.scroll_col + visible_cols {
            self.scroll_col = self.cursor_col - visible_cols + 1;
        }
    }

    /// Load content from string
    pub fn load(&mut self, content: &str) {
        self.buffer = TextBuffer::from_string(content);
        self.cursor_line = 0;
        self.cursor_col = 0;
        self.scroll_row = 0;
        self.scroll_col = 0;
        self.undo_stack.clear();
        self.redo_stack.clear();
        self.clear_selection();
    }

    /// Get content as string
    pub fn content(&self) -> String {
        self.buffer.to_string()
    }

    /// Clear the editor
    pub fn clear(&mut self) {
        self.buffer.clear();
        self.cursor_line = 0;
        self.cursor_col = 0;
        self.scroll_row = 0;
        self.scroll_col = 0;
        self.undo_stack.clear();
        self.redo_stack.clear();
        self.clear_selection();
    }

    /// Find text in the buffer starting from the cursor position
    /// Returns (line, col) of the match, or None if not found
    pub fn find_text(&self, search: &str, case_sensitive: bool, whole_word: bool) -> Option<(usize, usize)> {
        self.find_text_from(search, self.cursor_line, self.cursor_col + 1, case_sensitive, whole_word)
    }

    /// Find text starting from a specific position
    pub fn find_text_from(&self, search: &str, start_line: usize, start_col: usize, case_sensitive: bool, whole_word: bool) -> Option<(usize, usize)> {
        if search.is_empty() {
            return None;
        }

        let search_text = if case_sensitive {
            search.to_string()
        } else {
            search.to_uppercase()
        };

        // Search from start position to end
        for line_num in start_line..self.buffer.line_count() {
            if let Some(line) = self.buffer.line(line_num) {
                let line_text = if case_sensitive {
                    line.to_string()
                } else {
                    line.to_uppercase()
                };

                let search_start = if line_num == start_line { start_col } else { 0 };

                if let Some(pos) = line_text[search_start..].find(&search_text) {
                    let col = search_start + pos;

                    if whole_word {
                        // Check word boundaries
                        let before_ok = col == 0 || !line.chars().nth(col - 1).map(|c| c.is_alphanumeric() || c == '_').unwrap_or(false);
                        let after_ok = col + search.len() >= line.len() || !line.chars().nth(col + search.len()).map(|c| c.is_alphanumeric() || c == '_').unwrap_or(false);
                        if before_ok && after_ok {
                            return Some((line_num, col));
                        } else {
                            // Keep searching on this line
                            let result = self.find_text_from(search, line_num, col + 1, case_sensitive, whole_word);
                            if result.is_some() {
                                return result;
                            }
                        }
                    } else {
                        return Some((line_num, col));
                    }
                }
            }
        }

        // Wrap around to beginning
        for line_num in 0..=start_line {
            if let Some(line) = self.buffer.line(line_num) {
                let line_text = if case_sensitive {
                    line.to_string()
                } else {
                    line.to_uppercase()
                };

                let end_col = if line_num == start_line { start_col } else { line.len() };

                if let Some(pos) = line_text[..end_col].find(&search_text) {
                    if whole_word {
                        let before_ok = pos == 0 || !line.chars().nth(pos - 1).map(|c| c.is_alphanumeric() || c == '_').unwrap_or(false);
                        let after_ok = pos + search.len() >= line.len() || !line.chars().nth(pos + search.len()).map(|c| c.is_alphanumeric() || c == '_').unwrap_or(false);
                        if before_ok && after_ok {
                            return Some((line_num, pos));
                        }
                    } else {
                        return Some((line_num, pos));
                    }
                }
            }
        }

        None
    }

    /// Go to a specific line and column, selecting the text of given length
    pub fn go_to_and_select(&mut self, line: usize, col: usize, length: usize) {
        self.cursor_line = line.min(self.buffer.line_count().saturating_sub(1));
        self.cursor_col = col;
        self.selection_start = Some((line, col));
        self.selection_end = Some((line, col + length));
    }

    /// Go to a specific line number
    pub fn go_to_line(&mut self, line: usize) {
        self.cursor_line = line.saturating_sub(1).min(self.buffer.line_count().saturating_sub(1));
        self.cursor_col = 0;
        self.clear_selection();
    }

    /// Replace the current selection with new text
    /// Returns true if replacement was made
    pub fn replace_selection(&mut self, new_text: &str) -> bool {
        if !self.has_selection() {
            return false;
        }

        self.delete_selection();
        self.insert_text(new_text);
        true
    }

    /// Replace all occurrences of search text with replacement text
    /// Returns the number of replacements made
    pub fn replace_all(&mut self, search: &str, replace: &str, case_sensitive: bool, whole_word: bool) -> usize {
        if search.is_empty() {
            return 0;
        }

        let mut count = 0;

        // Start from beginning
        self.cursor_line = 0;
        self.cursor_col = 0;

        // Keep finding and replacing until no more matches
        while let Some((line, col)) = self.find_text_from(search, self.cursor_line, self.cursor_col, case_sensitive, whole_word) {
            // Select the found text
            self.cursor_line = line;
            self.cursor_col = col;
            self.selection_start = Some((line, col));
            self.selection_end = Some((line, col + search.len()));

            // Replace it
            self.delete_selection();
            self.insert_text(replace);

            count += 1;

            // Prevent infinite loop if replace contains search
            if replace.contains(search) && !case_sensitive {
                self.cursor_col = col + replace.len();
            }
        }

        count
    }
}

impl Default for Editor {
    fn default() -> Self {
        Self::new()
    }
}

/// Token types for syntax highlighting
#[derive(Clone, Copy, Debug)]
enum TokenKind {
    Keyword,
    String,
    Number,
    Comment,
    Operator,
    Identifier,
    Punctuation,
    Whitespace,
}

struct Token<'a> {
    kind: TokenKind,
    text: &'a str,
}

/// Simple tokenizer for BASIC syntax highlighting
fn tokenize_line(line: &str) -> Vec<Token<'_>> {
    let mut tokens = Vec::new();
    let chars: Vec<char> = line.chars().collect();
    let mut i = 0;

    while i < chars.len() {
        let start = i;

        // Whitespace
        if chars[i].is_whitespace() {
            while i < chars.len() && chars[i].is_whitespace() {
                i += 1;
            }
            tokens.push(Token {
                kind: TokenKind::Whitespace,
                text: &line[start..i],
            });
            continue;
        }

        // Comment (REM or ')
        if chars[i] == '\'' || (i + 3 <= chars.len() && line[i..].to_uppercase().starts_with("REM") && (i + 3 >= chars.len() || !chars[i + 3].is_alphanumeric())) {
            tokens.push(Token {
                kind: TokenKind::Comment,
                text: &line[start..],
            });
            break;
        }

        // String
        if chars[i] == '"' {
            i += 1;
            while i < chars.len() && chars[i] != '"' {
                i += 1;
            }
            if i < chars.len() {
                i += 1; // Include closing quote
            }
            tokens.push(Token {
                kind: TokenKind::String,
                text: &line[start..i],
            });
            continue;
        }

        // Number
        if chars[i].is_ascii_digit() || (chars[i] == '.' && i + 1 < chars.len() && chars[i + 1].is_ascii_digit()) {
            while i < chars.len() && (chars[i].is_ascii_digit() || chars[i] == '.' || chars[i] == 'E' || chars[i] == 'e' || chars[i] == '+' || chars[i] == '-' || chars[i] == '#' || chars[i] == '!') {
                i += 1;
            }
            tokens.push(Token {
                kind: TokenKind::Number,
                text: &line[start..i],
            });
            continue;
        }

        // Identifier or keyword
        if chars[i].is_alphabetic() || chars[i] == '_' {
            while i < chars.len() && (chars[i].is_alphanumeric() || chars[i] == '_' || chars[i] == '$' || chars[i] == '%' || chars[i] == '!' || chars[i] == '#' || chars[i] == '&') {
                i += 1;
            }
            let word = &line[start..i];
            let kind = if KEYWORDS.contains(&word.to_uppercase().as_str()) {
                TokenKind::Keyword
            } else {
                TokenKind::Identifier
            };
            tokens.push(Token { kind, text: word });
            continue;
        }

        // Operators
        if "+-*/\\^=<>".contains(chars[i]) {
            // Check for compound operators
            if i + 1 < chars.len() {
                let two = &line[i..i + 2];
                if two == "<>" || two == "<=" || two == ">=" {
                    i += 2;
                    tokens.push(Token {
                        kind: TokenKind::Operator,
                        text: &line[start..i],
                    });
                    continue;
                }
            }
            i += 1;
            tokens.push(Token {
                kind: TokenKind::Operator,
                text: &line[start..i],
            });
            continue;
        }

        // Punctuation
        i += 1;
        tokens.push(Token {
            kind: TokenKind::Punctuation,
            text: &line[start..i],
        });
    }

    tokens
}
