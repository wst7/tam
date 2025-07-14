use chrono::{DateTime, NaiveDateTime, Utc};

use crate::{
    db,
    model::{Task, TaskStatus},
    services::project,
    sql::{
        CREATE_TASK_SQL, DELETE_TASK_SQL, SELECT_ALL_TASKS_SQL, SELECT_TASK_SQL,
        UPDATE_TASK_STATUS_SQL, UPDATE_TASK_TITLE_SQL,
    },
};

pub fn add(title: String, p_id: Option<i32>) -> anyhow::Result<bool> {
    log::info!("add task: {} {:?}", title, p_id);
    let db = db::get_db()?;

    let project_id = match p_id {
        Some(id) => id,
        None => {
            let project = project::get_current_project()?;
            if project.id.is_none() {
                anyhow::bail!(
                    "No current project set. Use `tam project use <name>` to set a project."
                );
            }
            project.id.unwrap()
        }
    };
    log::info!("add task: {} {:?}", title, project_id);
    let mut stmt = db.prepare(CREATE_TASK_SQL)?;
    stmt.execute((title, project_id))?;

    Ok(true)
}

pub fn list(project_id: i32) -> anyhow::Result<Vec<Task>> {
    let db = db::get_db()?;
    let mut stmt = db.prepare(SELECT_ALL_TASKS_SQL)?;

    let tasks = stmt
        .query_map((project_id,), |row| {
            let id: i32 = row.get(0)?;
            let title: String = row.get(1)?;
            let status: TaskStatus = row.get(2)?;
            let created =
                NaiveDateTime::parse_from_str(&row.get::<_, String>(3)?, "%Y-%m-%d %H:%M:%S")
                    .map_err(|e| {
                        rusqlite::Error::FromSqlConversionFailure(
                            3,
                            rusqlite::types::Type::Text,
                            Box::new(e),
                        )
                    })?;
            let updated =
                NaiveDateTime::parse_from_str(&row.get::<_, String>(4)?, "%Y-%m-%d %H:%M:%S")
                    .map_err(|e| {
                        rusqlite::Error::FromSqlConversionFailure(
                            4,
                            rusqlite::types::Type::Text,
                            Box::new(e),
                        )
                    })?;
            Ok(Task {
                id: Some(id),
                title,
                status,
                created: DateTime::<Utc>::from_naive_utc_and_offset(created, Utc),
                updated: DateTime::<Utc>::from_naive_utc_and_offset(updated, Utc),
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(tasks)
}

fn query_task(id: String) -> anyhow::Result<Task> {
    let db = db::get_db()?;
    let mut stmt = db.prepare(SELECT_TASK_SQL)?;
    let task: Task = stmt.query_row((id,), |row| {
        let id: i32 = row.get(0)?;
        let title: String = row.get(1)?;
        let status: TaskStatus = row.get(2)?;
        let created = row.get::<_, String>(3)?;
        let updated = row.get::<_, String>(4)?;
        Ok(Task {
            id: Some(id),
            title,
            status,
            created: created
                .parse::<DateTime<Utc>>()
                .unwrap_or_else(|_| Utc::now()),
            updated: updated
                .parse::<DateTime<Utc>>()
                .unwrap_or_else(|_| Utc::now()),
        })
    })?;
    Ok(task)
}
pub fn start(id: String) -> anyhow::Result<bool> {
    let db = db::get_db()?;
    let task = query_task(id.clone())?;
    if task.status == TaskStatus::InProgress {
        return Err(anyhow::anyhow!(
            "Task '{}' is already in progress.",
            task.title
        ));
    }

    // Update task status to InProgress
    let mut update_stmt = db.prepare(UPDATE_TASK_STATUS_SQL)?;
    update_stmt.execute((TaskStatus::InProgress, task.id.unwrap()))?;

    Ok(true)
}

pub fn done(id: String) -> anyhow::Result<bool> {
    let db = db::get_db()?;
    let task = query_task(id.clone())?;
    if task.status == TaskStatus::Done {
        return Err(anyhow::anyhow!("Task '{}' is already done.", task.title));
    }

    // Update task status to Done
    let mut update_stmt = db.prepare(UPDATE_TASK_STATUS_SQL)?;
    update_stmt.execute((TaskStatus::Done, task.id.unwrap()))?;

    Ok(true)
}

pub fn edit(id: String, title: String) -> anyhow::Result<bool> {
    let db = db::get_db()?;
    let task = query_task(id.clone())?;

    // Update task title
    let mut update_stmt = db.prepare(UPDATE_TASK_TITLE_SQL)?;
    update_stmt.execute((title, task.id.unwrap()))?;

    Ok(true)
}

pub fn delete(id: String) -> anyhow::Result<bool> {
    let db = db::get_db()?;
    let task = query_task(id.clone())?;

    // Delete the task
    let mut stmt = db.prepare(DELETE_TASK_SQL)?;
    stmt.execute((task.id.unwrap(),))?;

    Ok(true)
}
