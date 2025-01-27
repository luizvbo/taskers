use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Task {
    pub id: u32,
    pub description: String,
    pub created_at: String,
    pub due_date: String,
    pub status: String, // "TODO", "DOING", "DONE"
}
