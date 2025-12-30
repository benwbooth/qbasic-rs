//! UI components for QBasic IDE

pub mod menubar;
pub mod editor;
pub mod statusbar;
pub mod immediate;
pub mod dialog;
pub mod layout;

pub use menubar::MenuBar;
pub use editor::Editor;
pub use statusbar::StatusBar;
pub use immediate::ImmediateWindow;
pub use dialog::Dialog;
pub use layout::{Rect, compute_layout, file_dialog_layout};
