/// Task struct and queue operations — Tanay's module.

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TaskStatus {
    Todo,
    InProgress,
    Done,
}

#[derive(Debug, Clone)]
pub struct Task {
    pub id: u32,
    pub title: String,
    pub status: TaskStatus,
    pub pomodoros_completed: u32,
}

pub struct TaskQueue {
    tasks: Vec<Task>,
    next_id: u32,
}

impl TaskQueue {
    pub fn new() -> Self {
        TaskQueue {
            tasks: Vec::new(),
            next_id: 1,
        }
    }

    pub fn add_task(&mut self, title: &str) {
        self.tasks.push(Task {
            id: self.next_id,
            title: title.to_string(),
            status: TaskStatus::Todo,
            pomodoros_completed: 0,
        });
        self.next_id += 1;
    }

    pub fn complete_task(&mut self, id: u32) {
        if let Some(task) = self.tasks.iter_mut().find(|t| t.id == id) {
            task.status = TaskStatus::Done;
        }
    }

    pub fn delete_task(&mut self, id: u32) {
        self.tasks.retain(|t| t.id != id);
    }

    pub fn tasks(&self) -> &[Task] {
        &self.tasks
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_add_task() {
        let mut queue = TaskQueue::new();
        queue.add_task("Write project report");
        let tasks = queue.tasks();
        assert_eq!(tasks.len(), 1);
        assert_eq!(tasks[0].title, "Write project report");
        assert_eq!(tasks[0].status, TaskStatus::Todo);
        assert_eq!(tasks[0].pomodoros_completed, 0);
    }

    #[test]
    fn test_complete_task() {
        let mut queue = TaskQueue::new();
        queue.add_task("Review pull requests");
        let id = queue.tasks()[0].id;
        queue.complete_task(id);
        assert_eq!(queue.tasks()[0].status, TaskStatus::Done);
    }

    #[test]
    fn test_delete_task() {
        let mut queue = TaskQueue::new();
        queue.add_task("Set up repo");
        let id = queue.tasks()[0].id;
        queue.delete_task(id);
        assert!(queue.tasks().is_empty());
    }
}
