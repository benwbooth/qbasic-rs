//! Terminal handling with raw ANSI escape sequences
//! No external TUI libraries - just raw escape codes

use std::io::{self, Read, Write};
use std::os::unix::io::AsRawFd;

/// DOS color palette (16 colors)
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[repr(u8)]
pub enum Color {
    Black = 0,
    Blue = 1,
    Green = 2,
    Cyan = 3,
    Red = 4,
    Magenta = 5,
    Brown = 6,
    LightGray = 7,
    DarkGray = 8,
    LightBlue = 9,
    LightGreen = 10,
    LightCyan = 11,
    LightRed = 12,
    LightMagenta = 13,
    Yellow = 14,
    White = 15,
}

impl Color {
    /// Get RGB values for DOS CGA/EGA palette
    /// These are the exact colors used in DOS text mode
    fn to_rgb(self) -> (u8, u8, u8) {
        match self {
            Color::Black => (0x00, 0x00, 0x00),
            Color::Blue => (0x00, 0x00, 0xAA),
            Color::Green => (0x00, 0xAA, 0x00),
            Color::Cyan => (0x00, 0xAA, 0xAA),
            Color::Red => (0xAA, 0x00, 0x00),
            Color::Magenta => (0xAA, 0x00, 0xAA),
            Color::Brown => (0xAA, 0x55, 0x00),
            Color::LightGray => (0xAA, 0xAA, 0xAA),
            Color::DarkGray => (0x55, 0x55, 0x55),
            Color::LightBlue => (0x55, 0x55, 0xFF),
            Color::LightGreen => (0x55, 0xFF, 0x55),
            Color::LightCyan => (0x55, 0xFF, 0xFF),
            Color::LightRed => (0xFF, 0x55, 0x55),
            Color::LightMagenta => (0xFF, 0x55, 0xFF),
            Color::Yellow => (0xFF, 0xFF, 0x55),
            Color::White => (0xFF, 0xFF, 0xFF),
        }
    }

    /// Convert DOS color to ANSI SGR foreground code (using true color)
    pub fn to_fg_sgr(self) -> String {
        let (r, g, b) = self.to_rgb();
        format!("38;2;{};{};{}", r, g, b)
    }

    /// Convert DOS color to ANSI SGR background code (using true color)
    pub fn to_bg_sgr(self) -> String {
        let (r, g, b) = self.to_rgb();
        format!("48;2;{};{};{}", r, g, b)
    }

    /// Invert the RGB values and find the closest matching palette color
    pub fn invert(self) -> Color {
        let (r, g, b) = self.to_rgb();
        let inv_r = 255 - r;
        let inv_g = 255 - g;
        let inv_b = 255 - b;

        // Find closest color in palette
        let colors = [
            Color::Black, Color::Blue, Color::Green, Color::Cyan,
            Color::Red, Color::Magenta, Color::Brown, Color::LightGray,
            Color::DarkGray, Color::LightBlue, Color::LightGreen, Color::LightCyan,
            Color::LightRed, Color::LightMagenta, Color::Yellow, Color::White,
        ];

        let mut best = Color::Black;
        let mut best_dist = u32::MAX;

        for color in colors {
            let (cr, cg, cb) = color.to_rgb();
            let dr = (inv_r as i32 - cr as i32).abs() as u32;
            let dg = (inv_g as i32 - cg as i32).abs() as u32;
            let db = (inv_b as i32 - cb as i32).abs() as u32;
            let dist = dr * dr + dg * dg + db * db;
            if dist < best_dist {
                best_dist = dist;
                best = color;
            }
        }

        best
    }
}

/// Mouse button
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MouseButton {
    Left,
    Middle,
    Right,
    WheelUp,
    WheelDown,
    None, // No button pressed (for motion-only events)
}

/// Mouse event
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MouseEvent {
    pub button: MouseButton,
    pub row: u16,
    pub col: u16,
    pub pressed: bool, // true for press, false for release
    pub motion: bool,  // true if this is a motion event (drag)
}

