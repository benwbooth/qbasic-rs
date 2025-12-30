//! QBasic-style menu bar

use crate::screen::Screen;
use crate::terminal::Color;
use crate::state::AppState;
use super::layout::Rect;

/// Menu item definition
#[derive(Clone)]
pub struct MenuItem {
    pub label: String,
    pub shortcut: Option<String>,
    pub enabled: bool,
    pub separator: bool,
}

impl MenuItem {
    pub fn new(label: &str, shortcut: Option<&str>) -> Self {
        Self {
            label: label.to_string(),
            shortcut: shortcut.map(|s| s.to_string()),
            enabled: true,
            separator: false,
        }
    }

    pub fn separator() -> Self {
        Self {
            label: String::new(),
            shortcut: None,
            enabled: false,
            separator: true,
        }
    }

    pub fn disabled(mut self) -> Self {
        self.enabled = false;
        self
    }
}

/// Menu definition
#[derive(Clone)]
pub struct Menu {
    pub title: String,
    pub hotkey: char,
    pub items: Vec<MenuItem>,
}

impl Menu {
    pub fn new(title: &str, hotkey: char) -> Self {
        Self {
            title: title.to_string(),
            hotkey,
            items: Vec::new(),
        }
    }

    pub fn item(mut self, label: &str, shortcut: Option<&str>) -> Self {
        self.items.push(MenuItem::new(label, shortcut));
        self
    }

    pub fn separator(mut self) -> Self {
        self.items.push(MenuItem::separator());
        self
    }

    pub fn width(&self) -> u16 {
        let max_label = self.items.iter()
            .filter(|i| !i.separator)
            .map(|i| i.label.len())
            .max()
            .unwrap_or(10);
        let max_shortcut = self.items.iter()
            .filter_map(|i| i.shortcut.as_ref())
            .map(|s| s.len())
            .max()
            .unwrap_or(0);

        (max_label + max_shortcut + 4).max(self.title.len() + 4) as u16
    }
}

/// The menu bar component
pub struct MenuBar {
    pub menus: Vec<Menu>,
}

impl MenuBar {
    /// Create the standard QBasic menu bar
    pub fn new() -> Self {
        Self {
            menus: vec![
                Menu::new("File", 'F')
                    .item("New", Some(""))
                    .item("Open...", Some(""))
                    .item("Save", Some(""))
                    .item("Save As...", Some(""))
                    .separator()
                    .item("Print...", Some(""))
                    .separator()
                    .item("Exit", Some("Alt+X")),

                Menu::new("Edit", 'E')
                    .item("Undo", Some("Ctrl+Z"))
                    .separator()
                    .item("Cut", Some("Ctrl+X"))
                    .item("Copy", Some("Ctrl+C"))
                    .item("Paste", Some("Ctrl+V"))
                    .item("Clear", Some("Del"))
                    .separator()
                    .item("New SUB...", Some(""))
                    .item("New FUNCTION...", Some("")),

                Menu::new("View", 'V')
                    .item("SUBs...", Some("F2"))
                    .item("Next Statement", Some(""))
                    .item("Output Screen", Some("F4"))
                    .separator()
                    .item("Included File", Some(""))
                    .item("Included Lines", Some("")),

                Menu::new("Search", 'S')
                    .item("Find...", Some("Ctrl+F"))
                    .item("Repeat Last Find", Some("F3"))
                    .item("Change...", Some(""))
                    .item("Label...", Some("")),

                Menu::new("Run", 'R')
                    .item("Start", Some("F5"))
                    .item("Restart", Some("Shift+F5"))
                    .item("Continue", Some("F5"))
                    .separator()
                    .item("Modify COMMAND$...", Some("")),

                Menu::new("Debug", 'D')
                    .item("Step", Some("F8"))
                    .item("Procedure Step", Some("F10"))
                    .separator()
                    .item("Toggle Breakpoint", Some("F9"))
                    .item("Clear All Breakpoints", Some(""))
                    .separator()
                    .item("Set Next Statement", Some("")),

                Menu::new("Options", 'O')
                    .item("Display...", Some(""))
                    .item("Help Path...", Some(""))
                    .item("Syntax Checking", Some("")),

                Menu::new("Help", 'H')
                    .item("Index", Some(""))
                    .item("Contents", Some(""))
                    .item("Topic:", Some("F1"))
                    .item("Using Help", Some(""))
                    .separator()
                    .item("About...", Some("")),
            ],
        }
    }

