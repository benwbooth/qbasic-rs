//! Double-buffered screen rendering system
//! Minimizes flicker by only updating changed cells

use crate::terminal::{Color, Terminal, CursorStyle};
use std::io;

/// A single cell on the screen
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct Cell {
    pub ch: char,
    pub fg: Color,
    pub bg: Color,
}

impl Default for Cell {
    fn default() -> Self {
        Self {
            ch: ' ',
            fg: Color::LightGray,
            bg: Color::Black,
        }
    }
}

impl Cell {
    pub fn new(ch: char, fg: Color, bg: Color) -> Self {
        Self { ch, fg, bg }
    }
}

/// Double-buffered screen
pub struct Screen {
    width: u16,
    height: u16,
    front: Vec<Cell>,  // Currently displayed
    back: Vec<Cell>,   // Being drawn to
    cursor_row: u16,
    cursor_col: u16,
    cursor_visible: bool,
    cursor_style: CursorStyle,
    /// Pending sixel graphics data (if any)
    sixel_data: Option<String>,
}

impl Screen {
    /// Create a new screen with given dimensions
    pub fn new(width: u16, height: u16) -> Self {
        let size = (width as usize) * (height as usize);
        let default_cell = Cell::default();

        Self {
            width,
            height,
            front: vec![Cell::new('\0', Color::Black, Color::Black); size], // Force initial draw
            back: vec![default_cell; size],
            cursor_row: 1,
            cursor_col: 1,
            cursor_visible: true,
            cursor_style: CursorStyle::BlinkingUnderline, // Default to blinking underline
            sixel_data: None,
        }
    }

    /// Get screen dimensions
    pub fn size(&self) -> (u16, u16) {
        (self.width, self.height)
    }

    /// Resize the screen
    pub fn resize(&mut self, width: u16, height: u16) {
        let size = (width as usize) * (height as usize);
        let default_cell = Cell::default();

        self.width = width;
        self.height = height;
        self.front = vec![Cell::new('\0', Color::Black, Color::Black); size];
        self.back = vec![default_cell; size];
    }

    /// Convert row/col to buffer index (1-based coordinates)
    fn index(&self, row: u16, col: u16) -> Option<usize> {
        if row >= 1 && row <= self.height && col >= 1 && col <= self.width {
            Some(((row - 1) as usize) * (self.width as usize) + ((col - 1) as usize))
        } else {
            None
        }
    }

    /// Set a cell in the back buffer
    pub fn set(&mut self, row: u16, col: u16, ch: char, fg: Color, bg: Color) {
        if let Some(idx) = self.index(row, col) {
            self.back[idx] = Cell::new(ch, fg, bg);
        }
    }

    /// Get a cell from the back buffer
    #[allow(dead_code)]
    pub fn get(&self, row: u16, col: u16) -> Option<Cell> {
        self.index(row, col).map(|idx| self.back[idx])
    }

    /// Write a string to the back buffer starting at given position
    pub fn write_str(&mut self, row: u16, col: u16, s: &str, fg: Color, bg: Color) {
        let mut c = col;
        for ch in s.chars() {
            if c > self.width {
                break;
            }
            self.set(row, c, ch, fg, bg);
            c += 1;
        }
    }

    /// Fill a rectangle with a character
    pub fn fill(&mut self, row: u16, col: u16, width: u16, height: u16, ch: char, fg: Color, bg: Color) {
        for r in row..row.saturating_add(height) {
            for c in col..col.saturating_add(width) {
                self.set(r, c, ch, fg, bg);
            }
        }
    }

    /// Clear the entire back buffer with default cell
    #[allow(dead_code)]
    pub fn clear(&mut self) {
        let default_cell = Cell::default();
        self.back.fill(default_cell);
    }

    /// Clear with specific colors
    pub fn clear_with(&mut self, fg: Color, bg: Color) {
        let cell = Cell::new(' ', fg, bg);
        self.back.fill(cell);
    }

