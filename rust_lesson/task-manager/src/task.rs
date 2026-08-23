use chrono::{DateTime, Local};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    pub id: usize,
    pub title: String,
    pub description: String,
    pub completed: bool,
    pub created_at: DateTime<Local>,
    pub updated_at: DateTime<Local>,
}

impl Task {
    pub fn new(id: usize, title: &str, description: &str) -> Self {
        let now = Local::now();
        Task {
            id,
            title: title.to_string(),
            description: description.to_string(),
            completed: false,
            created_at: now,
            updated_at: now,
        }
    }

    pub fn toggle_complete(&mut self) {
        self.completed = !self.completed;
        self.updated_at = Local::now();
    }

    pub fn update(&mut self, title: &str, description: &str) {
        self.title = title.to_string();
        self.description = description.to_string();
        self.updated_at = Local::now();
    }
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct TaskList {
    pub tasks: Vec<Task>,
    next_id: usize,
}

impl TaskList {
    pub fn new() -> Self {
        TaskList {
            tasks: Vec::new(),
            next_id: 1,
        }
    }

    pub fn add(&mut self, title: &str, description: &str) -> &Task {
        let task = Task::new(self.next_id, title, description);
        self.tasks.push(task);
        self.next_id += 1;
        self.tasks.last().unwrap()
    }

    pub fn get(&self, id: usize) -> Option<&Task> {
        self.tasks.iter().find(|t| t.id == id)
    }

    pub fn get_mut(&mut self, id: usize) -> Option<&mut Task> {
        self.tasks.iter_mut().find(|t| t.id == id)
    }

    pub fn remove(&mut self, id: usize) -> Option<Task> {
        let pos = self.tasks.iter().position(|t| t.id == id)?;
        Some(self.tasks.remove(pos))
    }

    pub fn toggle(&mut self, id: usize) -> Option<bool> {
        let task = self.get_mut(id)?;
        task.toggle_complete();
        Some(task.completed)
    }

    pub fn list_all(&self) -> &[Task] {
        &self.tasks
    }

    pub fn list_completed(&self) -> Vec<&Task> {
        self.tasks.iter().filter(|t| t.completed).collect()
    }

    pub fn list_pending(&self) -> Vec<&Task> {
        self.tasks.iter().filter(|t| !t.completed).collect()
    }

    pub fn is_empty(&self) -> bool {
        self.tasks.is_empty()
    }
}
