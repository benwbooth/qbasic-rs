//! Dialog components
//!
//! Each dialog implements the DialogController trait. The Dialogs struct
//! holds all dialog instances with typed access.

mod about;
mod confirm;
mod dialog_context;
mod dialog_controller;
mod dialog_registry;
mod dialog_widget;
mod display_options;
mod file_open;
mod file_save;
mod find;
mod goto;
mod help;
mod message;
mod new_program;
mod print;
mod replace;
mod simple_input;
mod welcome;

pub(super) use dialog_widget::DialogWidget;
pub use dialog_context::{DialogContext, DialogResult};
pub use dialog_controller::DialogController;
pub use dialog_registry::Dialogs;

// Re-export dialog types for direct access
pub use about::AboutDialog;
pub use confirm::ConfirmDialog;
pub use display_options::DisplayOptionsDialog;
pub use file_open::FileOpenDialog;
pub use file_save::FileSaveDialog;
pub use find::FindDialog;
pub use goto::GoToDialog;
pub use help::HelpDialog;
pub use message::MessageDialog;
pub use new_program::NewProgramDialog;
pub use print::PrintDialog;
pub use replace::ReplaceDialog;
pub use simple_input::{NewSubDialog, NewFunctionDialog, FindLabelDialog, CommandArgsDialog, HelpPathDialog};
pub use welcome::WelcomeDialog;
