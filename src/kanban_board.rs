use crate::task::Task;
use chrono::Local;
use std::{fs, path::Path};

#[derive(Debug, Default)]
pub struct KanbanBoard {
    pub tasks: Vec<Task>,
    pub selected_status: usize,
    pub selected_task: usize,
}

impl KanbanBoard {
    pub fn new() -> Self {
        Self {
            tasks: Vec::new(),
            selected_status: 0,
            selected_task: 0,
        }
    }

    pub fn save_to_file(&self) {
        let file_path = "kanban_board.json";
        if let Err(err) = fs::write(
            file_path,
            serde_json::to_string_pretty(&self.tasks).unwrap(),
        ) {
            eprintln!("Failed to save tasks: {}", err);
        }
    }

    pub fn load_from_file(&mut self) {
        let file_path = "kanban_board.json";
        if Path::new(file_path).exists() {
            if let Ok(data) = fs::read_to_string(file_path) {
                self.tasks = serde_json::from_str(&data).unwrap_or_else(|_| Vec::new());
            }
        }
    }

    pub fn add_task(&mut self, description: String, due_date: String) {
        let id = self.tasks.len() as u32 + 1;
        let created_at = Local::now().format("%Y-%m-%d").to_string();
        let task = Task {
            id,
            description,
            created_at,
            due_date,
            status: "TODO".to_string(),
        };
        self.tasks.push(task);
    }

    pub fn move_task(&mut self, direction: isize) {
        let statuses = ["TODO", "DOING", "DONE"];
        if let Some(task) = self
            .tasks
            .iter_mut()
            .find(|t| t.status == statuses[self.selected_status])
        {
            let new_status_index = (self.selected_status as isize + direction)
                .clamp(0, statuses.len() as isize - 1) as usize;
            task.status = statuses[new_status_index].to_string();
        }
    }

    pub fn get_tasks_by_status(&self, status: &str) -> Vec<&Task> {
        self.tasks.iter().filter(|t| t.status == status).collect()
    }
}
