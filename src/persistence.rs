/// Persistence layer — Tanay's module.
/// Actual serialization implemented at Checkpoint 2.

use crate::tasks::Task;

pub fn save_state(_tasks: &[Task]) -> Result<(), String> {
    todo!("Tanay: serialize tasks to JSON and write to disk")
}

pub fn load_state() -> Result<Vec<Task>, String> {
    todo!("Tanay: read JSON from disk and deserialize into Vec<Task>")
}
