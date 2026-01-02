//! QBasic IDE Simulator
//!
//! A faithful simulation of the MS-DOS QBasic IDE written in Rust
//! using raw ANSI escape sequences (no external TUI libraries).

mod terminal;
mod screen;
mod input;
mod state;
mod ui;
mod basic;
mod help;
mod app;

use std::io;

fn main() -> io::Result<()> {
    let mut app = app::App::new()?;
    app.run()
}
