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
use persistence::{load_tasks, load_history};

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

    // Load persisted state from previous runs.
    if let Ok(tasks) = load_tasks() {
        if !tasks.is_empty() {
            app.tasks = crate::tasks::TaskQueue::from_tasks(tasks);
        }
    }
    if let Ok(history) = load_history() {
        app.session_history = history;
    }

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
                    KeyCode::Esc => app.cancel_input(),
                    KeyCode::Up => {
                        if !app.is_typing() { app.select_prev(); }
                    }
                    KeyCode::Down => {
                        if !app.is_typing() { app.select_next(); }
                    }
                    KeyCode::Enter => {
                        if app.is_adding() {
                            app.confirm_add_task();
                        } else if app.is_editing() {
                            app.confirm_edit_task();
                        } else {
                            app.toggle_selected();
                        }
                    }
                    KeyCode::Backspace => {
                        if app.is_typing() { app.input_pop(); }
                    }
                    KeyCode::Char(c) => {
                        if app.is_typing() {
                            app.input_push(c);
                        } else {
                            match c {
                                'q' => { app.quit(); break; }
                                ' ' => app.toggle_pause(),
                                's' => app.skip(),
                                'u' => app.undo_last(),
                                'a' => app.start_add_task(),
                                'e' => app.start_edit_task(),
                                'd' => app.delete_selected(),
                                '[' => app.scroll_stats_up(),
                                ']' => app.scroll_stats_down(),
                                _ => {}
                            }
                        }
                    }
                    _ => {}
                }
            }
        }
    }

    disable_raw_mode().expect("failed to disable raw mode");
    execute!(terminal.backend_mut(), LeaveAlternateScreen)
        .expect("failed to leave alternate screen");

    if let Err(e) = app.save() {
        eprintln!("Warning: could not save data: {}", e);
    }
}
