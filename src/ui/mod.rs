//! UI components for QBasic IDE

pub mod menubar;
pub mod editor;
pub mod statusbar;
pub mod immediate;
pub mod output;
pub mod dialog;
pub mod layout;
pub mod scrollbar;
pub mod widget;
pub mod listview;
pub mod textfield;
pub mod button;
pub mod dialog_widgets;
pub mod dialog_manager;
pub mod editor_widgets;
pub mod dialogs;
pub mod modal;
pub mod floating_window;

pub use menubar::{MenuBar, MenuClickResult};
pub use editor::Editor;
pub use statusbar::StatusBar;
pub use immediate::{ImmediateWindow, ImmediateClickResult};
pub use output::{OutputWindow, OutputClickResult};
pub use dialog::Dialog;
pub use layout::{Rect, compute_layout};
pub use editor::{tokenize_line, TokenKind};
pub use dialogs::FileDialog;
pub use modal::{ModalDialog, ModalResult, ModalAction};
