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
    event::{self, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{Terminal, backend::CrosstermBackend};

use app::AppState;

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.contains(&"--stats".to_string()) {
        stats::print_stats();
        return;
    }

    enable_raw_mode().expect("failed to enable raw mode");
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen).expect("failed to enter alternate screen");

    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend).expect("failed to create terminal");

    let mut app = AppState::new();

    loop {
        // Pull in any timer ticks that arrived since the last iteration.
        app.process_timer_events();

        terminal.draw(|f| ui::render(f, &app)).expect("failed to draw");

        // Poll for input with a short timeout so the timer ticks drive redraws
        // even when the user is not pressing keys.
        if event::poll(Duration::from_millis(100)).expect("failed to poll events") {
            if let Event::Key(key) = event::read().expect("failed to read event") {
                // Guard against key-release events firing on some platforms.
                if key.kind != KeyEventKind::Press {
                    continue;
                }

                match key.code {
                    KeyCode::Char('q') => {
                        app.quit();
                        break;
                    }
                    KeyCode::Char(' ') => app.toggle_pause(),
                    KeyCode::Char('s') => app.skip(),
                    KeyCode::Char('u') => app.undo_last(),
                    _ => {}
                }
            }
        }
    }

    disable_raw_mode().expect("failed to disable raw mode");
    execute!(terminal.backend_mut(), LeaveAlternateScreen)
        .expect("failed to leave alternate screen");
}
