use std::fmt;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    pub title: String,

    pub status: TaskStatus,

    pub created: DateTime<Utc>,

    pub updated: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum TaskStatus {
    Done,
    Todo,
    InProgress,
    Delete,
}

impl fmt::Display for TaskStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TaskStatus::Done => write!(f, "✅ Done"),
            TaskStatus::Todo => write!(f, "⏳ Todo"),
            TaskStatus::InProgress => write!(f, "🚧 In Progress"),
            TaskStatus::Delete => write!(f, "🌲 Delete"),
        }
    }
}

impl Default for Task {
    fn default() -> Self {
        Task {
            title: "".to_string(),
            status: TaskStatus::Todo,
            created: Utc::now(),
            updated: Utc::now(),
        }
    }
}

impl Task {
    pub fn set_title(&mut self, title: String) {
        self.title = title;
        self.updated = Utc::now();
    }
    pub fn set_status(&mut self, status: TaskStatus) {
        self.status = status;
        self.updated = Utc::now();
    }
}
