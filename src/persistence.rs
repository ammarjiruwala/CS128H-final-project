use std::fs;
use std::path::Path;

use crate::stats::SessionRecord;
use crate::tasks::Task;

const TASKS_FILE:   &str = "pomodoro_tasks.json";
const HISTORY_FILE: &str = "pomodoro_history.json";

// ---------------------------------------------------------------------------
// Tasks
// ---------------------------------------------------------------------------

pub fn save_tasks(tasks: &[Task]) -> Result<(), String> {
    let json = serde_json::to_string_pretty(tasks).map_err(|e| e.to_string())?;
    fs::write(TASKS_FILE, json).map_err(|e| e.to_string())
}

pub fn load_tasks() -> Result<Vec<Task>, String> {
    if !Path::new(TASKS_FILE).exists() {
        return Ok(Vec::new());
    }
    let content = fs::read_to_string(TASKS_FILE).map_err(|e| e.to_string())?;
    serde_json::from_str(&content).map_err(|e| e.to_string())
}

// ---------------------------------------------------------------------------
// Session history
// ---------------------------------------------------------------------------

pub fn save_history(history: &[SessionRecord]) -> Result<(), String> {
    let json = serde_json::to_string_pretty(history).map_err(|e| e.to_string())?;
    fs::write(HISTORY_FILE, json).map_err(|e| e.to_string())
}

pub fn load_history() -> Result<Vec<SessionRecord>, String> {
    if !Path::new(HISTORY_FILE).exists() {
        return Ok(Vec::new());
    }
    let content = fs::read_to_string(HISTORY_FILE).map_err(|e| e.to_string())?;
    serde_json::from_str(&content).map_err(|e| e.to_string())
}
