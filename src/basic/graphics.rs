//! Graphics mode simulation
#![allow(dead_code)]
//!
//! Supports two rendering backends:
//! - Sixel graphics for terminals that support it (true pixel graphics)
//! - Unicode half-block characters as fallback:
//!   - ▀ (upper half block) U+2580
//!   - ▄ (lower half block) U+2584
//!   - █ (full block) U+2588
//!
//! Each terminal cell represents 2 vertical pixels in block mode

use super::sixel::SixelEncoder;


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

    /// Previous pixel data for change detection
    prev_pixels: Vec<u8>,

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

    /// Graphics dirty flag - set when pixels change
    dirty: bool,

    /// Dirty region bounds (pixel coordinates)
    dirty_x_min: u32,
    dirty_y_min: u32,
    dirty_x_max: u32,
    dirty_y_max: u32,

    /// Cached sixel output
    sixel_cache: String,

    /// Cached terminal size for sixel
    cached_term_size: (u16, u16),

    /// Generation counter - increments on every change
    generation: u64,

    /// Whether screen needs clearing (mode changed, etc.)
    needs_clear: bool,
}

impl GraphicsMode {
    // Typical character cell size in pixels
    const CHAR_WIDTH: u32 = 8;
    const CHAR_HEIGHT: u32 = 16;

    /// Create new graphics buffer sized to terminal dimensions
    pub fn new(cols: u32, rows: u32) -> Self {
        // Size pixel buffer to match terminal (cols * char_width, rows * char_height)
        let pixel_width = cols * Self::CHAR_WIDTH;
        let pixel_height = rows * Self::CHAR_HEIGHT;

        let text_cols = cols as u16;
        let text_rows = rows as u16;

        let pixel_count = (pixel_width * pixel_height) as usize;
        Self {
            mode: 12,
            width: pixel_width,
            height: pixel_height,
            pixels: vec![0; pixel_count],
            prev_pixels: vec![0; pixel_count],
            foreground: 15,
            background: 0,
            cursor_row: 1,
            cursor_col: 1,
            text_cols,
            text_rows,
            text_screen: vec![TextCell { char: ' ', fg: 15, bg: 0 }; (text_cols * text_rows) as usize],
            dirty: true,
            dirty_x_min: 0,
            dirty_y_min: 0,
            dirty_x_max: pixel_width.saturating_sub(1),
            dirty_y_max: pixel_height.saturating_sub(1),
            sixel_cache: String::new(),
            cached_term_size: (0, 0),
            generation: 0,
            needs_clear: true,
        }
    }

    /// Resize graphics buffer when terminal size changes
    /// cols/rows are character cell dimensions, pixel_width/pixel_height are actual pixel dimensions
    /// If pixel dimensions are 0, falls back to cols*8 / rows*16
    pub fn resize_pixels(&mut self, cols: u32, rows: u32, pixel_width: u32, pixel_height: u32) {
        // Use provided pixel dimensions or fall back to assumed cell size
        let pixel_width = if pixel_width > 0 { pixel_width } else { cols * Self::CHAR_WIDTH };
        let pixel_height = if pixel_height > 0 { pixel_height } else { rows * Self::CHAR_HEIGHT };

        // Only resize if dimensions actually changed
        if pixel_width == self.width && pixel_height == self.height {
            return;
        }

        let pixel_count = (pixel_width * pixel_height) as usize;
        self.width = pixel_width;
        self.height = pixel_height;
        self.pixels = vec![self.background; pixel_count];
        self.prev_pixels = vec![self.background; pixel_count];

        self.text_cols = cols as u16;
        self.text_rows = rows as u16;
        self.text_screen = vec![TextCell { char: ' ', fg: self.foreground, bg: self.background }; (cols * rows) as usize];

        self.dirty = true;
        self.dirty_x_min = 0;
        self.dirty_y_min = 0;
        self.dirty_x_max = pixel_width.saturating_sub(1);
        self.dirty_y_max = pixel_height.saturating_sub(1);
        self.sixel_cache.clear();
        self.needs_clear = true;
    }

    /// Resize graphics buffer when terminal size changes (legacy, assumes 8x16 cells)
    pub fn resize(&mut self, cols: u32, rows: u32) {
        self.resize_pixels(cols, rows, 0, 0);
    }

