/// Stub — Tanay's module.
/// Task struct and queue operations (add, complete, delete) will be implemented here.

#[derive(Debug, Clone)]
pub struct Task {
    pub id: u32,
    pub title: String,
}

pub struct TaskQueue {
    tasks: Vec<Task>,
}

impl TaskQueue {
    pub fn new() -> Self {
        TaskQueue { tasks: Vec::new() }
    }

    pub fn add_task(&mut self, _title: &str) {
        todo!("Tanay: implement add_task")
    }

    pub fn complete_task(&mut self, _id: u32) {
        todo!("Tanay: implement complete_task")
    }

    pub fn delete_task(&mut self, _id: u32) {
        todo!("Tanay: implement delete_task")
    }
}