/// Key events including special keys
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Key {
    Char(char),
    Enter,
    Escape,
    Backspace,
    Delete,
    Tab,
    ShiftTab,
    Up,
    Down,
    Left,
    Right,
    Home,
    End,
    PageUp,
    PageDown,
    Insert,
    // Shift+navigation keys
    ShiftUp,
    ShiftDown,
    ShiftLeft,
    ShiftRight,
    ShiftHome,
    ShiftEnd,
    ShiftSpace,
    CtrlSpace,
    // Ctrl+navigation keys
    CtrlUp,
    CtrlDown,
    CtrlLeft,
    CtrlRight,
    CtrlHome,
    CtrlEnd,
    CtrlPageUp,
    CtrlPageDown,
    CtrlBackspace,
    CtrlDelete,
    // Ctrl+Shift+navigation
    CtrlShiftLeft,
    CtrlShiftRight,
    CtrlShiftHome,
    CtrlShiftEnd,
    CtrlShiftK,
    // Alt+navigation
    AltUp,
    AltDown,
    F(u8), // F1-F12
    Alt(char),
    Ctrl(char),
    Mouse(MouseEvent),
    Unknown(Vec<u8>),
}

/// Original terminal settings for restoration
static mut ORIG_TERMIOS: Option<libc::termios> = None;

/// Terminal state manager
pub struct Terminal {
    stdout: io::Stdout,
    width: u16,
    height: u16,
}

impl Terminal {
    /// Initialize terminal in raw mode
    pub fn new() -> io::Result<Self> {
        let mut term = Self {
            stdout: io::stdout(),
            width: 80,
            height: 25,
        };

        // Get terminal size
        term.update_size();

        // Enable raw mode
        term.enable_raw_mode()?;

        // Setup terminal
        term.write_raw("\x1b[?25l")?; // Hide cursor
        term.write_raw("\x1b[?1003h")?; // Enable any-event mouse tracking (motion without buttons)
        term.write_raw("\x1b[?1006h")?; // Enable SGR extended mouse mode
        term.write_raw("\x1b[>4;2m")?; // Enable modifyOtherKeys mode 2 (xterm)
        term.write_raw("\x1b[>1u")?; // Enable Kitty keyboard protocol
        term.write_raw("\x1b[2J")?; // Clear screen
        term.write_raw("\x1b[H")?; // Home cursor

        Ok(term)
    }

    /// Get terminal dimensions
    pub fn size(&self) -> (u16, u16) {
        (self.width, self.height)
    }

    /// Update terminal size from ioctl
    pub fn update_size(&mut self) {
        unsafe {
            let mut ws: libc::winsize = std::mem::zeroed();
            if libc::ioctl(libc::STDOUT_FILENO, libc::TIOCGWINSZ, &mut ws) == 0 {
                self.width = ws.ws_col;
                self.height = ws.ws_row;
            }
        }
    }

    /// Enable raw mode (disable canonical mode, echo, etc.)
    fn enable_raw_mode(&self) -> io::Result<()> {
        unsafe {
            let fd = io::stdin().as_raw_fd();
            let mut termios: libc::termios = std::mem::zeroed();

            if libc::tcgetattr(fd, &mut termios) != 0 {
                return Err(io::Error::last_os_error());
            }

            // Save original settings
            ORIG_TERMIOS = Some(termios);

            // Modify for raw mode
            termios.c_lflag &= !(libc::ECHO | libc::ICANON | libc::ISIG | libc::IEXTEN);
            termios.c_iflag &= !(libc::IXON | libc::ICRNL | libc::BRKINT | libc::INPCK | libc::ISTRIP);
            termios.c_oflag &= !libc::OPOST;
            termios.c_cflag |= libc::CS8;

            // Set minimum chars and timeout for read
            // VMIN=0, VTIME=0 means non-blocking read
            termios.c_cc[libc::VMIN] = 0;
            termios.c_cc[libc::VTIME] = 0;

            if libc::tcsetattr(fd, libc::TCSAFLUSH, &termios) != 0 {
                return Err(io::Error::last_os_error());
            }
        }
        Ok(())
    }

    /// Disable raw mode (restore original settings)
    fn disable_raw_mode(&self) -> io::Result<()> {
        unsafe {
            if let Some(orig) = ORIG_TERMIOS {
                let fd = io::stdin().as_raw_fd();
                if libc::tcsetattr(fd, libc::TCSAFLUSH, &orig) != 0 {
                    return Err(io::Error::last_os_error());
                }
            }
        }
        Ok(())
    }

