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

use clap::Parser;
use std::io;
use std::path::PathBuf;

/// QBasic IDE Simulator - A faithful recreation of the MS-DOS QBasic IDE
#[derive(Parser)]
#[command(name = "qbasic-rs")]
#[command(version, about, long_about = None)]
struct Args {
    /// BASIC file to load on startup
    file: Option<PathBuf>,
}

fn main() -> io::Result<()> {
    let args = Args::parse();

    let mut app = app::App::new()?;

    if let Some(path) = args.file {
        app.load_file_from_path(path);
    }

    app.run()
}
