//! Graphics mode simulation using Unicode block characters
//!
//! Simulates QBasic graphics modes by using Unicode half-block characters:
//! - ▀ (upper half block) U+2580
//! - ▄ (lower half block) U+2584
//! - █ (full block) U+2588
//! - ░ ▒ ▓ for patterns
//!
//! Each terminal cell represents 2 vertical pixels

use crate::screen::Screen;
use crate::terminal::Color;

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
}

impl GraphicsMode {
    /// Create new graphics buffer
    pub fn new(cols: u32, rows: u32) -> Self {
        // Double vertical resolution using half-blocks
        let pixel_height = rows * 2;
        let pixel_width = cols;

        Self {
            mode: 0,
            width: pixel_width,
            height: pixel_height,
            pixels: vec![0; (pixel_width * pixel_height) as usize],
            foreground: 15,
            background: 0,
            cursor_row: 1,
            cursor_col: 1,
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

    /// Render graphics to screen buffer using Unicode half-blocks
    pub fn render_to_screen(&self, screen: &mut Screen, row: u16, col: u16, term_width: u16, term_height: u16) {
        if self.mode == 0 {
            // Text mode - nothing to render
            return;
        }

        // Calculate scaling
        let scale_x = self.width as f32 / term_width as f32;
        let scale_y = self.height as f32 / (term_height as f32 * 2.0); // *2 for half-blocks

        for term_row in 0..term_height {
            for term_col in 0..term_width {
                // Get pixel coordinates for top and bottom half of this cell
                let px = (term_col as f32 * scale_x) as i32;
                let py_top = (term_row as f32 * 2.0 * scale_y) as i32;
                let py_bot = ((term_row as f32 * 2.0 + 1.0) * scale_y) as i32;

                let color_top = self.point(px, py_top);
                let color_bot = self.point(px, py_bot);

                // Determine character and colors
                let (ch, fg, bg) = if color_top == color_bot {
                    // Same color - use full block or space
                    if color_top == 0 {
                        (' ', Color::White, Color::Black)
                    } else {
                        ('█', dos_to_ansi(color_top), Color::Black)
                    }
                } else if color_top == 0 {
                    // Only bottom half
                    ('▄', dos_to_ansi(color_bot), Color::Black)
                } else if color_bot == 0 {
                    // Only top half
                    ('▀', dos_to_ansi(color_top), Color::Black)
                } else {
                    // Both different colors - use upper half block with bg
                    ('▀', dos_to_ansi(color_top), dos_to_ansi(color_bot))
                };

                screen.set(row + term_row, col + term_col, ch, fg, bg);
            }
        }
    }
}

/// Convert DOS color index to ANSI color
fn dos_to_ansi(color: u8) -> Color {
    match color & 0x0F {
        0 => Color::Black,
        1 => Color::Blue,
        2 => Color::Green,
        3 => Color::Cyan,
        4 => Color::Red,
        5 => Color::Magenta,
        6 => Color::Brown,
        7 => Color::LightGray,
        8 => Color::DarkGray,
        9 => Color::LightBlue,
        10 => Color::LightGreen,
        11 => Color::LightCyan,
        12 => Color::LightRed,
        13 => Color::LightMagenta,
        14 => Color::Yellow,
        15 => Color::White,
        _ => Color::White,
    }
}

impl Default for GraphicsMode {
    fn default() -> Self {
        Self::new(80, 25)
    }
}
