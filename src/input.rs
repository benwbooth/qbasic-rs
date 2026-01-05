//! Input handling and key event processing

use crate::terminal::{Key, MouseEvent, MouseButton};

/// Processed input events for the application
#[derive(Clone, Debug, PartialEq)]
#[allow(dead_code)]
pub enum InputEvent {
    /// Mouse click
    MouseClick { row: u16, col: u16 },
    /// Mouse release
    MouseRelease { row: u16, col: u16 },
    /// Mouse drag (move while button held)
    MouseDrag { row: u16, col: u16 },
    /// Mouse move (no button pressed)
    MouseMove { row: u16, col: u16 },
    /// Mouse wheel scroll
    ScrollUp { row: u16, col: u16 },
    ScrollDown { row: u16, col: u16 },
    ScrollLeft { row: u16, col: u16 },
    ScrollRight { row: u16, col: u16 },
    /// Regular character input
    Char(char),
    /// Alt + character
    Alt(char),
    /// Ctrl + character
    Ctrl(char),
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
    /// Ctrl+Navigation
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
    /// Ctrl+Shift combinations
    CtrlShiftLeft,
    CtrlShiftRight,
    CtrlShiftHome,
    CtrlShiftEnd,
    CtrlShiftK,
    /// Alt+Navigation
    AltUp,
    AltDown,
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
    F(u8),
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
            Key::F(n) => InputEvent::F(n),
            Key::Alt(c) => InputEvent::Alt(c),
            Key::Ctrl(c) => InputEvent::Ctrl(c),
            Key::Mouse(MouseEvent { button: MouseButton::Left, row, col, pressed: true, motion: false, .. }) => {
                InputEvent::MouseClick { row, col }
            }
            Key::Mouse(MouseEvent { button: MouseButton::Left, row, col, pressed: false, .. }) => {
                InputEvent::MouseRelease { row, col }
            }
            Key::Mouse(MouseEvent { button: MouseButton::Left, row, col, motion: true, .. }) => {
                InputEvent::MouseDrag { row, col }
            }
            Key::Mouse(MouseEvent { button: MouseButton::WheelUp, row, col, shift: true, .. }) => {
                InputEvent::ScrollLeft { row, col }
            }
            Key::Mouse(MouseEvent { button: MouseButton::WheelDown, row, col, shift: true, .. }) => {
                InputEvent::ScrollRight { row, col }
            }
            Key::Mouse(MouseEvent { button: MouseButton::WheelUp, row, col, .. }) => {
                InputEvent::ScrollUp { row, col }
            }
            Key::Mouse(MouseEvent { button: MouseButton::WheelDown, row, col, .. }) => {
                InputEvent::ScrollDown { row, col }
            }
            Key::Mouse(MouseEvent { button: MouseButton::WheelLeft, row, col, .. }) => {
                InputEvent::ScrollLeft { row, col }
            }
            Key::Mouse(MouseEvent { button: MouseButton::WheelRight, row, col, .. }) => {
                InputEvent::ScrollRight { row, col }
            }
            Key::Mouse(MouseEvent { button: MouseButton::None, row, col, motion: true, .. }) => {
                InputEvent::MouseMove { row, col }
            }
            Key::Mouse(_) => InputEvent::Unknown,
            Key::Unknown(bytes) => InputEvent::UnknownBytes(bytes),
        }
    }
}

/// Check if a key combination requires menu focus
pub fn is_menu_trigger(event: &InputEvent) -> bool {
    matches!(
        event,
        InputEvent::F(10)
            | InputEvent::Alt('f')
            | InputEvent::Alt('e')
            | InputEvent::Alt('v')
            | InputEvent::Alt('s')
            | InputEvent::Alt('r')
            | InputEvent::Alt('d')
            | InputEvent::Alt('o')
            | InputEvent::Alt('h')
    )
}

/// Get menu index from Alt key
pub fn menu_index_from_alt(event: &InputEvent) -> Option<usize> {
    match event {
        InputEvent::Alt('f') => Some(0), // File
        InputEvent::Alt('e') => Some(1), // Edit
        InputEvent::Alt('v') => Some(2), // View
        InputEvent::Alt('s') => Some(3), // Search
        InputEvent::Alt('r') => Some(4), // Run
        InputEvent::Alt('d') => Some(5), // Debug
        InputEvent::Alt('o') => Some(6), // Options
        InputEvent::Alt('h') => Some(7), // Help
        _ => None,
    }
}
