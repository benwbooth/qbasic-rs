//! QBasic-style status bar

use crate::screen::Screen;
use crate::terminal::Color;
use crate::state::{AppState, EditorMode, RunState};
use super::layout::Rect;

/// The status bar at the bottom of the screen (stateless)
pub struct StatusBar;

impl StatusBar {
    pub fn new() -> Self {
        Self
    }
}

impl StatusBar {
    /// Draw the status bar
    pub fn draw(screen: &mut Screen, state: &AppState, cursor_line: usize, cursor_col: usize, bounds: Rect) {
        let row = bounds.y + 1; // 1-based row
        let width = bounds.width;
        let col = bounds.x + 1;

        // Background - cyan with white text like real QBasic
        screen.fill(row, col, width, 1, ' ', Color::White, Color::Cyan);

        // F1=Help shortcut on left
        screen.write_str(row, col, " <F1=Help>", Color::White, Color::Cyan);

        // Status message or run state in center
        let center_text = if let Some(msg) = &state.status_message {
            msg.clone()
        } else {
            match state.run_state {
                RunState::Editing => String::new(),
                RunState::Running => " Running... ".to_string(),
                RunState::WaitingForInput => " Running... ".to_string(),
                RunState::Paused => " Paused ".to_string(),
                RunState::Stepping => " Step ".to_string(),
                RunState::Finished => " Finished ".to_string(),
            }
        };

        if !center_text.is_empty() {
            let center_x = col + (width.saturating_sub(center_text.len() as u16)) / 2;
            screen.write_str(row, center_x, &center_text, Color::White, Color::Cyan);
        }

        // Right side: line:col and insert/overwrite mode
        let mode_str = match state.editor_mode {
            EditorMode::Insert => "INS",
            EditorMode::Overwrite => "OVR",
        };

        // Format: "00001:001" for line:col
        let pos_str = format!("{:05}:{:03}", cursor_line + 1, cursor_col + 1);
        let right_text = format!("{}  {}", pos_str, mode_str);
        let right_x = col + width.saturating_sub(right_text.len() as u16);

        // Draw vertical separator 3 chars left of position info
        let sep_x = right_x.saturating_sub(3);
        screen.draw_vrule(row, sep_x, Color::White, Color::Cyan);

        screen.write_str(row, right_x, &right_text, Color::White, Color::Cyan);
    }

    /// Draw the function key bar (optional, at very bottom)
    #[allow(dead_code)]
    pub fn draw_key_bar(screen: &mut Screen, row: u16, width: u16) {
        // Background
        screen.fill(row, 1, width, 1, ' ', Color::Black, Color::LightGray);

        // Function key hints
        let keys = [
            ("F1", "Help"),
            ("F2", "Subs"),
            ("F5", "Run"),
            ("F6", "Window"),
            ("F8", "Step"),
            ("F9", "Break"),
        ];

        let mut x = 1u16;
        for (key, desc) in keys.iter() {
            // Key name in black on white
            screen.write_str(row, x, key, Color::White, Color::Black);
            x += key.len() as u16;

            // Description in black on light gray
            screen.write_str(row, x, desc, Color::Black, Color::LightGray);
            x += desc.len() as u16 + 1;
        }
    }
}

impl Default for StatusBar {
    fn default() -> Self {
        Self::new()
    }
}

// Note: StatusBar does NOT implement MainWidget because it needs cursor info from Editor.
// It is drawn specially by the Widgets container which has access to both Editor and StatusBar.
