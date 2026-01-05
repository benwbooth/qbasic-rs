//! File Open dialog.

use std::path::PathBuf;

use crate::input::InputEvent;
use crate::screen::Screen;
use crate::state::AppState;
use crate::ui::theme::Theme;
use crate::ui::widget::EventResult;
use crate::ui::widget_tree::WidgetNode;
use crate::ui::widgets::{Button, Label, ListView, Spacer, TextField};

use super::{DialogContext, DialogController, DialogResult, DialogWidget};

pub struct FileOpenDialog {
    dialog: DialogWidget,
    current_path: PathBuf,
    open: bool,
}

impl FileOpenDialog {
    pub fn new() -> Self {
        let content = Self::build_content();
        let mut dialog = DialogWidget::with_theme("Open Program", content, Theme::qbasic_dialog())
            .with_size(60, 18)
            .with_min_size(40, 12);
        dialog.set_show_maximize(true);

        Self {
            dialog,
            current_path: std::env::current_dir().unwrap_or_default(),
            open: false,
        }
    }

    fn build_content() -> WidgetNode {
        WidgetNode::vstack("root")
            .padding(1)
            .child(
                WidgetNode::hstack("filename_row")
                    .leaf("filename_label", Label::new("File Name:").min_width(12))
                    .leaf("filename_field", TextField::new("filename"))
                    .spacing(0)
                    .build(),
            )
            .child(WidgetNode::leaf("spacer1", Spacer::fixed(1)))
            .child(
                WidgetNode::hstack("directory_row")
                    .leaf("directory_label", Label::new("Directory:").min_width(12))
                    .leaf("directory_display", Label::new(""))
                    .spacing(0)
                    .build(),
            )
            .child(WidgetNode::leaf("spacer2", Spacer::fixed(1)))
            .child(
                WidgetNode::hstack("labels_row")
                    .leaf("files_label", Label::new("Files:"))
                    .leaf("dirs_label", Label::new("Dirs/Drives:"))
                    .spacing(2)
                    .build(),
            )
            .child(
                WidgetNode::hstack("lists_row")
                    .leaf("files_list", ListView::new("files").with_border(true))
                    .leaf("dirs_list", ListView::new("dirs").with_border(true))
                    .spacing(2)
                    .build(),
            )
            .child(WidgetNode::leaf("spacer3", Spacer::fixed(1)))
            .child(
                WidgetNode::hstack("buttons_row")
                    .child(WidgetNode::leaf("btn_spacer_left", Spacer::new()))
                    .leaf("ok_button", Button::new("OK", "ok").min_width(8))
                    .leaf("cancel_button", Button::new("Cancel", "cancel").min_width(10))
                    .child(WidgetNode::leaf("btn_spacer_right", Spacer::new()))
                    .spacing(2)
                    .build(),
            )
            .build()
    }

    fn refresh_lists(&mut self) {
        let mut files = Vec::new();
        let mut dirs = vec!["..".to_string()];

        if let Ok(entries) = std::fs::read_dir(&self.current_path) {
            for entry in entries.filter_map(|e| e.ok()) {
                if let Ok(name) = entry.file_name().into_string() {
                    if entry.path().is_dir() {
                        dirs.push(name);
                    } else if name.to_lowercase().ends_with(".bas") {
                        files.push(name);
                    }
                }
            }
        }

        files.sort();
        dirs.sort();

        if let Some(list) = self.dialog.content_mut()
            .get_widget_mut(&["root", "lists_row", "files_list"])
            .and_then(|w| w.as_any_mut().downcast_mut::<ListView>())
        {
            list.set_items(files);
        }
        if let Some(list) = self.dialog.content_mut()
            .get_widget_mut(&["root", "lists_row", "dirs_list"])
            .and_then(|w| w.as_any_mut().downcast_mut::<ListView>())
        {
            list.set_items(dirs);
        }

        self.sync_directory_display();
    }

    fn sync_directory_display(&mut self) {
        let path_str = self.current_path.to_string_lossy().to_string();
        if let Some(label) = self.dialog.content_mut()
            .get_widget_mut(&["root", "directory_row", "directory_display"])
            .and_then(|w| w.as_any_mut().downcast_mut::<Label>())
        {
            label.set_text(path_str);
        }
    }

    fn navigate_to(&mut self, dir_name: &str) {
        let new_path = if dir_name == ".." {
            self.current_path.parent()
                .map(|p| p.to_path_buf())
                .unwrap_or_else(|| self.current_path.clone())
        } else {
            self.current_path.join(dir_name)
        };

        if new_path.is_dir() {
            self.current_path = new_path;
            self.refresh_lists();
        }
    }

    fn get_filename(&self) -> String {
        self.dialog.content()
            .get_widget(&["root", "filename_row", "filename_field"])
            .and_then(|w| w.as_any().downcast_ref::<TextField>())
            .map(|tf| tf.text().to_string())
            .unwrap_or_default()
    }

