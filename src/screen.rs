//! Double-buffered screen rendering system
#![allow(dead_code)]
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

#[derive(Clone, Copy, Debug)]
struct ClipRect {
    row: u16,
    col: u16,
    width: u16,
    height: u16,
}

impl ClipRect {
    fn contains(&self, row: u16, col: u16) -> bool {
        let row_end = self.row.saturating_add(self.height);
        let col_end = self.col.saturating_add(self.width);
        row >= self.row && row < row_end && col >= self.col && col < col_end
    }

    fn end_row(self) -> u32 {
        self.row as u32 + self.height as u32
    }

    fn end_col(self) -> u32 {
        self.col as u32 + self.width as u32
    }

    fn intersect(self, other: ClipRect) -> Option<ClipRect> {
        let row = self.row.max(other.row);
        let col = self.col.max(other.col);
        let end_row = self.end_row().min(other.end_row());
        let end_col = self.end_col().min(other.end_col());
        let row_u = row as u32;
        let col_u = col as u32;
        if end_row <= row_u || end_col <= col_u {
            return None;
        }
        Some(ClipRect {
            row,
            col,
            width: (end_col - col_u) as u16,
            height: (end_row - row_u) as u16,
        })
    }
}

/// A single sixel update region
#[derive(Clone)]
pub struct SixelUpdate {
    /// The sixel data
    pub data: String,
    /// X position in pixels
    pub x: u32,
    /// Y position in pixels
    pub y: u32,
    /// Width in pixels
    pub width: u32,
    /// Height in pixels
    pub height: u32,
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
    /// Pending sixel updates (for differential rendering)
    sixel_updates: Vec<SixelUpdate>,
    /// Last sixel generation rendered
    last_sixel_generation: u64,
    /// Current sixel generation
    sixel_generation: u64,
    /// Sixel position row offset (0-based)
    sixel_row: u16,
    /// Sixel position column offset (0-based)
    sixel_col: u16,
    /// Sixel rendered size in character cells (for border filling)
    sixel_cols: u16,
    sixel_rows: u16,
    /// Previous sixel position/size for change detection
    prev_sixel_row: u16,
    prev_sixel_col: u16,
    prev_sixel_rows: u16,
    prev_sixel_cols: u16,
    /// Whether screen needs clearing before next sixel output
    sixel_needs_clear: bool,
    /// Whether we're currently in sixel graphics mode
    in_sixel_mode: bool,
    /// Character cell width in pixels (for positioning partial sixels)
    char_width: u32,
    /// Character cell height in pixels
    char_height: u32,
    clip_stack: Vec<ClipRect>,
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
            sixel_updates: Vec::new(),
            last_sixel_generation: u64::MAX, // Force first render
            sixel_generation: 0,
            sixel_row: 0,
            sixel_col: 0,
            sixel_cols: 0,
            sixel_rows: 0,
            prev_sixel_row: u16::MAX, // Force border fill on first frame
            prev_sixel_col: u16::MAX,
            prev_sixel_rows: 0,
            prev_sixel_cols: 0,
            sixel_needs_clear: true, // Clear on first sixel output
            in_sixel_mode: false,
            char_width: 8,
            char_height: 16,
            clip_stack: Vec::new(),
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
        self.clip_stack.clear();
        self.sixel_updates.clear();
        self.last_sixel_generation = u64::MAX; // Force sixel re-render on resize
        self.sixel_needs_clear = true; // Clear screen on next sixel output
    }

    /// Set the character cell size in pixels (for sixel positioning)
    pub fn set_char_size(&mut self, width: u32, height: u32) {
        if width > 0 {
            self.char_width = width;
        }
        if height > 0 {
            self.char_height = height;
        }
    }

    /// Get the character cell size in pixels
    pub fn char_size(&self) -> (u32, u32) {
        (self.char_width, self.char_height)
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
        if !self.in_clip(row, col) {
            return;
        }
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

    /// Draw a horizontal rule with T-connectors at the edges
    /// Used for dividing sections within a box
    pub fn draw_hrule(&mut self, row: u16, col: u16, width: u16, fg: Color, bg: Color) {
        if width < 2 {
            return;
        }
        self.set(row, col, '├', fg, bg);
        for c in 1..width - 1 {
            self.set(row, col + c, '─', fg, bg);
        }
        self.set(row, col + width - 1, '┤', fg, bg);
    }

    /// Draw a vertical rule (single character separator)
    pub fn draw_vrule(&mut self, row: u16, col: u16, fg: Color, bg: Color) {
        self.set(row, col, '│', fg, bg);
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
                let rr = row + r;
                let cc = col + width + c;
                if !self.in_clip(rr, cc) {
                    continue;
                }
                if let Some(idx) = self.index(rr, cc) {
                    // Preserve the character but darken
                    let cell = &mut self.back[idx];
                    cell.fg = Color::DarkGray;
                    cell.bg = Color::Black;
                }
            }
        }

        // Bottom shadow
        for c in 2..width + 2 {
            let rr = row + height;
            let cc = col + c;
            if !self.in_clip(rr, cc) {
                continue;
            }
            if let Some(idx) = self.index(rr, cc) {
                let cell = &mut self.back[idx];
                cell.fg = Color::DarkGray;
                cell.bg = Color::Black;
            }
        }
    }

    /// Push a clipping rectangle (intersected with any existing clip)
    pub fn push_clip(&mut self, row: u16, col: u16, width: u16, height: u16) {
        let rect = ClipRect {
            row,
            col,
            width,
            height,
        };
        let next = if let Some(current) = self.clip_stack.last().copied() {
            current.intersect(rect).unwrap_or(ClipRect {
                row,
                col,
                width: 0,
                height: 0,
            })
        } else {
            rect
        };
        self.clip_stack.push(next);
    }

    /// Pop the most recent clipping rectangle
    pub fn pop_clip(&mut self) {
        self.clip_stack.pop();
    }

    fn in_clip(&self, row: u16, col: u16) -> bool {
        match self.clip_stack.last() {
            Some(rect) => rect.contains(row, col),
            None => true,
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
    /// The generation is used to detect if the sixel has changed since last render.
    pub fn set_sixel(&mut self, sixel_data: String, generation: u64) {
        self.sixel_data = Some(sixel_data);
        self.sixel_generation = generation;
    }

    /// Clear sixel graphics data and exit sixel mode
    pub fn clear_sixel(&mut self) {
        self.sixel_data = None;
        self.sixel_updates.clear();
        self.in_sixel_mode = false;
        // Reset sixel generation to force fresh render next time
        self.sixel_generation = 0;
        self.last_sixel_generation = 0;
        // Request clear on next sixel output (in case we re-enter graphics mode)
        self.sixel_needs_clear = true;
    }

    /// Add a sixel update for differential rendering
    /// x, y are in pixels; width, height are in pixels
    pub fn add_sixel_update(&mut self, data: String, x: u32, y: u32, width: u32, height: u32, generation: u64) {
        self.sixel_updates.push(SixelUpdate { data, x, y, width, height });
        self.sixel_generation = generation;
        self.in_sixel_mode = true;
    }

    /// Check if we're currently in sixel graphics mode
    pub fn is_sixel_mode(&self) -> bool {
        self.in_sixel_mode
    }

    /// Set sixel position offset and size (in character cells, 0-based position)
    pub fn set_sixel_position(&mut self, row: u16, col: u16, rows: u16, cols: u16) {
        self.sixel_row = row;
        self.sixel_col = col;
        self.sixel_rows = rows;
        self.sixel_cols = cols;
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
        // Check for sixel graphics mode - prefer sixel_updates (differential) over sixel_data (full)
        if !self.sixel_updates.is_empty() {
            let updates = std::mem::take(&mut self.sixel_updates);
            self.last_sixel_generation = self.sixel_generation;
            return self.flush_sixel_updates(term, &updates);
        }

        // If we're in sixel mode but have no updates, just do text overlay without touching sixel
        if self.in_sixel_mode {
            return self.flush_text_overlay_only(term);
        }

        // Legacy full sixel mode
        if let Some(sixel_data) = self.sixel_data.take() {
            // Only output sixel if it changed or screen needs refresh
            let sixel_changed = self.sixel_generation != self.last_sixel_generation
                || self.sixel_needs_clear
                || self.sixel_row != self.prev_sixel_row
                || self.sixel_col != self.prev_sixel_col;

            self.last_sixel_generation = self.sixel_generation;
            return self.flush_sixel(term, &sixel_data, sixel_changed);
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
        // Note: Don't reset sixel generation here - resize() handles that case
    }

    /// Request screen clear before next sixel output
    /// Call this when entering graphics mode or when graphics mode changes
    pub fn request_sixel_clear(&mut self) {
        self.sixel_needs_clear = true;
    }

    /// Flush sixel graphics with optional text overlay
    ///
    /// Outputs sixel graphics first, then overlays any non-empty text cells.
    /// This allows mixing graphics and text (for INPUT prompts, scores, etc.)
    ///
    /// If `output_sixel` is false, skips the sixel output (used when graphics haven't changed)
    pub fn flush_sixel(&mut self, term: &mut Terminal, sixel_data: &str, output_sixel: bool) -> io::Result<()> {
        // Check if sixel position/size changed - if so, we need to redraw borders
        let position_changed = self.sixel_row != self.prev_sixel_row
            || self.sixel_col != self.prev_sixel_col
            || self.sixel_rows != self.prev_sixel_rows
            || self.sixel_cols != self.prev_sixel_cols;

        // On first frame or when sixel position changes, fill borders with black
        if self.sixel_needs_clear || position_changed {
            term.set_colors(Color::LightGray, Color::Black)?;

            if self.sixel_needs_clear {
                term.clear()?;
                self.sixel_needs_clear = false;
            }

            // Fill border areas with black (areas not covered by sixel)
            let sixel_end_row = self.sixel_row.saturating_add(self.sixel_rows);
            let sixel_end_col = self.sixel_col.saturating_add(self.sixel_cols);

            // Top border (rows before sixel)
            for r in 1..=self.sixel_row {
                term.goto(r, 1)?;
                for _ in 1..=self.width {
                    term.write_char(' ')?;
                }
            }

            // Bottom border (rows after sixel)
            for r in (sixel_end_row + 1)..=self.height {
                term.goto(r, 1)?;
                for _ in 1..=self.width {
                    term.write_char(' ')?;
                }
            }

            // Left and right borders (sides of sixel area)
            for r in (self.sixel_row + 1)..=sixel_end_row.min(self.height) {
                // Left border
                if self.sixel_col > 0 {
                    term.goto(r, 1)?;
                    for _ in 1..=self.sixel_col {
                        term.write_char(' ')?;
                    }
                }
                // Right border
                if sixel_end_col < self.width {
                    term.goto(r, sixel_end_col + 1)?;
                    for _ in (sixel_end_col + 1)..=self.width {
                        term.write_char(' ')?;
                    }
                }
            }

            // Remember current position/size
            self.prev_sixel_row = self.sixel_row;
            self.prev_sixel_col = self.sixel_col;
            self.prev_sixel_rows = self.sixel_rows;
            self.prev_sixel_cols = self.sixel_cols;
        }

        // Position at offset and output sixel graphics (convert 0-based to 1-based)
        // Only output if graphics changed to reduce text overlay flicker
        if output_sixel {
            let row = self.sixel_row.saturating_add(1);
            let col = self.sixel_col.saturating_add(1);
            term.goto(row, col)?;
            term.write_raw(sixel_data)?;
        }

        // Text overlay - draw text on top of sixel
        // IMPORTANT: Don't output spaces over the sixel! Only output actual text characters.
        // Spaces would overwrite the sixel graphics with blank cells.
        let mut last_fg = Color::Black;
        let mut last_bg = Color::Black;

        for r in 1..=self.height {
            for c in 1..=self.width {
                if let Some(idx) = self.index(r, c) {
                    let front = self.front[idx];
                    let back = self.back[idx];

                    let back_is_text = back.ch != ' ' && back.ch != '\0';
                    let front_is_text = front.ch != ' ' && front.ch != '\0';

                    // Only output if:
                    // - There's actual text to draw (not spaces)
                    // - OR we need to clear old text that was there (front had text, back doesn't)
                    let should_output = back_is_text || (front_is_text && !back_is_text);

                    if should_output {
                        term.goto(r, c)?;
                        if back.fg != last_fg || back.bg != last_bg {
                            term.set_colors(back.fg, back.bg)?;
                            last_fg = back.fg;
                            last_bg = back.bg;
                        }
                        term.write_char(back.ch)?;
                        self.front[idx] = back;
                    }
                }
            }
        }

        // Handle cursor visibility
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

    /// Flush differential sixel updates
    /// Each update is positioned at its pixel coordinates
    pub fn flush_sixel_updates(&mut self, term: &mut Terminal, updates: &[SixelUpdate]) -> io::Result<()> {
        // Handle screen clearing if needed
        if self.sixel_needs_clear {
            term.set_colors(Color::LightGray, Color::Black)?;
            term.clear()?;
            self.sixel_needs_clear = false;
        }

        // Output each sixel update at its position
        for update in updates {
            // Convert pixel position to character cell (1-based)
            // Note: sixel renders at cursor position in pixel space, so we position
            // the cursor at the character cell that contains the top-left pixel
            let row = (update.y / self.char_height) as u16 + 1;
            let col = (update.x / self.char_width) as u16 + 1;

            term.goto(row, col)?;
            term.write_raw(&update.data)?;
        }

        // Text overlay - draw text on top of sixel
        let mut last_fg = Color::Black;
        let mut last_bg = Color::Black;

        for r in 1..=self.height {
            for c in 1..=self.width {
                if let Some(idx) = self.index(r, c) {
                    let front = self.front[idx];
                    let back = self.back[idx];

                    let back_is_text = back.ch != ' ' && back.ch != '\0';
                    let front_is_text = front.ch != ' ' && front.ch != '\0';

                    let should_output = back_is_text || (front_is_text && !back_is_text);

                    if should_output {
                        term.goto(r, c)?;
                        if back.fg != last_fg || back.bg != last_bg {
                            term.set_colors(back.fg, back.bg)?;
                            last_fg = back.fg;
                            last_bg = back.bg;
                        }
                        term.write_char(back.ch)?;
                        self.front[idx] = back;
                    }
                }
            }
        }

        // Handle cursor visibility
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

    /// Flush only text overlay changes without touching sixel graphics
    /// Used when in sixel mode but no graphics updates needed
    fn flush_text_overlay_only(&mut self, term: &mut Terminal) -> io::Result<()> {
        let mut last_fg = Color::Black;
        let mut last_bg = Color::Black;

        for r in 1..=self.height {
            for c in 1..=self.width {
                if let Some(idx) = self.index(r, c) {
                    let front = self.front[idx];
                    let back = self.back[idx];

                    let back_is_text = back.ch != ' ' && back.ch != '\0';
                    let front_is_text = front.ch != ' ' && front.ch != '\0';

                    // Only output if there's actual text to draw or text to clear
                    let should_output = back_is_text || (front_is_text && !back_is_text);

                    if should_output {
                        term.goto(r, c)?;
                        if back.fg != last_fg || back.bg != last_bg {
                            term.set_colors(back.fg, back.bg)?;
                            last_fg = back.fg;
                            last_bg = back.bg;
                        }
                        term.write_char(back.ch)?;
                        self.front[idx] = back;
                    }
                }
            }
        }

        // Handle cursor visibility
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
