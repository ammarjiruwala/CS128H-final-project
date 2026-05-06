use chrono::{Datelike, Duration, Local, NaiveDate, NaiveDateTime};
use crossterm::style::{Color, Print, ResetColor, SetForegroundColor};
use crossterm::ExecutableCommand;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::io::{stdout, Write};

/// One completed or skipped session recorded to disk.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionRecord {
    /// "Focus", "ShortBreak", or "LongBreak"
    pub session_type: String,
    /// true = ran to zero naturally; false = user pressed skip
    pub completed: bool,
    pub timestamp: String,
    /// Actual seconds spent in this session (partial for skipped Focus sessions).
    #[serde(default)]
    pub focus_secs: u64,
}

impl SessionRecord {
    pub fn new(session_type: &str, completed: bool, focus_secs: u64) -> Self {
        SessionRecord {
            session_type: session_type.to_string(),
            completed,
            timestamp: Local::now().format("%Y-%m-%d %H:%M:%S").to_string(),
            focus_secs,
        }
    }
}

// ---------------------------------------------------------------------------
// All-time summary
// ---------------------------------------------------------------------------

pub struct AllTimeStats {
    pub total_completed: u32,
    pub total_skipped: u32,
    pub total_focus_secs: u64,
    pub best_streak: u32,
}

pub fn compute_all_time_stats(history: &[SessionRecord]) -> AllTimeStats {
    let focus: Vec<&SessionRecord> = history.iter()
        .filter(|r| r.session_type == "Focus")
        .collect();

    let total_completed  = focus.iter().filter(|r| r.completed).count() as u32;
    let total_skipped    = focus.iter().filter(|r| !r.completed).count() as u32;
    let total_focus_secs = focus.iter().map(|r| r.focus_secs).sum();

    let mut best_streak = 0u32;
    let mut cur = 0u32;
    for r in &focus {
        if r.completed { cur += 1; best_streak = best_streak.max(cur); }
        else           { cur = 0; }
    }

    AllTimeStats { total_completed, total_skipped, total_focus_secs, best_streak }
}

// ---------------------------------------------------------------------------
// Heatmap data
// ---------------------------------------------------------------------------

/// Returns a map of NaiveDate → total focus seconds on that day.
pub fn daily_focus_map(history: &[SessionRecord]) -> HashMap<NaiveDate, u64> {
    let mut map: HashMap<NaiveDate, u64> = HashMap::new();
    for r in history {
        if r.session_type == "Focus" && r.focus_secs > 0 {
            if let Ok(dt) = NaiveDateTime::parse_from_str(&r.timestamp, "%Y-%m-%d %H:%M:%S") {
                *map.entry(dt.date()).or_insert(0) += r.focus_secs;
            }
        }
    }
    map
}

/// Returns a 7×weeks grid of focus seconds (row = Mon–Sun, col = oldest→newest week).
pub fn build_heatmap(history: &[SessionRecord], weeks: usize) -> Vec<Vec<u64>> {
    let focus_map = daily_focus_map(history);
    let today     = Local::now().date_naive();

    // Anchor to Monday of the current week.
    let days_since_monday = today.weekday().num_days_from_monday() as i64;
    let this_monday       = today - Duration::days(days_since_monday);
    let start_monday      = this_monday - Duration::weeks((weeks as i64) - 1);

    (0..7)
        .map(|day| {
            (0..weeks)
                .map(|week| {
                    let date = start_monday + Duration::days((week * 7 + day) as i64);
                    if date > today { 0 }
                    else { *focus_map.get(&date).unwrap_or(&0) }
                })
                .collect()
        })
        .collect()
}

// ---------------------------------------------------------------------------
// --stats CLI output (with coloured heatmap)
// ---------------------------------------------------------------------------

fn heatmap_color(focus_secs: u64) -> Color {
    match focus_secs {
        0           => Color::Rgb { r: 30,  g: 35,  b: 40  }, // empty
        1..=899     => Color::Rgb { r: 14,  g: 68,  b: 41  }, // < 15 min
        900..=1799  => Color::Rgb { r: 0,   g: 109, b: 50  }, // 15–30 min
        1800..=2999 => Color::Rgb { r: 38,  g: 166, b: 65  }, // 30–50 min
        _           => Color::Rgb { r: 57,  g: 211, b: 83  }, // 50 min+
    }
}

pub fn print_stats() {
    let history = crate::persistence::load_history().unwrap_or_default();
    let mut out  = stdout();

    println!();

    if history.is_empty() {
        println!("  No session history yet. Run the app to start tracking!");
        println!();
        return;
    }

    let s      = compute_all_time_stats(&history);
    let total  = s.total_completed + s.total_skipped;
    let rate   = if total > 0 { s.total_completed * 100 / total } else { 0 };
    let at_mins = s.total_focus_secs / 60;
    let at_time = if at_mins >= 60 {
        format!("{}h {}m", at_mins / 60, at_mins % 60)
    } else {
        format!("{}m", at_mins)
    };

    // ── All-time summary ──────────────────────────────────────────────────────
    println!("  Terminal Pomodoro — All-Time Stats");
    println!("  {}", "─".repeat(38));
    println!("  Completed sessions : {}", s.total_completed);
    println!("  Skipped sessions   : {}", s.total_skipped);
    println!("  Completion rate    : {}%", rate);
    println!("  Total focus time   : {}", at_time);
    println!("  Best streak        : {} sessions", s.best_streak);
    println!();

    // ── Activity heatmap (14 weeks) ───────────────────────────────────────────
    println!("  Activity — last 14 weeks");
    println!();

    let heatmap    = build_heatmap(&history, 14);
    let day_labels = ["Mo", "Tu", "We", "Th", "Fr", "Sa", "Su"];

    for (day_idx, row) in heatmap.iter().enumerate() {
        print!("  {} ", day_labels[day_idx]);
        for &secs in row {
            let _ = out.execute(SetForegroundColor(heatmap_color(secs)));
            let _ = out.execute(Print("▪ "));
        }
        let _ = out.execute(ResetColor);
        println!();
    }

    // Legend
    println!();
    print!("  Less ");
    for &secs in &[0u64, 500, 1200, 2000, 3200] {
        let _ = out.execute(SetForegroundColor(heatmap_color(secs)));
        let _ = out.execute(Print("▪ "));
    }
    let _ = out.execute(ResetColor);
    println!("More");
    println!();

    // ── Recent sessions ───────────────────────────────────────────────────────
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
            let note = if r.session_type == "Focus" && !r.completed && r.focus_secs > 0 {
                format!("  ({}m elapsed)", r.focus_secs / 60)
            } else {
                String::new()
            };
            println!("    [{}] {}  {}{}", icon, label, r.timestamp, note);
        }
        println!();
    }

    let _ = out.flush();
}
