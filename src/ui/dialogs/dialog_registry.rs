//! Dialogs container - holds all dialog instances with typed access.

use crate::input::InputEvent;
use crate::screen::Screen;
use crate::state::AppState;

use super::{
    AboutDialog, CommandArgsDialog, ConfirmDialog, DialogContext, DialogController,
    DialogResult, DisplayOptionsDialog, FileOpenDialog, FileSaveDialog, FindDialog,
    FindLabelDialog, GoToDialog, HelpDialog, HelpPathDialog, MessageDialog,
    NewFunctionDialog, NewProgramDialog, NewSubDialog, PrintDialog, ReplaceDialog,
    WelcomeDialog,
};

/// Container holding all dialog instances.
///
/// Each dialog is stored as its concrete type for direct access.
/// Use `all_mut()` to iterate over all dialogs as trait objects.
pub struct Dialogs {
    screen_size: (u16, u16),
    pub welcome: WelcomeDialog,
    pub help: HelpDialog,
    pub about: AboutDialog,
    pub find: FindDialog,
    pub replace: ReplaceDialog,
    pub goto: GoToDialog,
    pub file_open: FileOpenDialog,
    pub file_save: FileSaveDialog,
    pub display_options: DisplayOptionsDialog,
    pub message: MessageDialog,
    pub confirm: ConfirmDialog,
    pub new_program: NewProgramDialog,
    pub print: PrintDialog,
    pub new_sub: NewSubDialog,
    pub new_function: NewFunctionDialog,
    pub find_label: FindLabelDialog,
    pub command_args: CommandArgsDialog,
    pub help_path: HelpPathDialog,
}

impl Dialogs {
    pub fn new(width: u16, height: u16) -> Self {
        Self {
            screen_size: (width, height),
            welcome: WelcomeDialog::new(),
            help: HelpDialog::new(),
            about: AboutDialog::new(),
            find: FindDialog::new(),
            replace: ReplaceDialog::new(),
            goto: GoToDialog::new(),
            file_open: FileOpenDialog::new(),
            file_save: FileSaveDialog::new(),
            display_options: DisplayOptionsDialog::new(),
            message: MessageDialog::new(),
            confirm: ConfirmDialog::new(),
            new_program: NewProgramDialog::new(),
            print: PrintDialog::new(),
            new_sub: NewSubDialog::new(),
            new_function: NewFunctionDialog::new(),
            find_label: FindLabelDialog::new(),
            command_args: CommandArgsDialog::new(),
            help_path: HelpPathDialog::new(),
        }
    }

    /// Iterate over all dialogs as trait objects
    fn all_mut(&mut self) -> impl Iterator<Item = &mut dyn DialogController> {
        [
            &mut self.welcome as &mut dyn DialogController,
            &mut self.help,
            &mut self.about,
            &mut self.find,
            &mut self.replace,
            &mut self.goto,
            &mut self.file_open,
            &mut self.file_save,
            &mut self.display_options,
            &mut self.message,
            &mut self.confirm,
            &mut self.new_program,
            &mut self.print,
            &mut self.new_sub,
            &mut self.new_function,
            &mut self.find_label,
            &mut self.command_args,
            &mut self.help_path,
        ]
        .into_iter()
    }

    /// Check if any dialog is currently open
    pub fn is_active(&self) -> bool {
        self.welcome.is_open()
            || self.help.is_open()
            || self.about.is_open()
            || self.find.is_open()
            || self.replace.is_open()
            || self.goto.is_open()
            || self.file_open.is_open()
            || self.file_save.is_open()
            || self.display_options.is_open()
            || self.message.is_open()
            || self.confirm.is_open()
            || self.new_program.is_open()
            || self.print.is_open()
            || self.new_sub.is_open()
            || self.new_function.is_open()
            || self.find_label.is_open()
            || self.command_args.is_open()
            || self.help_path.is_open()
    }

    /// Close any open dialog
    pub fn close_active(&mut self) {
        for d in self.all_mut() {
            if d.is_open() {
                d.close();
                return;
            }
        }
    }

    /// Update screen size for all dialogs
    pub fn set_screen_size(&mut self, width: u16, height: u16) {
        self.screen_size = (width, height);
        for d in self.all_mut() {
            d.set_screen_size(width, height);
        }
    }

    /// Draw the active dialog (if any)
    pub fn draw(&mut self, screen: &mut Screen, state: &AppState) -> bool {
        for d in self.all_mut() {
            if d.is_open() {
                d.draw(screen, state);
                return true;
            }
        }
        false
    }

    /// Handle an event for the active dialog
    pub fn handle_event(&mut self, event: &InputEvent, ctx: &mut DialogContext) -> DialogResult {
        for d in self.all_mut() {
            if d.is_open() {
                return d.handle_event(event, ctx);
            }
        }
        DialogResult::Open
    }
}
