mod app;
mod persistence;
mod stats;
mod tasks;
mod timer;
mod ui;

use std::env;
use std::io;
use std::time::Duration;

use crossterm::{
    event::{self, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{Terminal, backend::CrosstermBackend};

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.contains(&"--stats".to_string()) {
        stats::print_stats();
        return;
    }

    // Put the terminal into raw mode and switch to the alternate screen
    // (the alternate screen means the normal terminal contents are restored on exit).
    enable_raw_mode().expect("failed to enable raw mode");
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen).expect("failed to enter alternate screen");

    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend).expect("failed to create terminal");

    // Draw loop: redraw on every keypress or every 100 ms, exit on 'q'.
    loop {
        terminal.draw(|f| ui::render(f)).expect("failed to draw");

        // Poll for input with a short timeout so the loop stays responsive.
        if event::poll(Duration::from_millis(100)).expect("failed to poll events") {
            if let Event::Key(key) = event::read().expect("failed to read event") {
                if key.code == KeyCode::Char('q') {
                    break;
                }
            }
        }
    }

    // Restore the terminal before exiting so the user's shell is not broken.
    disable_raw_mode().expect("failed to disable raw mode");
    execute!(terminal.backend_mut(), LeaveAlternateScreen)
        .expect("failed to leave alternate screen");
}