    /// Draw the menu bar
    pub fn draw(&self, screen: &mut Screen, state: &AppState, bounds: Rect) {
        let row = bounds.y + 1; // 1-based row
        let width = bounds.width;

        // Menu bar background (light gray like QBasic)
        screen.fill(row, bounds.x + 1, width, 1, ' ', Color::Black, Color::LightGray);

        // Draw each menu title
        let mut x = bounds.x + 2;
        for (i, menu) in self.menus.iter().enumerate() {
            let is_selected = state.menu_open && state.menu_index == i;

            let fg = if is_selected { Color::White } else { Color::Black };
            let bg = if is_selected { Color::Black } else { Color::LightGray };

            // Space before
            screen.set(row, x, ' ', fg, bg);
            x += 1;

            // Title with hotkey highlighting
            for (j, ch) in menu.title.chars().enumerate() {
                let ch_fg = if ch.to_ascii_lowercase() == menu.hotkey.to_ascii_lowercase() {
                    Color::White
                } else {
                    fg
                };
                screen.set(row, x + j as u16, ch, ch_fg, bg);
            }
            x += menu.title.len() as u16;

            // Space after
            screen.set(row, x, ' ', fg, bg);
            x += 2; // Extra space between menus
        }
    }

    /// Draw the dropdown menu (call after other UI elements)
    pub fn draw_dropdown(&self, screen: &mut Screen, state: &AppState) {
        if !state.menu_open {
            return;
        }
        let menu = &self.menus[state.menu_index];

        // Calculate position
        let mut x = 1u16;
        for i in 0..state.menu_index {
            x += self.menus[i].title.len() as u16 + 3;
        }

        let width = menu.width();
        let height = menu.items.len() as u16 + 2;
        let y = 2u16;

        // Draw box
        screen.draw_box(y, x, width, height, Color::Black, Color::White);

        // Draw shadow
        screen.draw_shadow(y, x, width, height);

        // Draw items
        for (i, item) in menu.items.iter().enumerate() {
            let row = y + 1 + i as u16;
            let is_selected = state.menu_item == i && !item.separator;

            if item.separator {
                // Draw separator
                screen.set(row, x, '├', Color::Black, Color::White);
                for c in 1..width - 1 {
                    screen.set(row, x + c, '─', Color::Black, Color::White);
                }
                screen.set(row, x + width - 1, '┤', Color::Black, Color::White);
            } else {
                let fg = if is_selected { Color::White } else { Color::Black };
                let bg = if is_selected { Color::Black } else { Color::White };

                // Clear the row
                for c in 1..width - 1 {
                    screen.set(row, x + c, ' ', fg, bg);
                }

                // Item label (with hotkey)
                let mut label_x = x + 2;
                for ch in item.label.chars() {
                    screen.set(row, label_x, ch, fg, bg);
                    label_x += 1;
                }

                // Shortcut (right-aligned)
                if let Some(shortcut) = &item.shortcut {
                    let shortcut_x = x + width - 2 - shortcut.len() as u16;
                    for (j, ch) in shortcut.chars().enumerate() {
                        screen.set(row, shortcut_x + j as u16, ch, Color::DarkGray, bg);
                    }
                }
            }
        }
    }

    /// Handle menu navigation, returns true if handled
    pub fn handle_input(&self, state: &mut AppState, event: &crate::input::InputEvent) -> Option<MenuAction> {
        use crate::input::InputEvent;

        match event {
            InputEvent::Escape => {
                state.close_menu();
                Some(MenuAction::Close)
            }
            InputEvent::Enter => {
                let menu = &self.menus[state.menu_index];
                let item = &menu.items[state.menu_item];
                if !item.separator && item.enabled {
                    state.close_menu();
                    Some(MenuAction::Execute(state.menu_index, state.menu_item))
                } else {
                    None
                }
            }
            InputEvent::CursorUp => {
                let menu = &self.menus[state.menu_index];
                loop {
                    if state.menu_item == 0 {
                        state.menu_item = menu.items.len() - 1;
                    } else {
                        state.menu_item -= 1;
                    }
                    if !menu.items[state.menu_item].separator {
                        break;
                    }
                }
                Some(MenuAction::Navigate)
            }
            InputEvent::CursorDown => {
                let menu = &self.menus[state.menu_index];
                loop {
                    state.menu_item = (state.menu_item + 1) % menu.items.len();
                    if !menu.items[state.menu_item].separator {
                        break;
                    }
                }
                Some(MenuAction::Navigate)
            }
            InputEvent::CursorLeft => {
                if state.menu_index == 0 {
                    state.menu_index = self.menus.len() - 1;
                } else {
                    state.menu_index -= 1;
                }
                state.menu_item = 0;
                // Skip to first non-separator
                let menu = &self.menus[state.menu_index];
                while menu.items[state.menu_item].separator {
                    state.menu_item += 1;
                }
                Some(MenuAction::Navigate)
            }
            InputEvent::CursorRight => {
                state.menu_index = (state.menu_index + 1) % self.menus.len();
                state.menu_item = 0;
                // Skip to first non-separator
                let menu = &self.menus[state.menu_index];
                while menu.items[state.menu_item].separator {
                    state.menu_item += 1;
                }
                Some(MenuAction::Navigate)
            }
            _ => None,
        }
    }
}

impl Default for MenuBar {
    fn default() -> Self {
        Self::new()
    }
}

/// Action result from menu interaction
#[derive(Clone, Debug)]
pub enum MenuAction {
    Close,
    Navigate,
    Execute(usize, usize), // (menu_index, item_index)
}