    /// Draw a single-line box
    pub fn draw_box(&mut self, row: u16, col: u16, width: u16, height: u16, fg: Color, bg: Color) {
        if width < 2 || height < 2 {
            return;
        }

        // Corners
        self.set(row, col, '┌', fg, bg);
        self.set(row, col + width - 1, '┐', fg, bg);
        self.set(row + height - 1, col, '└', fg, bg);
        self.set(row + height - 1, col + width - 1, '┘', fg, bg);

        // Top and bottom edges
        for c in 1..width - 1 {
            self.set(row, col + c, '─', fg, bg);
            self.set(row + height - 1, col + c, '─', fg, bg);
        }

        // Left and right edges
        for r in 1..height - 1 {
            self.set(row + r, col, '│', fg, bg);
            self.set(row + r, col + width - 1, '│', fg, bg);
        }

        // Fill interior
        for r in 1..height - 1 {
            for c in 1..width - 1 {
                self.set(row + r, col + c, ' ', fg, bg);
            }
        }
    }

    /// Draw a double-line box (for dialogs)
    #[allow(dead_code)]
    pub fn draw_double_box(&mut self, row: u16, col: u16, width: u16, height: u16, fg: Color, bg: Color) {
        if width < 2 || height < 2 {
            return;
        }

        // Corners
        self.set(row, col, '╔', fg, bg);
        self.set(row, col + width - 1, '╗', fg, bg);
        self.set(row + height - 1, col, '╚', fg, bg);
        self.set(row + height - 1, col + width - 1, '╝', fg, bg);

        // Top and bottom edges
        for c in 1..width - 1 {
            self.set(row, col + c, '═', fg, bg);
            self.set(row + height - 1, col + c, '═', fg, bg);
        }

        // Left and right edges
        for r in 1..height - 1 {
            self.set(row + r, col, '║', fg, bg);
            self.set(row + r, col + width - 1, '║', fg, bg);
        }

        // Fill interior
        for r in 1..height - 1 {
            for c in 1..width - 1 {
                self.set(row + r, col + c, ' ', fg, bg);
            }
        }
    }

    /// Draw a shadow effect (DOS style - dark area to right and below)
    pub fn draw_shadow(&mut self, row: u16, col: u16, width: u16, height: u16) {
        // Right shadow (2 chars wide)
        for r in 1..=height {
            for c in 0..2 {
                if let Some(idx) = self.index(row + r, col + width + c) {
                    // Preserve the character but darken
                    let cell = &mut self.back[idx];
                    cell.fg = Color::DarkGray;
                    cell.bg = Color::Black;
                }
            }
        }

        // Bottom shadow
        for c in 2..width + 2 {
            if let Some(idx) = self.index(row + height, col + c) {
                let cell = &mut self.back[idx];
                cell.fg = Color::DarkGray;
                cell.bg = Color::Black;
            }
        }
    }

    /// Set cursor position
    pub fn set_cursor(&mut self, row: u16, col: u16) {
        self.cursor_row = row;
        self.cursor_col = col;
    }

    /// Get cursor position
    #[allow(dead_code)]
    pub fn cursor(&self) -> (u16, u16) {
        (self.cursor_row, self.cursor_col)
    }

    /// Set cursor visibility
    pub fn set_cursor_visible(&mut self, visible: bool) {
        self.cursor_visible = visible;
    }

    /// Set cursor style
    pub fn set_cursor_style(&mut self, style: CursorStyle) {
        self.cursor_style = style;
    }

    /// Set sixel graphics data for next flush
    ///
    /// When set, flush() will render sixel graphics instead of the cell buffer.
    pub fn set_sixel(&mut self, sixel_data: String) {
        self.sixel_data = Some(sixel_data);
    }

    /// Clear sixel graphics data
    pub fn clear_sixel(&mut self) {
        self.sixel_data = None;
    }

    /// Apply mouse cursor effect at given position (orange background, inverted foreground)
    pub fn apply_mouse_cursor(&mut self, row: u16, col: u16) {
        if row == 0 || col == 0 {
            return; // Invalid position
        }
        if let Some(idx) = self.index(row, col) {
            let cell = self.back[idx];
            // Invert the foreground color and use orange/brown background
            let inverted_fg = Self::invert_color(cell.fg);
            self.back[idx] = Cell::new(cell.ch, inverted_fg, Color::Brown);
        }
    }

