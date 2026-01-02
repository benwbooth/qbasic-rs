//! Dialog components
//!
//! Each dialog is a self-contained struct that owns its state and handles
//! its own drawing and event handling.

mod file_dialog;

pub use file_dialog::FileDialog;