    /// Write raw bytes to terminal
    pub fn write_raw(&mut self, s: &str) -> io::Result<()> {
        self.stdout.write_all(s.as_bytes())?;
        Ok(())
    }

    /// Flush output buffer
    pub fn flush(&mut self) -> io::Result<()> {
        self.stdout.flush()
    }

    /// Move cursor to position (1-based, like ANSI)
    pub fn goto(&mut self, row: u16, col: u16) -> io::Result<()> {
        write!(self.stdout, "\x1b[{};{}H", row, col)?;
        Ok(())
    }

    /// Set foreground and background colors
    pub fn set_colors(&mut self, fg: Color, bg: Color) -> io::Result<()> {
        write!(self.stdout, "\x1b[{};{}m", fg.to_fg_sgr(), bg.to_bg_sgr())?;
        Ok(())
    }

    /// Reset colors to default
    pub fn reset_colors(&mut self) -> io::Result<()> {
        self.write_raw("\x1b[0m")
    }

    /// Clear entire screen
    pub fn clear(&mut self) -> io::Result<()> {
        self.write_raw("\x1b[2J\x1b[H")
    }

    /// Clear to end of line
    #[allow(dead_code)]
    pub fn clear_eol(&mut self) -> io::Result<()> {
        self.write_raw("\x1b[K")
    }

    /// Show cursor
    pub fn show_cursor(&mut self) -> io::Result<()> {
        self.write_raw("\x1b[?25h")
    }

    /// Hide cursor
    pub fn hide_cursor(&mut self) -> io::Result<()> {
        self.write_raw("\x1b[?25l")
    }

    /// Set cursor style (block, underline, bar - each with blinking or steady)
    pub fn set_cursor_style(&mut self, style: CursorStyle) -> io::Result<()> {
        let code = match style {
            CursorStyle::BlinkingBlock => "\x1b[1 q",
            CursorStyle::SteadyBlock => "\x1b[2 q",
            CursorStyle::BlinkingUnderline => "\x1b[3 q",
            CursorStyle::SteadyUnderline => "\x1b[4 q",
            CursorStyle::BlinkingBar => "\x1b[5 q",
            CursorStyle::SteadyBar => "\x1b[6 q",
        };
        self.write_raw(code)
    }

    /// Read a key from input (non-blocking)
    #[allow(dead_code)]
    pub fn read_key(&self) -> io::Result<Option<Key>> {
        let (key, _bytes) = self.read_key_raw()?;
        Ok(key)
    }

    /// Read key and return both the parsed key and raw bytes (for debugging)
    pub fn read_key_raw(&self) -> io::Result<(Option<Key>, Vec<u8>)> {
        let mut buf = [0u8; 32];
        let mut stdin = io::stdin();

        let n = stdin.read(&mut buf)?;
        if n == 0 {
            return Ok((None, vec![]));
        }

        // If we got ESC, try to read more bytes (for escape sequences)
        let mut total = n;
        if buf[0] == 0x1b && n == 1 {
            // Wait a bit and try to read more
            std::thread::sleep(std::time::Duration::from_millis(10));
            if let Ok(more) = stdin.read(&mut buf[n..]) {
                total += more;
            }
        }

        let bytes = buf[..total].to_vec();
        Ok((Some(Self::parse_key(&buf[..total])), bytes))
    }