    /// Invert a color for cursor effect
    fn invert_color(color: Color) -> Color {
        match color {
            Color::Black => Color::White,
            Color::White => Color::Black,
            Color::Blue => Color::Yellow,
            Color::Yellow => Color::Blue,
            Color::Green => Color::LightMagenta,
            Color::LightMagenta => Color::Green,
            Color::Cyan => Color::LightRed,
            Color::LightRed => Color::Cyan,
            Color::Red => Color::LightCyan,
            Color::LightCyan => Color::Red,
            Color::Magenta => Color::LightGreen,
            Color::LightGreen => Color::Magenta,
            Color::Brown => Color::LightBlue,
            Color::LightBlue => Color::Brown,
            Color::LightGray => Color::DarkGray,
            Color::DarkGray => Color::LightGray,
        }
    }

    /// Flush changes to the terminal (only updates changed cells)
    pub fn flush(&mut self, term: &mut Terminal) -> io::Result<()> {
        // Check for sixel graphics mode
        if let Some(sixel_data) = self.sixel_data.take() {
            return self.flush_sixel(term, &sixel_data);
        }

        let mut last_fg = Color::Black;
        let mut last_bg = Color::Black;
        let mut last_row: u16 = 0;
        let mut last_col: u16 = 0;
        let mut need_move = true;

        for row in 1..=self.height {
            for col in 1..=self.width {
                let idx = self.index(row, col).unwrap();
                let front = self.front[idx];
                let back = self.back[idx];

                if front != back {
                    // Need to update this cell

                    // Move cursor if needed
                    if need_move || row != last_row || col != last_col + 1 {
                        term.goto(row, col)?;
                    }

                    // Change colors if needed
                    if back.fg != last_fg || back.bg != last_bg {
                        term.set_colors(back.fg, back.bg)?;
                        last_fg = back.fg;
                        last_bg = back.bg;
                    }

                    // Write character
                    term.write_char(back.ch)?;

                    // Update front buffer
                    self.front[idx] = back;

                    last_row = row;
                    last_col = col;
                    need_move = false;
                }
            }
        }

        // Position cursor
        if self.cursor_visible {
            term.goto(self.cursor_row, self.cursor_col)?;
            term.set_cursor_style(self.cursor_style)?;
            term.show_cursor()?;
        } else {
            term.hide_cursor()?;
        }

        term.flush()?;
        Ok(())
    }

    /// Force full redraw on next flush
    pub fn invalidate(&mut self) {
        for cell in &mut self.front {
            cell.ch = '\0';
        }
    }

    /// Flush sixel graphics with optional text overlay
    ///
    /// Outputs sixel graphics first, then overlays any non-empty text cells.
    /// This allows mixing graphics and text (for INPUT prompts, scores, etc.)
    pub fn flush_sixel(&mut self, term: &mut Terminal, sixel_data: &str) -> io::Result<()> {
        // Position at top-left and output sixel graphics
        term.goto(1, 1)?;
        term.write_raw(sixel_data)?;

        // Now overlay text cells that have content
        // This allows PRINT/LOCATE/INPUT to work in graphics mode
        let mut last_fg = Color::Black;
        let mut last_bg = Color::Black;

        for row in 1..=self.height {
            for col in 1..=self.width {
                if let Some(idx) = self.index(row, col) {
                    let cell = self.back[idx];
                    // Only draw non-space characters (text overlay)
                    if cell.ch != ' ' && cell.ch != '\0' {
                        term.goto(row, col)?;
                        if cell.fg != last_fg || cell.bg != last_bg {
                            term.set_colors(cell.fg, cell.bg)?;
                            last_fg = cell.fg;
                            last_bg = cell.bg;
                        }
                        term.write_char(cell.ch)?;
                    }
                }
            }
        }

        // Position cursor if visible
        if self.cursor_visible {
            term.goto(self.cursor_row, self.cursor_col)?;
            term.set_cursor_style(self.cursor_style)?;
            term.show_cursor()?;
        } else {
            term.hide_cursor()?;
        }

        term.flush()?;

        // Invalidate front buffer since we bypassed normal diff rendering
        self.invalidate();

        Ok(())
    }
}

/// Horizontal line drawing
#[allow(dead_code)]
pub fn hline(screen: &mut Screen, row: u16, col: u16, len: u16, ch: char, fg: Color, bg: Color) {
    for c in 0..len {
        screen.set(row, col + c, ch, fg, bg);
    }
}

/// Vertical line drawing
#[allow(dead_code)]
pub fn vline(screen: &mut Screen, row: u16, col: u16, len: u16, ch: char, fg: Color, bg: Color) {
    for r in 0..len {
        screen.set(row + r, col, ch, fg, bg);
    }
}
