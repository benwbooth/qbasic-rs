//! Input handling and key event processing

use crate::terminal::{Key, MouseEvent, MouseButton};

/// Processed input events for the application
#[derive(Clone, Debug)]
pub enum InputEvent {
    /// Mouse click
    MouseClick { row: u16, col: u16 },
    /// Mouse release
    MouseRelease { row: u16, col: u16 },
    /// Mouse drag (move while button held)
    MouseDrag { row: u16, col: u16 },
    /// Mouse wheel scroll
    ScrollUp { row: u16, col: u16 },
    ScrollDown { row: u16, col: u16 },
    /// Regular character input
    Char(char),
    /// Navigation keys
    CursorUp,
    CursorDown,
    CursorLeft,
    CursorRight,
    Home,
    End,
    PageUp,
    PageDown,
    /// Shift+Navigation for selection
    ShiftUp,
    ShiftDown,
    ShiftLeft,
    ShiftRight,
    ShiftHome,
    ShiftEnd,
    ShiftSpace,
    CtrlSpace,
    /// Ctrl+Navigation for selection
    CtrlUp,
    CtrlDown,
    CtrlLeft,
    CtrlRight,
    CtrlHome,
    CtrlEnd,
    CtrlPageUp,
    CtrlPageDown,
    /// Editing keys
    Enter,
    Backspace,
    Delete,
    Tab,
    ShiftTab,
    Insert,
    /// Escape key
    Escape,
    /// Function keys
    F1,  // Help
    F2,  // SUBs
    F3,  // Repeat last find
    F4,  // View output
    F5,  // Run
    F6,  // Next window
    F7,  // Step (debug)
    F8,  // Step over (debug)
    F9,  // Toggle breakpoint
    F10, // Menu
    /// Menu shortcuts
    AltF,  // File menu
    AltE,  // Edit menu
    AltV,  // View menu
    AltS,  // Search menu
    AltR,  // Run menu
    AltD,  // Debug menu
    AltO,  // Options menu
    AltH,  // Help menu
    AltX,  // Exit
    /// Ctrl shortcuts
    CtrlA, // Select all
    CtrlC, // Copy
    CtrlV, // Paste
    CtrlX, // Cut
    CtrlS, // Save
    CtrlO, // Open
    CtrlN, // New
    CtrlF, // Find
    CtrlG, // Go to line
    CtrlZ, // Undo
    CtrlY, // Redo
    CtrlQ, // Quit
    CtrlBackspace, // Delete word left
    CtrlDelete, // Delete word right
    /// Ctrl+Shift for selection
    CtrlShiftLeft,
    CtrlShiftRight,
    CtrlShiftHome,
    CtrlShiftEnd,
    CtrlShiftK, // Delete line
    /// Line operations
    CtrlD, // Duplicate line
    CtrlSlash, // Toggle comment
    AltUp, // Move line up
    AltDown, // Move line down
    /// Other
    Unknown,
    UnknownBytes(Vec<u8>),
    None,
}

