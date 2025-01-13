use std::fmt;

use chrono::{DateTime, Utc};
use cursive_table_view::TableViewItem;
use serde::{Deserialize, Serialize};

use crate::ui::BasicColumn;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    pub title: String,

    pub status: TaskStatus,

    pub created: DateTime<Utc>,

    pub updated: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, PartialOrd, Eq, Ord)]
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

impl TableViewItem<BasicColumn> for Task {
    fn to_column(&self, column: BasicColumn) -> String {
        match column {
            BasicColumn::Title => self.title.clone(),
            BasicColumn::Status => self.status.to_string(),
            BasicColumn::Created => self.created.to_string(),
            _ => "index".to_string()
        }
    }
    fn cmp(&self, other: &Self, column: BasicColumn) -> std::cmp::Ordering
        where
            Self: Sized {
        match column {
            BasicColumn::Title => self.title.cmp(&other.title),
            BasicColumn::Status => self.status.cmp(&other.status),
            BasicColumn::Created => self.created.cmp(&other.created),
            _ => self.title.cmp(&other.title),
        }
    }
}