    fn set_filename(&mut self, name: &str) {
        if let Some(tf) = self.dialog.content_mut()
            .get_widget_mut(&["root", "filename_row", "filename_field"])
            .and_then(|w| w.as_any_mut().downcast_mut::<TextField>())
        {
            tf.set_text(name);
            tf.set_cursor_pos(name.chars().count());
        }
    }

    fn get_selected_file(&self) -> Option<String> {
        self.dialog.content()
            .get_widget(&["root", "lists_row", "files_list"])
            .and_then(|w| w.as_any().downcast_ref::<ListView>())
            .and_then(|list| list.selected_item().map(|s| s.to_string()))
    }

    fn get_selected_dir(&self) -> Option<String> {
        self.dialog.content()
            .get_widget(&["root", "lists_row", "dirs_list"])
            .and_then(|w| w.as_any().downcast_ref::<ListView>())
            .and_then(|list| list.selected_item().map(|s| s.to_string()))
    }

    fn sync_focus_decor(&mut self) {
        let focus_path = self.dialog.content().focus_path();
        let filename_focused = focus_path.iter().any(|id| id == "filename_field");
        let files_focused = focus_path.iter().any(|id| id == "files_list");
        let dirs_focused = focus_path.iter().any(|id| id == "dirs_list");

        if let Some(label) = self.dialog.content_mut()
            .get_widget_mut(&["root", "filename_row", "filename_label"])
            .and_then(|w| w.as_any_mut().downcast_mut::<Label>())
        {
            label.set_highlight(filename_focused);
        }
        if let Some(label) = self.dialog.content_mut()
            .get_widget_mut(&["root", "labels_row", "files_label"])
            .and_then(|w| w.as_any_mut().downcast_mut::<Label>())
        {
            label.set_highlight(files_focused);
        }
        if let Some(label) = self.dialog.content_mut()
            .get_widget_mut(&["root", "labels_row", "dirs_label"])
            .and_then(|w| w.as_any_mut().downcast_mut::<Label>())
        {
            label.set_highlight(dirs_focused);
        }
    }
}

impl FileOpenDialog {
    fn load_file(&self, ctx: &mut DialogContext, path: PathBuf) {
        match std::fs::read_to_string(&path) {
            Ok(content) => {
                ctx.editor.load(&content);
                ctx.state.file_path = Some(path);
                ctx.state.set_modified(false);
                ctx.state.set_status("File loaded");
            }
            Err(e) => {
                ctx.state.set_status(format!("Error loading file: {}", e));
            }
        }
    }
}

impl DialogController for FileOpenDialog {

    fn open(&mut self, ctx: &mut DialogContext) {
        self.open = true;
        if let Some(path) = &ctx.state.file_path {
            if let Some(parent) = path.parent() {
                self.current_path = parent.to_path_buf();
            }
        }
        self.refresh_lists();
        self.set_filename("");
        self.dialog.focus_first();
        self.dialog.center();
        ctx.state.focus_dialog();
    }

    fn is_open(&self) -> bool {
        self.open
    }

    fn close(&mut self) {
        self.open = false;
    }

    fn set_screen_size(&mut self, width: u16, height: u16) {
        self.dialog.set_screen_size(width, height);
    }

    fn draw(&mut self, screen: &mut Screen, _state: &AppState) {
        if !self.open {
            return;
        }
        self.sync_focus_decor();
        self.dialog.center();
        self.dialog.draw_with_theme(screen);
    }

    fn handle_event(&mut self, event: &InputEvent, ctx: &mut DialogContext) -> DialogResult {
        if !self.open {
            return DialogResult::Open;
        }

        let result = self.dialog.handle_event(event);
        self.sync_focus_decor();

        if let EventResult::Action(action) = result {
            match action.as_str() {
                "files_select" => {
                    if let Some(name) = self.get_selected_file() {
                        self.set_filename(&name);
                    }
                }
                "files_activate" => {
                    if let Some(name) = self.get_selected_file() {
                        self.set_filename(&name);
                        let path = self.current_path.join(&name);
                        self.load_file(ctx, path);
                        return DialogResult::Closed;
                    }
                }
                "dirs_activate" => {
                    if let Some(name) = self.get_selected_dir() {
                        self.navigate_to(&name);
                    }
                }
                "ok" | "filename_submit" => {
                    let filename = self.get_filename();
                    if !filename.is_empty() {
                        let path = self.current_path.join(&filename);
                        self.load_file(ctx, path);
                        return DialogResult::Closed;
                    }
                }
                "cancel" | "dialog_cancel" => return DialogResult::Closed,
                _ => {}
            }
        }

        DialogResult::Open
    }
}
