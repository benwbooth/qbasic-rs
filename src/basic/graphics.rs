//! Graphics mode simulation using Unicode block characters
//!
//! Simulates QBasic graphics modes by using Unicode half-block characters:
//! - ▀ (upper half block) U+2580
//! - ▄ (lower half block) U+2584
//! - █ (full block) U+2588
//! - ░ ▒ ▓ for patterns
//!
//! Each terminal cell represents 2 vertical pixels


/// A cell in the text screen buffer
#[derive(Clone, Copy)]
pub struct TextCell {
    pub char: char,
    pub fg: u8,
    pub bg: u8,
}

impl Default for TextCell {
    fn default() -> Self {
        Self {
            char: ' ',
            fg: 7,  // Light gray
            bg: 0,  // Black
        }
    }
}

/// Graphics pixel buffer
pub struct GraphicsMode {
    /// Current screen mode (0=text, 1/2/7/9/12/13=graphics)
    pub mode: u8,

    /// Pixel buffer width
    pub width: u32,

    /// Pixel buffer height
    pub height: u32,

    /// Pixel data (color indices 0-15)
    pixels: Vec<u8>,

    /// Current foreground color
    pub foreground: u8,

    /// Current background color
    pub background: u8,

    /// Cursor row (1-based)
    pub cursor_row: u16,

    /// Cursor column (1-based)
    pub cursor_col: u16,

    /// Text screen dimensions
    pub text_cols: u16,
    pub text_rows: u16,

    /// Text screen buffer for text-mode output
    pub text_screen: Vec<TextCell>,
}

impl GraphicsMode {
    /// Create new graphics buffer
    pub fn new(cols: u32, rows: u32) -> Self {
        // Double vertical resolution using half-blocks
        let pixel_height = rows * 2;
        let pixel_width = cols;

        let text_cols = cols as u16;
        let text_rows = rows as u16;

        Self {
            mode: 0,
            width: pixel_width,
            height: pixel_height,
            pixels: vec![0; (pixel_width * pixel_height) as usize],
            foreground: 7,
            background: 0,  // Black background
            cursor_row: 1,
            cursor_col: 1,
            text_cols,
            text_rows,
            text_screen: vec![TextCell { char: ' ', fg: 7, bg: 0 }; (text_cols * text_rows) as usize],
        }
    }

    /// Set graphics mode
    pub fn set_mode(&mut self, mode: u8) {
        self.mode = mode;

        // Set resolution based on mode
        let (w, h) = match mode {
            0 => (80, 25),     // Text mode
            1 => (320, 200),   // CGA 4-color
            2 => (640, 200),   // CGA 2-color
            7 => (320, 200),   // EGA 16-color
            9 => (640, 350),   // EGA 16-color
            12 => (640, 480),  // VGA 16-color
            13 => (320, 200),  // VGA 256-color (we simulate with 16)
            _ => (320, 200),
        };

        self.width = w;
        self.height = h;
        self.pixels = vec![0; (w * h) as usize];
        self.foreground = 15;
        self.background = 0;
    }

    /// Clear screen
    pub fn cls(&mut self) {
        self.pixels.fill(self.background);
        // Clear text screen with current background color
        for cell in &mut self.text_screen {
            cell.char = ' ';
            cell.fg = self.foreground;
            cell.bg = self.background;
        }
        self.cursor_row = 1;
        self.cursor_col = 1;
    }

    /// Set colors
    pub fn set_color(&mut self, fg: u8, bg: u8) {
        self.foreground = fg & 0x0F;
        self.background = bg & 0x0F;
    }

    /// Set cursor position
    pub fn locate(&mut self, row: u16, col: u16) {
        self.cursor_row = row;
        self.cursor_col = col;
    }

