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
pub mod event_router;
pub mod listview;
pub mod textfield;
pub mod button;
pub mod file_dialog_widgets;
pub mod dialog_widgets;
pub mod dialog_manager;
pub mod editor_widgets;

pub use menubar::{MenuBar, MenuClickResult};
pub use editor::Editor;
pub use statusbar::StatusBar;
pub use immediate::{ImmediateWindow, ImmediateClickResult};
pub use output::{OutputWindow, OutputClickResult};
pub use dialog::Dialog;
pub use layout::{Rect, compute_layout, file_dialog_layout};
pub use editor::{tokenize_line, TokenKind};

// Widget system exports
pub use widget::{Widget, EventResult};
pub use event_router::{EventRouter, WidgetContainer};
pub use scrollbar::{VerticalScrollbar, HorizontalScrollbar, ScrollbarColors, ScrollbarState};
pub use listview::{ListView, ListViewColors};
pub use textfield::{TextField, TextFieldColors};
pub use button::{Button, ButtonColors};
pub use file_dialog_widgets::{handle_file_list_click, FileListAction};
pub use dialog_widgets::{
    Checkbox, RadioButton, Label, DialogWidget, DialogField, CompositeDialog,
    create_find_dialog, create_replace_dialog, create_goto_dialog,
    create_simple_input_dialog, create_confirm_dialog, create_message_dialog,
};
pub use dialog_manager::{DialogManager, DialogAction};
pub use editor_widgets::{handle_editor_click, handle_vscroll_drag, handle_hscroll_drag, EditorClickAction};
