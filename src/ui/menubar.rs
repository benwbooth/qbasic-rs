//! QBasic-style menu bar

use crate::screen::Screen;
use crate::terminal::Color;
use crate::state::AppState;
use super::layout::{Rect, LayoutItem, ComputedLayout, compute_layout};

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

    #[allow(dead_code)]
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
            .filter(|s| !s.is_empty())
            .map(|s| s.len())
            .max()
            .unwrap_or(0);

        // 4 for borders + 2 for label padding + 2 for gap between label and shortcut
        let shortcut_space = if max_shortcut > 0 { max_shortcut + 2 } else { 0 };
        (max_label + shortcut_space + 6).max(self.title.len() + 4) as u16
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
                    .item("New Program", Some(""))
                    .item("Open Program...", Some(""))
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

            // Title with hotkey highlighting (first match only)
            let mut hotkey_highlighted = false;
            for (j, ch) in menu.title.chars().enumerate() {
                let ch_fg = if !hotkey_highlighted && ch.to_ascii_lowercase() == menu.hotkey.to_ascii_lowercase() {
                    hotkey_highlighted = true;
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
        screen.draw_box(y, x, width, height, Color::Black, Color::LightGray);

        // Draw shadow
        screen.draw_shadow(y, x, width, height);

        // Draw items
        for (i, item) in menu.items.iter().enumerate() {
            let row = y + 1 + i as u16;
            let is_selected = state.menu_item == i && !item.separator;

            if item.separator {
                // Draw separator
                screen.set(row, x, '├', Color::Black, Color::LightGray);
                for c in 1..width - 1 {
                    screen.set(row, x + c, '─', Color::Black, Color::LightGray);
                }
                screen.set(row, x + width - 1, '┤', Color::Black, Color::LightGray);
            } else {
                let fg = if is_selected { Color::White } else { Color::Black };
                let bg = if is_selected { Color::Black } else { Color::LightGray };

                // Clear the row
                for c in 1..width - 1 {
                    screen.set(row, x + c, ' ', fg, bg);
                }

                // Item label with first letter as hotkey (highlighted in white)
                let mut label_x = x + 2;
                for (j, ch) in item.label.chars().enumerate() {
                    let ch_fg = if j == 0 {
                        Color::White
                    } else {
                        fg
                    };
                    screen.set(row, label_x, ch, ch_fg, bg);
                    label_x += 1;
                }

                // Shortcut (right-aligned) - same color as label
                if let Some(shortcut) = &item.shortcut {
                    if !shortcut.is_empty() {
                        let shortcut_x = x + width - 2 - shortcut.len() as u16;
                        for (j, ch) in shortcut.chars().enumerate() {
                            screen.set(row, shortcut_x + j as u16, ch, fg, bg);
                        }
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

/// Result of handling a menu click using the layout system
#[derive(Clone, Debug, PartialEq)]
pub enum MenuClickResult {
    /// No action taken
    None,
    /// Open menu at index
    OpenMenu(usize),
    /// Close the menu
    CloseMenu,
    /// Execute menu item (menu_index, item_index)
    Execute(usize, usize),
    /// Click was absorbed (on border, separator, etc.)
    Absorbed,
}

impl MenuBar {
    /// Build a layout for the menu bar titles
    /// Returns an HStack of menu title leaves named "menu_0", "menu_1", etc.
    pub fn titles_layout(&self) -> LayoutItem {
        let children: Vec<LayoutItem> = self.menus.iter().enumerate()
            .map(|(i, menu)| {
                // Each menu title has a space before, the title, and a space after
                LayoutItem::leaf(format!("menu_{}", i))
                    .fixed_width(menu.title.len() as u16 + 2)
                    .fixed_height(1)
            })
            .collect();

        LayoutItem::hstack(children).spacing(1).fixed_height(1)
    }

    /// Compute the menu bar layout within the given bounds
    pub fn compute_titles_layout(&self, bounds: Rect) -> ComputedLayout {
        compute_layout(&self.titles_layout(), bounds)
    }

    /// Build a layout for a dropdown menu
    /// Returns a VStack of item leaves named "item_0", "item_1", etc.
    pub fn dropdown_layout(&self, menu_index: usize) -> LayoutItem {
        let menu = &self.menus[menu_index];
        let children: Vec<LayoutItem> = menu.items.iter().enumerate()
            .map(|(i, _item)| {
                LayoutItem::leaf(format!("item_{}", i))
                    .fixed_height(1)
            })
            .collect();

        LayoutItem::vstack(children)
            .fixed_width(menu.width().saturating_sub(2)) // Subtract borders
            .fixed_height(menu.items.len() as u16)
    }

    /// Get the bounds for a dropdown menu (1-based screen coordinates)
    pub fn dropdown_bounds(&self, menu_index: usize) -> Rect {
        // Calculate x position by summing widths of previous menus
        let mut x = 2u16; // Start at column 2 (1 for padding + 1 for border offset)
        for i in 0..menu_index {
            x += self.menus[i].title.len() as u16 + 3; // title + spaces + spacing
        }

        let menu = &self.menus[menu_index];
        Rect {
            x,
            y: 2, // Just below menu bar (row 2, 0-based)
            width: menu.width(),
            height: menu.items.len() as u16 + 2, // +2 for borders
        }
    }

    /// Compute the dropdown layout within its bounds
    pub fn compute_dropdown_layout(&self, menu_index: usize) -> ComputedLayout {
        let bounds = self.dropdown_bounds(menu_index);
        // Content area is inside the borders
        let content_bounds = Rect {
            x: bounds.x + 1,
            y: bounds.y + 1,
            width: bounds.width.saturating_sub(2),
            height: bounds.height.saturating_sub(2),
        };
        compute_layout(&self.dropdown_layout(menu_index), content_bounds)
    }

    /// Handle a click on the menu bar area
    /// bounds should be the menu_bar rect from the main layout (0-based)
    /// row, col are 1-based screen coordinates
    pub fn handle_bar_click(&self, row: u16, col: u16, bounds: Rect, state: &AppState) -> MenuClickResult {
        // Convert bounds to 1-based for our layout computation
        let layout_bounds = Rect {
            x: bounds.x + 2, // Start at column 2 for the first menu
            y: bounds.y + 1, // 1-based row
            width: bounds.width.saturating_sub(2),
            height: 1,
        };

        let layout = self.compute_titles_layout(layout_bounds);

        // Check which menu title was clicked
        if let Some(hit_id) = layout.hit_test(row, col) {
            if let Some(idx_str) = hit_id.strip_prefix("menu_") {
                if let Ok(idx) = idx_str.parse::<usize>() {
                    return MenuClickResult::OpenMenu(idx);
                }
            }
        }

        // Clicked on menu bar but not on a menu title
        if state.menu_open {
            MenuClickResult::CloseMenu
        } else {
            MenuClickResult::None
        }
    }

    /// Handle a click when a dropdown is open
    /// row, col are 1-based screen coordinates
    pub fn handle_dropdown_click(&self, row: u16, col: u16, state: &AppState) -> MenuClickResult {
        if !state.menu_open {
            return MenuClickResult::None;
        }

        let bounds = self.dropdown_bounds(state.menu_index);

        // Check if click is outside the dropdown entirely
        if row < bounds.y || row >= bounds.y + bounds.height
            || col < bounds.x || col >= bounds.x + bounds.width
        {
            return MenuClickResult::CloseMenu;
        }

        // Check if click is on the border
        if row == bounds.y || row == bounds.y + bounds.height - 1
            || col == bounds.x || col == bounds.x + bounds.width - 1
        {
            return MenuClickResult::Absorbed;
        }

        // Click is in content area - use layout for hit testing
        let layout = self.compute_dropdown_layout(state.menu_index);

        if let Some(hit_id) = layout.hit_test(row, col) {
            if let Some(idx_str) = hit_id.strip_prefix("item_") {
                if let Ok(idx) = idx_str.parse::<usize>() {
                    let menu = &self.menus[state.menu_index];
                    if idx < menu.items.len() {
                        if menu.items[idx].separator {
                            return MenuClickResult::Absorbed;
                        } else {
                            return MenuClickResult::Execute(state.menu_index, idx);
                        }
                    }
                }
            }
        }

        MenuClickResult::Absorbed
    }
}

// Implement MainWidget trait
use super::main_widget::{MainWidget, WidgetAction, event_in_bounds};
use crate::state::Focus;

impl MainWidget for MenuBar {
    fn id(&self) -> &'static str {
        "menu_bar"
    }

    fn draw(&mut self, screen: &mut Screen, state: &AppState, bounds: Rect) {
        MenuBar::draw(self, screen, state, bounds);
    }

    fn handle_event(&mut self, event: &crate::input::InputEvent, state: &mut AppState, bounds: Rect) -> WidgetAction {
        use crate::input::InputEvent;

        // Handle mouse clicks
        if let InputEvent::MouseClick { row, col } = event {
            // If menu is open, check for dropdown clicks first (dropdown overlays other widgets)
            if state.menu_open {
                match self.handle_dropdown_click(*row, *col, state) {
                    MenuClickResult::Execute(menu_idx, item_idx) => {
                        state.close_menu();
                        return WidgetAction::MenuAction(menu_idx, item_idx);
                    }
                    MenuClickResult::Absorbed => {
                        return WidgetAction::Consumed;
                    }
                    MenuClickResult::CloseMenu => {
                        state.close_menu();
                        // Fall through to check if click was on menu bar
                    }
                    _ => {}
                }
            }

            // Check for menu bar clicks
            if event_in_bounds(event, bounds) {
                match self.handle_bar_click(*row, *col, bounds, state) {
                    MenuClickResult::OpenMenu(i) => {
                        state.menu_index = i;
                        state.menu_item = 0;
                        state.open_menu();
                        return WidgetAction::Consumed;
                    }
                    MenuClickResult::CloseMenu => {
                        state.close_menu();
                        return WidgetAction::Consumed;
                    }
                    _ => return WidgetAction::Consumed,
                }
            }
            return WidgetAction::Ignored;
        }

        // Handle keyboard when menu is open
        if state.menu_open && state.focus == Focus::Menu {
            if let Some(action) = self.handle_input(state, event) {
                match action {
                    MenuAction::Execute(menu_idx, item_idx) => {
                        return WidgetAction::MenuAction(menu_idx, item_idx);
                    }
                    MenuAction::Close => {
                        return WidgetAction::Consumed;
                    }
                    MenuAction::Navigate => {
                        return WidgetAction::Consumed;
                    }
                }
            }
        }

        WidgetAction::Ignored
    }

    fn focusable(&self) -> bool {
        true
    }

    fn focus_type(&self) -> Option<Focus> {
        Some(Focus::Menu)
    }
}