    /// Set graphics mode
    pub fn set_mode(&mut self, mode: u8) {
        self.mode = mode;
        self.cls();
        self.needs_clear = true;
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
        self.dirty = true;
        // Mark entire screen as dirty after clear
        self.dirty_x_min = 0;
        self.dirty_y_min = 0;
        self.dirty_x_max = self.width.saturating_sub(1);
        self.dirty_y_max = self.height.saturating_sub(1);
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
            let xu = x as u32;
            let yu = y as u32;
            let idx = (yu * self.width + xu) as usize;
            let new_color = color & 0x0F;

            // Only mark dirty if pixel actually changed
            if self.pixels[idx] != new_color {
                self.pixels[idx] = new_color;
                self.dirty = true;

                // Update dirty region bounds
                if self.dirty_x_min > self.dirty_x_max {
                    // No dirty region yet, start new one
                    self.dirty_x_min = xu;
                    self.dirty_x_max = xu;
                    self.dirty_y_min = yu;
                    self.dirty_y_max = yu;
                } else {
                    // Expand existing dirty region
                    self.dirty_x_min = self.dirty_x_min.min(xu);
                    self.dirty_x_max = self.dirty_x_max.max(xu);
                    self.dirty_y_min = self.dirty_y_min.min(yu);
                    self.dirty_y_max = self.dirty_y_max.max(yu);
                }
            }
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
        // Mark the full bounding box as dirty for proper partial updates
        // This ensures that when erasing a circle, the entire area is redrawn
        self.mark_bounding_box_dirty(cx - r, cy - r, cx + r, cy + r);

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

    /// Draw a circle arc (or full circle if no angles specified)
    /// In QBasic, angles are in radians: 0 is at 3 o'clock, going counter-clockwise
    pub fn circle_arc(&mut self, cx: i32, cy: i32, r: i32, color: u8, start: Option<f64>, end: Option<f64>) {
        match (start, end) {
            (Some(start_angle), Some(end_angle)) => {
                // Mark the full bounding box as dirty
                self.mark_bounding_box_dirty(cx - r, cy - r, cx + r, cy + r);

                // Draw arc using parametric approach
                // Step around the arc and draw each point
                let step = 1.0 / (r as f64).max(1.0);
                let mut angle = start_angle;

                // Handle wrap-around (e.g., start=7pi/4, end=pi/4)
                let end_adjusted = if end_angle < start_angle {
                    end_angle + std::f64::consts::PI * 2.0
                } else {
                    end_angle
                };

                while angle <= end_adjusted {
                    let px = cx + (r as f64 * angle.cos()).round() as i32;
                    let py = cy - (r as f64 * angle.sin()).round() as i32; // Y is inverted in screen coords
                    self.pset(px, py, color);
                    angle += step;
                }

                // Make sure we hit the end point
                let px = cx + (r as f64 * end_angle.cos()).round() as i32;
                let py = cy - (r as f64 * end_angle.sin()).round() as i32;
                self.pset(px, py, color);
            }
            _ => {
                // No arc angles, draw full circle
                self.circle(cx, cy, r, color);
            }
        }
    }

    /// Mark a bounding box as dirty (for shapes that affect more than just drawn pixels)
    fn mark_bounding_box_dirty(&mut self, x1: i32, y1: i32, x2: i32, y2: i32) {
        // Clamp to screen bounds
        let x_min = x1.max(0) as u32;
        let y_min = y1.max(0) as u32;
        let x_max = (x2.max(0) as u32).min(self.width.saturating_sub(1));
        let y_max = (y2.max(0) as u32).min(self.height.saturating_sub(1));

        if x_min <= x_max && y_min <= y_max {
            self.dirty = true;
            if self.dirty_x_min > self.dirty_x_max {
                // No dirty region yet
                self.dirty_x_min = x_min;
                self.dirty_x_max = x_max;
                self.dirty_y_min = y_min;
                self.dirty_y_max = y_max;
            } else {
                // Expand existing dirty region
                self.dirty_x_min = self.dirty_x_min.min(x_min);
                self.dirty_x_max = self.dirty_x_max.max(x_max);
                self.dirty_y_min = self.dirty_y_min.min(y_min);
                self.dirty_y_max = self.dirty_y_max.max(y_max);
            }
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

impl GraphicsMode {
    /// Render the pixel buffer as sixel graphics
    ///
    /// Returns the sixel escape sequence string that can be written to terminal.
    /// The `scale` parameter controls the pixel size (1 = native, 2 = 2x, etc.)
    pub fn render_sixel(&self, scale: u32) -> String {
        let mut encoder = SixelEncoder::new();
        encoder.encode(&self.pixels, self.width, self.height, scale).to_string()
    }

    /// Render the pixel buffer as sixel graphics
    /// Buffer is already sized to terminal, so render at 1:1
    pub fn render_sixel_fit(&mut self, term_cols: u16, term_rows: u16, _char_width: u32, _char_height: u32) -> &str {
        // Check if we can use cached sixel
        let term_size = (term_cols, term_rows);
        if !self.dirty && self.cached_term_size == term_size && !self.sixel_cache.is_empty() {
            return &self.sixel_cache;
        }

        // Render at native size (1:1) since buffer is already sized to terminal
        let mut encoder = SixelEncoder::new();
        self.sixel_cache = encoder.encode(&self.pixels, self.width, self.height, 1).to_string();
        self.cached_term_size = term_size;
        self.dirty = false;
        self.generation = self.generation.wrapping_add(1);

        &self.sixel_cache
    }

    /// Get reference to the pixel buffer
    pub fn pixels(&self) -> &[u8] {
        &self.pixels
    }

    /// Check if in graphics mode - always true since we always support graphics
    pub fn is_graphics_mode(&self) -> bool {
        true
    }

    /// Check if graphics have changed since last render
    pub fn is_dirty(&self) -> bool {
        self.dirty
    }

    /// Get current generation (changes on every pixel modification)
    pub fn generation(&self) -> u64 {
        self.generation
    }

    /// Get cached sixel if available and not dirty
    pub fn get_cached_sixel(&self) -> Option<&str> {
        if !self.dirty && !self.sixel_cache.is_empty() {
            Some(&self.sixel_cache)
        } else {
            None
        }
    }

    /// Check if screen needs clearing (mode changed, first render, etc.)
    /// This consumes the flag - subsequent calls return false until set again.
    pub fn take_needs_clear(&mut self) -> bool {
        let result = self.needs_clear;
        self.needs_clear = false;
        result
    }

    /// Reset dirty region to empty (no dirty pixels)
    fn reset_dirty_region(&mut self) {
        self.dirty_x_min = u32::MAX;
        self.dirty_y_min = u32::MAX;
        self.dirty_x_max = 0;
        self.dirty_y_max = 0;
        self.dirty = false;
    }

    /// Get the dirty region aligned to sixel boundaries (6-pixel rows)
    /// Returns (x, y, width, height) or None if no dirty region
    pub fn get_dirty_region(&self) -> Option<(u32, u32, u32, u32)> {
        if !self.dirty || self.dirty_x_min > self.dirty_x_max || self.dirty_y_min > self.dirty_y_max {
            return None;
        }

        // Align y to 6-pixel boundaries for sixel
        let y_aligned = (self.dirty_y_min / 6) * 6;
        let y_end = ((self.dirty_y_max + 6) / 6) * 6;
        let height = (y_end - y_aligned).min(self.height - y_aligned);

        Some((
            self.dirty_x_min,
            y_aligned,
            self.dirty_x_max - self.dirty_x_min + 1,
            height,
        ))
    }

    /// Render just the dirty region as sixel graphics
    /// Returns (sixel_data, x_pixel, y_pixel, width, height) or None if nothing dirty
    pub fn render_dirty_region(&mut self) -> Option<(String, u32, u32, u32, u32)> {
        let (x, y, w, h) = self.get_dirty_region()?;

        let mut encoder = SixelEncoder::new();
        let sixel = encoder.encode_region(&self.pixels, self.width, self.height, x, y, w, h).to_string();

        // Reset dirty region
        self.reset_dirty_region();
        self.generation = self.generation.wrapping_add(1);

        Some((sixel, x, y, w, h))
    }

    /// Check if the dirty region is the full screen (meaning we should do a full redraw)
    pub fn is_full_redraw_needed(&self) -> bool {
        if !self.dirty {
            return false;
        }
        // If dirty region is more than 25% of the screen, do full redraw
        let dirty_area = (self.dirty_x_max.saturating_sub(self.dirty_x_min) + 1)
            * (self.dirty_y_max.saturating_sub(self.dirty_y_min) + 1);
        let full_area = self.width * self.height;
        dirty_area > full_area / 4
    }

    /// Render the pixel buffer as sixel graphics with differential updates
    /// Returns a list of updates: (sixel_data, x_pixel, y_pixel, width, height)
    /// On first call or full redraw, returns single full-screen update
    ///
    /// char_width and char_height are used to align partial updates to cell boundaries
    pub fn render_sixel_differential_aligned(&mut self, term_cols: u16, term_rows: u16, char_width: u32, char_height: u32) -> Vec<(String, u32, u32, u32, u32)> {
        let term_size = (term_cols, term_rows);
        let mut updates = Vec::new();

        // If not dirty, return empty
        if !self.dirty && self.cached_term_size == term_size {
            return updates;
        }

        // Check if we need full redraw (first render, resize, or large dirty area)
        let full_redraw = self.cached_term_size != term_size || self.is_full_redraw_needed() || self.sixel_cache.is_empty();

        if full_redraw {
            // Full screen render
            let mut encoder = SixelEncoder::new();
            self.sixel_cache = encoder.encode(&self.pixels, self.width, self.height, 1).to_string();
            updates.push((self.sixel_cache.clone(), 0, 0, self.width, self.height));
        } else if let Some((x, y, w, h)) = self.get_dirty_region() {
            // Align dirty region to character cell boundaries for correct sixel positioning
            // Calculate which character cells are affected
            let cell_x = x / char_width;
            let cell_y = y / char_height;
            let cell_x_end = (x + w + char_width - 1) / char_width;
            let cell_y_end = (y + h + char_height - 1) / char_height;

            // Add top margin to handle terminal-specific sixel positioning quirks.
            // Some terminals render sixels lower than the cursor position (e.g., aligned
            // to text baseline rather than cell top). Adding margin at the top ensures
            // the intended content is visible. We don't add bottom margin to avoid
            // extending past screen bounds and causing scrolling.
            const TOP_MARGIN_ROWS: u32 = 2;
            let cell_y_start = cell_y.saturating_sub(TOP_MARGIN_ROWS);

            // Convert back to pixel coordinates (now cell-aligned)
            let aligned_x = cell_x * char_width;
            let aligned_y = cell_y_start * char_height;

            // Calculate width and height to cover the full cell range, rounded up to 6 pixels
            let aligned_w = (cell_x_end * char_width).saturating_sub(aligned_x);
            let pixel_end_y = cell_y_end * char_height;
            let aligned_h = ((pixel_end_y.saturating_sub(aligned_y) + 5) / 6) * 6;

            // Clamp to screen bounds to avoid scrolling
            let aligned_w = aligned_w.min(self.width.saturating_sub(aligned_x));
            let aligned_h = aligned_h.min(self.height.saturating_sub(aligned_y));

            if aligned_w > 0 && aligned_h > 0 {
                // Encode the aligned region
                let mut encoder = SixelEncoder::new();
                let sixel = encoder.encode_region(&self.pixels, self.width, self.height,
                                                  aligned_x, aligned_y, aligned_w, aligned_h).to_string();
                updates.push((sixel, aligned_x, aligned_y, aligned_w, aligned_h));
            }

            self.reset_dirty_region();
            self.generation = self.generation.wrapping_add(1);
            return updates;
        }

        self.cached_term_size = term_size;
        self.reset_dirty_region();
        self.generation = self.generation.wrapping_add(1);

        updates
    }

    /// Render the pixel buffer as sixel graphics with differential updates (legacy, no alignment)
    pub fn render_sixel_differential(&mut self, term_cols: u16, term_rows: u16) -> Vec<(String, u32, u32, u32, u32)> {
        // Default to 8x16 char cells if not specified
        self.render_sixel_differential_aligned(term_cols, term_rows, 8, 16)
    }
}
