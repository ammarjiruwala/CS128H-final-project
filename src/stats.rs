use chrono::Local;
use serde::{Deserialize, Serialize};

/// One completed or skipped session recorded to disk.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionRecord {
    /// "Focus", "ShortBreak", or "LongBreak"
    pub session_type: String,
    /// true = ran to zero naturally; false = user pressed skip
    pub completed: bool,
    pub timestamp: String,
}

impl SessionRecord {
    pub fn new(session_type: &str, completed: bool) -> Self {
        SessionRecord {
            session_type: session_type.to_string(),
            completed,
            timestamp: Local::now().format("%Y-%m-%d %H:%M:%S").to_string(),
        }
    }
}

/// Print a human-readable stats summary to stdout.
/// Reads history from disk — used by the `--stats` CLI flag.
pub fn print_stats() {
    let history = crate::persistence::load_history().unwrap_or_default();

    if history.is_empty() {
        println!();
        println!("  No session history yet. Run the app to start tracking!");
        println!();
        return;
    }

    let focus: Vec<&SessionRecord> = history.iter()
        .filter(|r| r.session_type == "Focus")
        .collect();

    let completed  = focus.iter().filter(|r| r.completed).count() as u32;
    let skipped    = focus.iter().filter(|r| !r.completed).count() as u32;
    let total      = completed + skipped;
    let rate       = if total > 0 { completed * 100 / total } else { 0 };
    let focus_mins = completed * 25;

    // Longest streak of consecutive completed focus sessions.
    let mut longest: u32 = 0;
    let mut cur: u32 = 0;
    for r in &focus {
        if r.completed { cur += 1; longest = longest.max(cur); }
        else           { cur = 0; }
    }

    println!();
    println!("  Terminal Pomodoro — Session History");
    println!("  {}", "─".repeat(36));
    println!("  Completed sessions : {}", completed);
    println!("  Skipped sessions   : {}", skipped);
    println!("  Completion rate    : {}%", rate);
    println!("  Total focus time   : {}m{}", focus_mins,
        if focus_mins >= 60 { format!("  ({:.1}h)", focus_mins as f32 / 60.0) }
        else { String::new() }
    );
    println!("  Longest streak     : {} sessions", longest);
    println!();

    let recent: Vec<&SessionRecord> = history.iter().rev().take(10).collect();
    if !recent.is_empty() {
        println!("  Recent sessions (latest first):");
        for r in recent {
            let icon  = if r.completed { "✓" } else { "✗" };
            let label = match r.session_type.as_str() {
                "Focus"      => "Focus      ",
                "ShortBreak" => "Short Break",
                "LongBreak"  => "Long Break ",
                other        => other,
            };
            let note = if !r.completed { "  ← skipped" } else { "" };
            println!("    [{}] {}  {}{}", icon, label, r.timestamp, note);
        }
        println!();
    }
}
