use chrono::{DateTime, Utc};
use rusqlite::{
    types::{FromSql, FromSqlError, FromSqlResult, ValueRef},
    ToSql,
};
use serde::{Deserialize, Serialize};
use tabled::{builder::Builder, settings::Style};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Project {
    pub id: Option<i32>,
    pub name: String,
    pub created: Option<DateTime<Utc>>,
    pub updated: Option<DateTime<Utc>>,
}

impl Project {
    pub fn new(name: String) -> Self {
        Self {
            id: None,
            name,
            created: None,
            updated: None,
        }
    }
    pub fn print(list: &[Self]) {
        let mut builder = Builder::default();
        builder.push_record(["Project", "Created", "Updated"]);
        for item in list {
            builder.push_record([
                item.name.clone(),
                item.created
                    .as_ref()
                    .map(|d| d.format("%Y-%m-%d %H:%M:%S").to_string())
                    .unwrap_or("".to_string()),
                item.updated
                    .as_ref()
                    .map(|d| d.format("%Y-%m-%d %H:%M:%S").to_string())
                    .unwrap_or("".to_string()),
            ]);
        }

        let mut table = builder.build();
        table.with(Style::rounded());
        println!("{}", table);
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    pub id: Option<i32>,

    pub project_id: i32,

    /// Project name, resolved via JOIN when the task is loaded.
    pub project: String,

    pub title: String,

    /// Task description/context. The column existed from the start but was
    /// unused before the agent-first extension.
    pub content: String,

    pub status: TaskStatus,

    /// 1 (highest) .. 5 (lowest).
    pub priority: i32,

    /// Canonical comma separated tag list (lowercase, deduplicated).
    pub tags: String,

    pub assigned_agent: Option<String>,

    /// Unix seconds; `None` means no lease.
    pub lease_expires_at: Option<i64>,

    /// Incremented on every effective write; used for optimistic concurrency.
    pub revision: i32,

    pub created: DateTime<Utc>,

    pub updated: DateTime<Utc>,
}

/// One row of the `task_events` audit log.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskEvent {
    pub id: i32,
    pub task_id: i32,
    pub project_id: i32,
    pub event: String,
    pub actor: String,
    pub from_status: Option<TaskStatus>,
    pub to_status: Option<TaskStatus>,
    /// Raw JSON metadata (stored as TEXT).
    pub metadata: String,
    pub created: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum TaskStatus {
    Todo = 0,
    InProgress = 1,
    Done = 2,
}

impl TaskStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            TaskStatus::Todo => "todo",
            TaskStatus::InProgress => "in-progress",
            TaskStatus::Done => "done",
        }
    }

    pub fn is_done(&self) -> bool {
        self == &TaskStatus::Done
    }
    pub fn is_todo(&self) -> bool {
        self == &TaskStatus::Todo
    }
    pub fn is_in_progress(&self) -> bool {
        self == &TaskStatus::InProgress
    }
    pub fn next(&self) -> Self {
        match self {
            TaskStatus::Todo => TaskStatus::InProgress,
            TaskStatus::InProgress => TaskStatus::Done,
            TaskStatus::Done => TaskStatus::Todo,
        }
    }
    pub fn prev(&self) -> Self {
        match self {
            TaskStatus::Todo => TaskStatus::Done,
            TaskStatus::InProgress => TaskStatus::Todo,
            TaskStatus::Done => TaskStatus::InProgress,
        }
    }
}

impl ToSql for TaskStatus {
    fn to_sql(&self) -> rusqlite::Result<rusqlite::types::ToSqlOutput<'_>> {
        Ok((*self as i32).into())
    }
}

impl FromSql for TaskStatus {
    fn column_result(value: ValueRef<'_>) -> FromSqlResult<Self> {
        match value.as_i64()? {
            0 => Ok(TaskStatus::Todo),
            1 => Ok(TaskStatus::InProgress),
            2 => Ok(TaskStatus::Done),
            _ => Err(FromSqlError::Other("Invalid TaskStatus value".into())),
        }
    }
}

impl Default for Task {
    fn default() -> Self {
        Task {
            id: None,
            project_id: 0,
            project: String::new(),
            title: "".to_string(),
            content: String::new(),
            status: TaskStatus::Todo,
            priority: 3,
            tags: String::new(),
            assigned_agent: None,
            lease_expires_at: None,
            revision: 1,
            created: Utc::now(),
            updated: Utc::now(),
        }
    }
}

impl ToString for TaskStatus {
    fn to_string(&self) -> String {
        match self {
            TaskStatus::Todo => "⏳ Todo".to_string(),
            TaskStatus::InProgress => "🚧 In Progress".to_string(),
            TaskStatus::Done => "✅ Done".to_string(),
        }
    }
}

impl Task {
    pub fn set_title(&mut self, title: String) {
        self.title = title;
        self.updated = Utc::now();
    }

    pub fn print(list: &[Self]) {
        let mut builder = Builder::default();
        builder.push_record(["ID", "Title", "Status"]);
        for item in list {
            builder.push_record([
                item.id.unwrap().to_string(),
                item.title.clone(),
                item.status.to_string(),
                // item.created.to_string(),
                // item.updated.to_string(),
            ]);
        }

        let mut table = builder.build();
        table.with(Style::rounded());
        println!("{}", table);
    }
}