    /// Parse raw bytes into a Key
    fn parse_key(buf: &[u8]) -> Key {
        // Check for SGR mouse events: \x1b[<Cb;Cx;CyM or \x1b[<Cb;Cx;Cym
        if buf.len() >= 6 && buf[0] == 0x1b && buf[1] == b'[' && buf[2] == b'<' {
            if let Some(mouse) = Self::parse_sgr_mouse(buf) {
                return Key::Mouse(mouse);
            }
        }

        match buf {
            // Single characters
            [b'\r'] | [b'\n'] => Key::Enter,
            [0x1b] => Key::Escape,
            [0x7f] | [0x08] => Key::Backspace,
            [b'\t'] => Key::Tab,
            [0x00] => Key::CtrlSpace, // Ctrl+Space sends NUL

            // Ctrl+letter (0x01-0x1a = Ctrl+A through Ctrl+Z)
            [c] if *c >= 1 && *c <= 26 => Key::Ctrl((b'a' + c - 1) as char),

            // Regular printable character
            [c] if *c >= 32 && *c < 127 => Key::Char(*c as char),

            // UTF-8 sequences (2-4 bytes)
            _ if buf.len() >= 2 && buf[0] >= 0xC0 => {
                if let Ok(s) = std::str::from_utf8(buf) {
                    if let Some(c) = s.chars().next() {
                        return Key::Char(c);
                    }
                }
                Key::Unknown(buf.to_vec())
            }

            // Shift+Tab (backtab)
            [0x1b, b'[', b'Z'] => Key::ShiftTab,

            // Escape sequences
            [0x1b, b'[', b'A'] => Key::Up,
            [0x1b, b'[', b'B'] => Key::Down,
            [0x1b, b'[', b'C'] => Key::Right,
            [0x1b, b'[', b'D'] => Key::Left,
            [0x1b, b'[', b'H'] => Key::Home,
            [0x1b, b'[', b'F'] => Key::End,
            [0x1b, b'[', b'1', b'~'] => Key::Home,
            [0x1b, b'[', b'4', b'~'] => Key::End,
            [0x1b, b'[', b'2', b'~'] => Key::Insert,
            [0x1b, b'[', b'3', b'~'] => Key::Delete,
            [0x1b, b'[', b'5', b'~'] => Key::PageUp,
            [0x1b, b'[', b'6', b'~'] => Key::PageDown,

            // Shift+Arrow keys (modifier 2 = shift)
            [0x1b, b'[', b'1', b';', b'2', b'A'] => Key::ShiftUp,
            [0x1b, b'[', b'1', b';', b'2', b'B'] => Key::ShiftDown,
            [0x1b, b'[', b'1', b';', b'2', b'C'] => Key::ShiftRight,
            [0x1b, b'[', b'1', b';', b'2', b'D'] => Key::ShiftLeft,
            [0x1b, b'[', b'1', b';', b'2', b'H'] => Key::ShiftHome,
            [0x1b, b'[', b'1', b';', b'2', b'F'] => Key::ShiftEnd,
            // Shift+Space (kitty/xterm extended keyboard protocols)
            // CSI 32 ; 2 u  or  CSI 27 ; 2 ; 32 ~
            [0x1b, b'[', b'3', b'2', b';', b'2', b'u'] => Key::ShiftSpace,
            [0x1b, b'[', b'2', b'7', b';', b'2', b';', b'3', b'2', b'~'] => Key::ShiftSpace,

            // Ctrl+Arrow keys (modifier 5 = ctrl)
            [0x1b, b'[', b'1', b';', b'5', b'A'] => Key::CtrlUp,
            [0x1b, b'[', b'1', b';', b'5', b'B'] => Key::CtrlDown,
            [0x1b, b'[', b'1', b';', b'5', b'C'] => Key::CtrlRight,
            [0x1b, b'[', b'1', b';', b'5', b'D'] => Key::CtrlLeft,
            [0x1b, b'[', b'1', b';', b'5', b'H'] => Key::CtrlHome,
            [0x1b, b'[', b'1', b';', b'5', b'F'] => Key::CtrlEnd,
            [0x1b, b'[', b'5', b';', b'5', b'~'] => Key::CtrlPageUp,
            [0x1b, b'[', b'6', b';', b'5', b'~'] => Key::CtrlPageDown,

            // Ctrl+Backspace and Ctrl+Delete
            [0x1f] => Key::CtrlBackspace,  // Ctrl+Backspace sends 0x1f
            [0x1b, b'[', b'3', b';', b'5', b'~'] => Key::CtrlDelete,

            // Ctrl+Shift+Arrow keys (modifier 6 = ctrl+shift)
            [0x1b, b'[', b'1', b';', b'6', b'C'] => Key::CtrlShiftRight,
            [0x1b, b'[', b'1', b';', b'6', b'D'] => Key::CtrlShiftLeft,
            [0x1b, b'[', b'1', b';', b'6', b'H'] => Key::CtrlShiftHome,
            [0x1b, b'[', b'1', b';', b'6', b'F'] => Key::CtrlShiftEnd,

            // Ctrl+Shift+K (delete line)
            [0x0b] => Key::CtrlShiftK, // Ctrl+K sends 0x0b, Ctrl+Shift+K may vary

            // Alt+Arrow keys (modifier 3 = alt)
            [0x1b, b'[', b'1', b';', b'3', b'A'] => Key::AltUp,
            [0x1b, b'[', b'1', b';', b'3', b'B'] => Key::AltDown,
            // Alt+arrows without modifier (ESC + arrow)
            [0x1b, 0x1b, b'[', b'A'] => Key::AltUp,
            [0x1b, 0x1b, b'[', b'B'] => Key::AltDown,

            // Function keys
            [0x1b, b'O', b'P'] | [0x1b, b'[', b'1', b'1', b'~'] => Key::F(1),
            [0x1b, b'O', b'Q'] | [0x1b, b'[', b'1', b'2', b'~'] => Key::F(2),
            [0x1b, b'O', b'R'] | [0x1b, b'[', b'1', b'3', b'~'] => Key::F(3),
            [0x1b, b'O', b'S'] | [0x1b, b'[', b'1', b'4', b'~'] => Key::F(4),
            [0x1b, b'[', b'1', b'5', b'~'] => Key::F(5),
            [0x1b, b'[', b'1', b'7', b'~'] => Key::F(6),
            [0x1b, b'[', b'1', b'8', b'~'] => Key::F(7),
            [0x1b, b'[', b'1', b'9', b'~'] => Key::F(8),
            [0x1b, b'[', b'2', b'0', b'~'] => Key::F(9),
            [0x1b, b'[', b'2', b'1', b'~'] => Key::F(10),
            [0x1b, b'[', b'2', b'3', b'~'] => Key::F(11),
            [0x1b, b'[', b'2', b'4', b'~'] => Key::F(12),

            // Alt+letter (ESC followed by letter)
            [0x1b, c] if *c >= b'a' && *c <= b'z' => Key::Alt(*c as char),
            [0x1b, c] if *c >= b'A' && *c <= b'Z' => Key::Alt((*c as char).to_ascii_lowercase()),

            _ => Key::Unknown(buf.to_vec()),
        }
    }