impl From<Key> for InputEvent {
    fn from(key: Key) -> Self {
        match key {
            Key::Char(c) => InputEvent::Char(c),
            Key::Enter => InputEvent::Enter,
            Key::Escape => InputEvent::Escape,
            Key::Backspace => InputEvent::Backspace,
            Key::Delete => InputEvent::Delete,
            Key::Tab => InputEvent::Tab,
            Key::ShiftTab => InputEvent::ShiftTab,
            Key::Up => InputEvent::CursorUp,
            Key::Down => InputEvent::CursorDown,
            Key::Left => InputEvent::CursorLeft,
            Key::Right => InputEvent::CursorRight,
            Key::Home => InputEvent::Home,
            Key::End => InputEvent::End,
            Key::PageUp => InputEvent::PageUp,
            Key::PageDown => InputEvent::PageDown,
            Key::Insert => InputEvent::Insert,
            Key::ShiftUp => InputEvent::ShiftUp,
            Key::ShiftDown => InputEvent::ShiftDown,
            Key::ShiftLeft => InputEvent::ShiftLeft,
            Key::ShiftRight => InputEvent::ShiftRight,
            Key::ShiftHome => InputEvent::ShiftHome,
            Key::ShiftEnd => InputEvent::ShiftEnd,
            Key::ShiftSpace => InputEvent::ShiftSpace,
            Key::CtrlSpace => InputEvent::CtrlSpace,
            Key::CtrlUp => InputEvent::CtrlUp,
            Key::CtrlDown => InputEvent::CtrlDown,
            Key::CtrlLeft => InputEvent::CtrlLeft,
            Key::CtrlRight => InputEvent::CtrlRight,
            Key::CtrlHome => InputEvent::CtrlHome,
            Key::CtrlEnd => InputEvent::CtrlEnd,
            Key::CtrlPageUp => InputEvent::CtrlPageUp,
            Key::CtrlPageDown => InputEvent::CtrlPageDown,
            Key::CtrlBackspace => InputEvent::CtrlBackspace,
            Key::CtrlDelete => InputEvent::CtrlDelete,
            Key::CtrlShiftLeft => InputEvent::CtrlShiftLeft,
            Key::CtrlShiftRight => InputEvent::CtrlShiftRight,
            Key::CtrlShiftHome => InputEvent::CtrlShiftHome,
            Key::CtrlShiftEnd => InputEvent::CtrlShiftEnd,
            Key::CtrlShiftK => InputEvent::CtrlShiftK,
            Key::AltUp => InputEvent::AltUp,
            Key::AltDown => InputEvent::AltDown,
            Key::Ctrl('d') => InputEvent::CtrlD,
            Key::Ctrl('/') => InputEvent::CtrlSlash,
            Key::F(1) => InputEvent::F1,
            Key::F(2) => InputEvent::F2,
            Key::F(3) => InputEvent::F3,
            Key::F(4) => InputEvent::F4,
            Key::F(5) => InputEvent::F5,
            Key::F(6) => InputEvent::F6,
            Key::F(7) => InputEvent::F7,
            Key::F(8) => InputEvent::F8,
            Key::F(9) => InputEvent::F9,
            Key::F(10) => InputEvent::F10,
            Key::Alt('f') => InputEvent::AltF,
            Key::Alt('e') => InputEvent::AltE,
            Key::Alt('v') => InputEvent::AltV,
            Key::Alt('s') => InputEvent::AltS,
            Key::Alt('r') => InputEvent::AltR,
            Key::Alt('d') => InputEvent::AltD,
            Key::Alt('o') => InputEvent::AltO,
            Key::Alt('h') => InputEvent::AltH,
            Key::Alt('x') => InputEvent::AltX,
            Key::Ctrl('a') => InputEvent::CtrlA,
            Key::Ctrl('c') => InputEvent::CtrlC,
            Key::Ctrl('v') => InputEvent::CtrlV,
            Key::Ctrl('x') => InputEvent::CtrlX,
            Key::Ctrl('s') => InputEvent::CtrlS,
            Key::Ctrl('o') => InputEvent::CtrlO,
            Key::Ctrl('n') => InputEvent::CtrlN,
            Key::Ctrl('f') => InputEvent::CtrlF,
            Key::Ctrl('g') => InputEvent::CtrlG,
            Key::Ctrl('z') => InputEvent::CtrlZ,
            Key::Ctrl('y') => InputEvent::CtrlY,
            Key::Ctrl('q') => InputEvent::CtrlQ,
            Key::Mouse(MouseEvent { button: MouseButton::Left, row, col, pressed: true, motion: false }) => {
                InputEvent::MouseClick { row, col }
            }
            Key::Mouse(MouseEvent { button: MouseButton::Left, row, col, pressed: false, .. }) => {
                InputEvent::MouseRelease { row, col }
            }
            Key::Mouse(MouseEvent { button: MouseButton::Left, row, col, motion: true, .. }) => {
                InputEvent::MouseDrag { row, col }
            }
            Key::Mouse(MouseEvent { button: MouseButton::WheelUp, row, col, .. }) => {
                InputEvent::ScrollUp { row, col }
            }
            Key::Mouse(MouseEvent { button: MouseButton::WheelDown, row, col, .. }) => {
                InputEvent::ScrollDown { row, col }
            }
            Key::Mouse(_) => InputEvent::Unknown, // Ignore other mouse events
            Key::Unknown(bytes) => InputEvent::UnknownBytes(bytes),
            _ => InputEvent::Unknown,
        }
    }
}

/// Check if a key combination requires menu focus
pub fn is_menu_trigger(event: &InputEvent) -> bool {
    matches!(
        event,
        InputEvent::F10
            | InputEvent::AltF
            | InputEvent::AltE
            | InputEvent::AltV
            | InputEvent::AltS
            | InputEvent::AltR
            | InputEvent::AltD
            | InputEvent::AltO
            | InputEvent::AltH
    )
}

/// Get menu index from Alt key
pub fn menu_index_from_alt(event: &InputEvent) -> Option<usize> {
    match event {
        InputEvent::AltF => Some(0), // File
        InputEvent::AltE => Some(1), // Edit
        InputEvent::AltV => Some(2), // View
        InputEvent::AltS => Some(3), // Search
        InputEvent::AltR => Some(4), // Run
        InputEvent::AltD => Some(5), // Debug
        InputEvent::AltO => Some(6), // Options
        InputEvent::AltH => Some(7), // Help
        _ => None,
    }
}