    /// Resize the text screen buffer to match terminal size
    pub fn resize(&mut self, cols: u16, rows: u16) {
        if cols == self.text_cols && rows == self.text_rows {
            return;
        }

        let mut new_screen = vec![TextCell { char: ' ', fg: self.foreground, bg: self.background }; (cols * rows) as usize];

        // Copy existing content
        for row in 0..self.text_rows.min(rows) {
            for col in 0..self.text_cols.min(cols) {
                let old_idx = (row as usize) * (self.text_cols as usize) + (col as usize);
                let new_idx = (row as usize) * (cols as usize) + (col as usize);
                if old_idx < self.text_screen.len() && new_idx < new_screen.len() {
                    new_screen[new_idx] = self.text_screen[old_idx];
                }
            }
        }

        self.text_cols = cols;
        self.text_rows = rows;
        self.text_screen = new_screen;

        // Clamp cursor to new bounds
        if self.cursor_col > cols {
            self.cursor_col = cols;
        }
        if self.cursor_row > rows {
            self.cursor_row = rows;
        }
    }

    /// Print text at current cursor position
    pub fn print_text(&mut self, text: &str, advance_cursor: bool) {
        for ch in text.chars() {
            if ch == '\n' {
                self.cursor_row += 1;
                self.cursor_col = 1;
            } else {
                self.put_char(self.cursor_row, self.cursor_col, ch);
                self.cursor_col += 1;
                if self.cursor_col > self.text_cols {
                    self.cursor_col = 1;
                    self.cursor_row += 1;
                }
            }
        }
        if advance_cursor {
            self.cursor_row += 1;
            self.cursor_col = 1;
        }
        // Scroll if needed
        if self.cursor_row > self.text_rows {
            self.scroll_up();
            self.cursor_row = self.text_rows;
        }
    }

    /// Put a character at a specific position
    pub fn put_char(&mut self, row: u16, col: u16, ch: char) {
        if row >= 1 && row <= self.text_rows && col >= 1 && col <= self.text_cols {
            let idx = ((row - 1) as usize) * (self.text_cols as usize) + ((col - 1) as usize);
            if idx < self.text_screen.len() {
                self.text_screen[idx] = TextCell {
                    char: ch,
                    fg: self.foreground,
                    bg: self.background,
                };
            }
        }
    }

    /// Scroll the text screen up by one line
    fn scroll_up(&mut self) {
        let cols = self.text_cols as usize;
        // Move all rows up by one
        for row in 1..self.text_rows as usize {
            let src_start = row * cols;
            let dst_start = (row - 1) * cols;
            for c in 0..cols {
                self.text_screen[dst_start + c] = self.text_screen[src_start + c];
            }
        }
        // Clear the last row
        let last_row_start = ((self.text_rows - 1) as usize) * cols;
        for c in 0..cols {
            self.text_screen[last_row_start + c] = TextCell {
                char: ' ',
                fg: self.foreground,
                bg: self.background,
            };
        }
    }

    /// Get a character at a specific position
    pub fn get_char(&self, row: u16, col: u16) -> TextCell {
        if row >= 1 && row <= self.text_rows && col >= 1 && col <= self.text_cols {
            let idx = ((row - 1) as usize) * (self.text_cols as usize) + ((col - 1) as usize);
            if idx < self.text_screen.len() {
                return self.text_screen[idx];
            }
        }
        TextCell::default()
    }

    /// Set a pixel
    pub fn pset(&mut self, x: i32, y: i32, color: u8) {
        if x >= 0 && y >= 0 && (x as u32) < self.width && (y as u32) < self.height {
            let idx = (y as u32) * self.width + (x as u32);
            self.pixels[idx as usize] = color & 0x0F;
        }
    }

    /// Get pixel color
    pub fn point(&self, x: i32, y: i32) -> u8 {
        if x >= 0 && y >= 0 && (x as u32) < self.width && (y as u32) < self.height {
            let idx = (y as u32) * self.width + (x as u32);
            self.pixels[idx as usize]
        } else {
            0
        }
    }

