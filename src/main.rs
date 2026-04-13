mod app;
mod persistence;
mod stats;
mod tasks;
mod timer;
mod ui;

use std::env;

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.contains(&"--stats".to_string()) {
        stats::print_stats();
        return;
    }

    println!("Terminal Pomodoro — UI coming in Checkpoint 2.");
    println!("Run `cargo test` to verify the timer state machine.");
}
