//! UI components for QBasic IDE

pub mod menubar;
pub mod editor;
pub mod statusbar;
pub mod immediate;
pub mod output;
pub mod layout;
pub mod scrollbar;
pub mod widget;
pub mod editor_widgets;
pub mod dialogs;
pub mod modal;
pub mod floating_window;
pub mod window_chrome;
pub mod theme;
pub mod widget_tree;
pub mod widgets;
pub mod widget_container;
pub mod main_widget;

pub use menubar::MenuBar;
pub use main_widget::WidgetAction;
pub use editor::Editor;
pub use statusbar::StatusBar;
pub use immediate::ImmediateWindow;
pub use output::OutputWindow;
pub use layout::{Rect, compute_layout};
pub use modal::{ModalDialog, ModalResult, ModalAction};
pub use widget_container::Widgets;