    /// Draw a line using Bresenham's algorithm
    pub fn line(&mut self, x1: i32, y1: i32, x2: i32, y2: i32, color: u8) {
        let dx = (x2 - x1).abs();
        let dy = -(y2 - y1).abs();
        let sx = if x1 < x2 { 1 } else { -1 };
        let sy = if y1 < y2 { 1 } else { -1 };
        let mut err = dx + dy;
        let mut x = x1;
        let mut y = y1;

        loop {
            self.pset(x, y, color);

            if x == x2 && y == y2 {
                break;
            }

            let e2 = 2 * err;
            if e2 >= dy {
                if x == x2 {
                    break;
                }
                err += dy;
                x += sx;
            }
            if e2 <= dx {
                if y == y2 {
                    break;
                }
                err += dx;
                y += sy;
            }
        }
    }

    /// Draw a box (unfilled rectangle)
    pub fn draw_box(&mut self, x1: i32, y1: i32, x2: i32, y2: i32, color: u8) {
        self.line(x1, y1, x2, y1, color); // Top
        self.line(x1, y2, x2, y2, color); // Bottom
        self.line(x1, y1, x1, y2, color); // Left
        self.line(x2, y1, x2, y2, color); // Right
    }

    /// Draw a filled box
    pub fn fill_box(&mut self, x1: i32, y1: i32, x2: i32, y2: i32, color: u8) {
        let (x_min, x_max) = if x1 <= x2 { (x1, x2) } else { (x2, x1) };
        let (y_min, y_max) = if y1 <= y2 { (y1, y2) } else { (y2, y1) };

        for y in y_min..=y_max {
            for x in x_min..=x_max {
                self.pset(x, y, color);
            }
        }
    }

    /// Draw a circle using midpoint algorithm
    pub fn circle(&mut self, cx: i32, cy: i32, r: i32, color: u8) {
        let mut x = r;
        let mut y = 0;
        let mut p = 1 - r;

        // Plot initial points
        self.circle_points(cx, cy, x, y, color);

        while x > y {
            y += 1;
            if p <= 0 {
                p += 2 * y + 1;
            } else {
                x -= 1;
                p += 2 * y - 2 * x + 1;
            }
            self.circle_points(cx, cy, x, y, color);
        }
    }

    fn circle_points(&mut self, cx: i32, cy: i32, x: i32, y: i32, color: u8) {
        self.pset(cx + x, cy + y, color);
        self.pset(cx - x, cy + y, color);
        self.pset(cx + x, cy - y, color);
        self.pset(cx - x, cy - y, color);
        self.pset(cx + y, cy + x, color);
        self.pset(cx - y, cy + x, color);
        self.pset(cx + y, cy - x, color);
        self.pset(cx - y, cy - x, color);
    }

    /// Flood fill (simple recursive - limited for large areas)
    pub fn paint(&mut self, x: i32, y: i32, fill_color: u8) {
        if x < 0 || y < 0 || (x as u32) >= self.width || (y as u32) >= self.height {
            return;
        }

        let target_color = self.point(x, y);
        if target_color == fill_color {
            return;
        }

        // Use a simple scanline fill instead of recursive
        self.scanline_fill(x, y, target_color, fill_color);
    }

    fn scanline_fill(&mut self, start_x: i32, start_y: i32, target: u8, fill: u8) {
        let mut stack = vec![(start_x, start_y)];

        while let Some((x, y)) = stack.pop() {
            if x < 0 || y < 0 || (x as u32) >= self.width || (y as u32) >= self.height {
                continue;
            }

            if self.point(x, y) != target {
                continue;
            }

            // Find left edge
            let mut left = x;
            while left > 0 && self.point(left - 1, y) == target {
                left -= 1;
            }

            // Find right edge and fill
            let mut right = x;
            while (right as u32) < self.width && self.point(right, y) == target {
                self.pset(right, y, fill);
                right += 1;
            }

            // Add spans above and below
            for scan_x in left..right {
                if y > 0 && self.point(scan_x, y - 1) == target {
                    stack.push((scan_x, y - 1));
                }
                if (y as u32) < self.height - 1 && self.point(scan_x, y + 1) == target {
                    stack.push((scan_x, y + 1));
                }
            }
        }
    }

}

impl Default for GraphicsMode {
    fn default() -> Self {
        Self::new(80, 25)
    }
}