    /// Parse SGR extended mouse format: \x1b[<Cb;Cx;CyM or \x1b[<Cb;Cx;Cym
    fn parse_sgr_mouse(buf: &[u8]) -> Option<MouseEvent> {
        // Convert to string for easier parsing
        let s = std::str::from_utf8(buf).ok()?;
        if !s.starts_with("\x1b[<") {
            return None;
        }

        let content = &s[3..]; // Skip "\x1b[<"
        let pressed = content.ends_with('M');
        let content = content.trim_end_matches(|c| c == 'M' || c == 'm');

        let parts: Vec<&str> = content.split(';').collect();
        if parts.len() != 3 {
            return None;
        }

        let cb: u8 = parts[0].parse().ok()?;
        let col: u16 = parts[1].parse().ok()?;
        let row: u16 = parts[2].parse().ok()?;

        let button = match cb & 0b11 {
            0 => MouseButton::Left,
            1 => MouseButton::Middle,
            2 => MouseButton::Right,
            3 => MouseButton::None, // No button (motion without press)
            _ => return None,
        };

        // Check for motion (bit 5 = 32)
        let motion = (cb & 32) != 0;

        // Check for wheel events
        let button = if cb & 64 != 0 {
            if cb & 1 != 0 {
                MouseButton::WheelDown
            } else {
                MouseButton::WheelUp
            }
        } else {
            button
        };

        Some(MouseEvent {
            button,
            row,
            col,
            pressed,
            motion,
        })
    }

    /// Write a string at current position
    #[allow(dead_code)]
    pub fn write_str(&mut self, s: &str) -> io::Result<()> {
        self.stdout.write_all(s.as_bytes())?;
        Ok(())
    }

