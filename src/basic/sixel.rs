//! Sixel graphics encoder for terminal output
#![allow(dead_code)]
//!
//! Converts a pixel buffer with 16-color palette to sixel format
//! for display in terminals that support sixel graphics.

/// Standard 16-color CGA/EGA palette (RGB values 0-255)
pub const PALETTE_16: [(u8, u8, u8); 16] = [
    (0, 0, 0),       // 0: Black
    (0, 0, 170),     // 1: Blue
    (0, 170, 0),     // 2: Green
    (0, 170, 170),   // 3: Cyan
    (170, 0, 0),     // 4: Red
    (170, 0, 170),   // 5: Magenta
    (170, 85, 0),    // 6: Brown
    (170, 170, 170), // 7: Light Gray
    (85, 85, 85),    // 8: Dark Gray
    (85, 85, 255),   // 9: Light Blue
    (85, 255, 85),   // 10: Light Green
    (85, 255, 255),  // 11: Light Cyan
    (255, 85, 85),   // 12: Light Red
    (255, 85, 255),  // 13: Light Magenta
    (255, 255, 85),  // 14: Yellow
    (255, 255, 255), // 15: White
];

/// Sixel encoder
pub struct SixelEncoder {
    /// Output buffer
    output: String,
}

impl SixelEncoder {
    pub fn new() -> Self {
        Self {
            output: String::new(),
        }
    }

    /// Encode a pixel buffer to sixel format
    ///
    /// * `pixels` - Color indices (0-15) for each pixel, row-major order
    /// * `width` - Image width in pixels
    /// * `height` - Image height in pixels
    /// * `scale` - Scale factor (1 = 1:1, 2 = 2x size, etc.)
    pub fn encode(&mut self, pixels: &[u8], width: u32, height: u32, scale: u32) -> &str {
        self.output.clear();

        let scale = scale.max(1);
        let scaled_width = width * scale;
        let scaled_height = height * scale;

        // Sixel introducer: DCS q
        // P1=0 (pixel aspect ratio 2:1), P2=1 (no background), P3=0 (horizontal grid size)
        self.output.push_str("\x1bP0;1;q");

        // Define color palette (convert 0-255 to 0-100 for sixel)
        for (i, &(r, g, b)) in PALETTE_16.iter().enumerate() {
            let r100 = (r as u32 * 100 / 255) as u8;
            let g100 = (g as u32 * 100 / 255) as u8;
            let b100 = (b as u32 * 100 / 255) as u8;
            self.output.push_str(&format!("#{};2;{};{};{}", i, r100, g100, b100));
        }

        // Encode sixel data
        // Sixel encodes 6 vertical pixels per character
        // Character value = sum of (bit << position) + 63, where bit is 0 or 1
        // Positions: bit 0 = top pixel, bit 5 = bottom pixel

        let mut y = 0u32;
        while y < scaled_height {
            // For each row of 6 pixels (a "sixel row")
            let sixel_row_height = 6.min(scaled_height - y);

            // Process each color separately (sixel draws one color at a time)
            for color in 0..16u8 {
                // Select this color
                self.output.push_str(&format!("#{}", color));

                let mut x = 0u32;
                let mut run_char: Option<char> = None;
                let mut run_length = 0u32;

                while x < scaled_width {
                    // Build the sixel character for this column
                    let mut sixel_bits = 0u8;

                    for bit in 0..sixel_row_height {
                        let py = y + bit;
                        let src_y = py / scale;
                        let src_x = x / scale;

                        if src_y < height && src_x < width {
                            let idx = (src_y * width + src_x) as usize;
                            if idx < pixels.len() && pixels[idx] == color {
                                sixel_bits |= 1 << bit;
                            }
                        }
                    }

                    let sixel_char = (sixel_bits + 63) as char;

                    // Run-length encoding
                    if Some(sixel_char) == run_char {
                        run_length += 1;
                    } else {
                        // Flush previous run
                        self.flush_run(run_char, run_length);
                        run_char = Some(sixel_char);
                        run_length = 1;
                    }

                    x += 1;
                }

                // Flush final run
                self.flush_run(run_char, run_length);

                // Carriage return (go back to start of this sixel row for next color)
                self.output.push('$');
            }

            // Move to next sixel row (down 6 pixels)
            self.output.push('-');
            y += 6;
        }

        // String terminator
        self.output.push_str("\x1b\\");

        &self.output
    }

    /// Flush a run of characters with optional RLE
    fn flush_run(&mut self, ch: Option<char>, count: u32) {
        if let Some(c) = ch {
            if count == 0 {
                return;
            }
            // Don't RLE for '?' (0x3F = all zeros) as it's often just empty space
            // For other characters or long runs, use RLE
            if count > 3 {
                self.output.push_str(&format!("!{}{}", count, c));
            } else {
                for _ in 0..count {
                    self.output.push(c);
                }
            }
        }
    }

    /// Check if terminal likely supports sixel
    /// This checks the TERM environment variable for known sixel-capable terminals
    pub fn terminal_supports_sixel() -> bool {
        if let Ok(term) = std::env::var("TERM") {
            let term_lower = term.to_lowercase();
            // Known sixel-supporting terminals
            term_lower.contains("xterm")
                || term_lower.contains("mlterm")
                || term_lower.contains("yaft")
                || term_lower.contains("foot")
                || term_lower.contains("contour")
                || term_lower.contains("wezterm")
                || term_lower.contains("kitty")  // kitty has its own protocol but some builds support sixel
        } else {
            false
        }
    }
}

impl Default for SixelEncoder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_encode() {
        let mut encoder = SixelEncoder::new();
        // 2x2 image: red, green / blue, yellow
        let pixels = vec![4, 2, 1, 14];
        let sixel = encoder.encode(&pixels, 2, 2, 1);

        // Should start with DCS and end with ST
        assert!(sixel.starts_with("\x1bP"));
        assert!(sixel.ends_with("\x1b\\"));
    }

    #[test]
    fn test_scaled_encode() {
        let mut encoder = SixelEncoder::new();
        // 1x1 white pixel, scaled 2x
        let pixels = vec![15];
        let sixel = encoder.encode(&pixels, 1, 1, 2);

        assert!(sixel.starts_with("\x1bP"));
        assert!(sixel.ends_with("\x1b\\"));
    }
}