    /// Write a character at current position
    pub fn write_char(&mut self, c: char) -> io::Result<()> {
        write!(self.stdout, "{}", c)?;
        Ok(())
    }

    /// Draw a box with single-line border
    #[allow(dead_code)]
    pub fn draw_box(&mut self, row: u16, col: u16, width: u16, height: u16, fg: Color, bg: Color) -> io::Result<()> {
        self.set_colors(fg, bg)?;

        // Top border
        self.goto(row, col)?;
        self.write_char('┌')?;
        for _ in 0..width.saturating_sub(2) {
            self.write_char('─')?;
        }
        self.write_char('┐')?;

        // Side borders
        for r in 1..height.saturating_sub(1) {
            self.goto(row + r, col)?;
            self.write_char('│')?;
            for _ in 0..width.saturating_sub(2) {
                self.write_char(' ')?;
            }
            self.write_char('│')?;
        }

        // Bottom border
        self.goto(row + height - 1, col)?;
        self.write_char('└')?;
        for _ in 0..width.saturating_sub(2) {
            self.write_char('─')?;
        }
        self.write_char('┘')?;

        Ok(())
    }

    /// Draw a double-line box (for dialogs)
    #[allow(dead_code)]
    pub fn draw_double_box(&mut self, row: u16, col: u16, width: u16, height: u16, fg: Color, bg: Color) -> io::Result<()> {
        self.set_colors(fg, bg)?;

        // Top border
        self.goto(row, col)?;
        self.write_char('╔')?;
        for _ in 0..width.saturating_sub(2) {
            self.write_char('═')?;
        }
        self.write_char('╗')?;

        // Side borders
        for r in 1..height.saturating_sub(1) {
            self.goto(row + r, col)?;
            self.write_char('║')?;
            for _ in 0..width.saturating_sub(2) {
                self.write_char(' ')?;
            }
            self.write_char('║')?;
        }

        // Bottom border
        self.goto(row + height - 1, col)?;
        self.write_char('╚')?;
        for _ in 0..width.saturating_sub(2) {
            self.write_char('═')?;
        }
        self.write_char('╝')?;

        Ok(())
    }

    /// Fill a rectangular area with a character
    #[allow(dead_code)]
    pub fn fill_rect(&mut self, row: u16, col: u16, width: u16, height: u16, c: char, fg: Color, bg: Color) -> io::Result<()> {
        self.set_colors(fg, bg)?;
        for r in 0..height {
            self.goto(row + r, col)?;
            for _ in 0..width {
                self.write_char(c)?;
            }
        }
        Ok(())
    }

    /// Draw shadow effect (right and bottom edges)
    #[allow(dead_code)]
    pub fn draw_shadow(&mut self, row: u16, col: u16, width: u16, height: u16) -> io::Result<()> {
        self.set_colors(Color::Black, Color::Black)?;

        // Right shadow
        for r in 1..=height {
            self.goto(row + r, col + width)?;
            self.write_str("  ")?;
        }

        // Bottom shadow
        self.goto(row + height, col + 2)?;
        for _ in 0..width {
            self.write_char(' ')?;
        }

        Ok(())
    }
}

impl Drop for Terminal {
    fn drop(&mut self) {
        // Disable extended keyboard protocols
        let _ = self.write_raw("\x1b[<u"); // Disable Kitty keyboard protocol
        let _ = self.write_raw("\x1b[>4;0m"); // Disable modifyOtherKeys
        // Disable mouse tracking
        let _ = self.write_raw("\x1b[?1006l");
        let _ = self.write_raw("\x1b[?1003l");
        // Restore terminal state
        let _ = self.write_raw("\x1b[0 q"); // Reset cursor to terminal default
        let _ = self.show_cursor();
        let _ = self.reset_colors();
        let _ = self.clear();
        let _ = self.goto(1, 1);
        let _ = self.flush();
        let _ = self.disable_raw_mode();
    }
}

#[derive(Clone, Copy, Debug)]
#[allow(dead_code)]
pub enum CursorStyle {
    BlinkingBlock,
    SteadyBlock,
    BlinkingUnderline,
    SteadyUnderline,
    BlinkingBar,
    SteadyBar,
}
