use crate::agent::{self, AgentError};
use crate::sql::{
    COUNT_ACTIVE_TASKS_SQL, CREATE_PROJECT_SQL, DELETE_PROJECT_SQL, RESTORE_PROJECT_SQL,
    SELECT_PROJECTS_SQL, SELECT_PROJECT_INCLUDING_DELETED_SQL, SELECT_PROJECT_SQL,
    UPDATE_PROJECT_SQL,
};
use crate::{db::get_db, model::Project};
use chrono::{DateTime, NaiveDateTime, Utc};
use rusqlite::OptionalExtension;

/// Canonicalize a project name coming from the CLI or the TUI: surrounding
/// whitespace is ignored (the database stores the canonical form) and blank
/// names are rejected.
pub(crate) fn normalize_project_name(raw: &str) -> anyhow::Result<String> {
    let name = raw.trim();
    if name.is_empty() {
        return Err(agent::to_anyhow(AgentError::invalid(
            "project name cannot be empty",
        )));
    }
    Ok(name.to_string())
}

fn row_datetime(row: &rusqlite::Row<'_>, index: usize) -> rusqlite::Result<DateTime<Utc>> {
    let value: String = row.get(index)?;
    let naive = NaiveDateTime::parse_from_str(&value, "%Y-%m-%d %H:%M:%S").map_err(|e| {
        rusqlite::Error::FromSqlConversionFailure(index, rusqlite::types::Type::Text, Box::new(e))
    })?;
    Ok(DateTime::<Utc>::from_naive_utc_and_offset(naive, Utc))
}

/// Add a project. Returns the project and whether an existing soft-deleted
/// row was restored instead of inserting a new one.
pub fn add(name: String) -> anyhow::Result<(Project, bool)> {
    let name = normalize_project_name(&name)?;
    let conn = get_db()?;

    // `project.name` is UNIQUE across soft-deleted rows: same name + deleted
    // row means "restore", same name + active row means "already exists".
    let target_state = conn
        .query_row(
            SELECT_PROJECT_INCLUDING_DELETED_SQL,
            (name.clone(),),
            |row| row.get::<_, bool>(0),
        )
        .optional()?;
    if let Some(is_deleted) = target_state {
        if !is_deleted {
            return Err(agent::to_anyhow(AgentError::conflict(format!(
                "project '{}' already exists",
                name
            ))));
        }
        conn.execute(RESTORE_PROJECT_SQL, (name.clone(),))
            .map_err(|e| anyhow::anyhow!("failed to restore project '{}': {}", name, e))?;
        return Ok((Project::new(name), true));
    }

    let mut stmt = conn.prepare(CREATE_PROJECT_SQL)?;
    match stmt.execute((name.clone(),)) {
        Ok(_) => {}
        Err(e) => {
            if e.to_string().contains("UNIQUE constraint failed") {
                return Err(agent::to_anyhow(AgentError::conflict(format!(
                    "project '{}' already exists",
                    name
                ))));
            } else {
                anyhow::bail!("failed to add project '{}': {}", name, e);
            }
        }
    }
    Ok((Project::new(name), false))
}
pub fn list() -> anyhow::Result<Vec<Project>> {
    let conn = get_db()?;
    let mut stmt = conn.prepare(SELECT_PROJECTS_SQL)?;
    let projects = stmt
        .query_map((), |row| {
            let id: i32 = row.get(0)?;
            let name: String = row.get(1)?;
            let created =
                NaiveDateTime::parse_from_str(&row.get::<_, String>(2)?, "%Y-%m-%d %H:%M:%S")
                    .map_err(|e| {
                        rusqlite::Error::FromSqlConversionFailure(
                            1,
                            rusqlite::types::Type::Text,
                            Box::new(e),
                        )
                    })?;
            let updated =
                NaiveDateTime::parse_from_str(&row.get::<_, String>(3)?, "%Y-%m-%d %H:%M:%S")
                    .map_err(|e| {
                        rusqlite::Error::FromSqlConversionFailure(
                            3,
                            rusqlite::types::Type::Text,
                            Box::new(e),
                        )
                    })?;
            Ok(Project {
                id: Some(id),
                name,
                created: Some(DateTime::<Utc>::from_naive_utc_and_offset(created, Utc)),
                updated: Some(DateTime::<Utc>::from_naive_utc_and_offset(updated, Utc)),
            })
        })?
        .collect::<Result<Vec<Project>, _>>()?;
    Ok(projects)
}

pub fn query_project(name: String) -> anyhow::Result<Project> {
    let name = normalize_project_name(&name)?;
    let conn = get_db()?;
    let mut stmt = conn.prepare(SELECT_PROJECT_SQL)?;
    let project = stmt
        .query_row((name.clone(),), |row| {
            let id: i32 = row.get(0)?;
            let name: String = row.get(1)?;
            let created = row_datetime(row, 2)?;
            let updated = row_datetime(row, 3)?;

            Ok(Project {
                id: Some(id),
                name,
                created: Some(created),
                updated: Some(updated),
            })
        })
        .map_err(|e| match e {
            rusqlite::Error::QueryReturnedNoRows => {
                anyhow::anyhow!("project '{}' not found", name)
            }
            other => anyhow::anyhow!("failed to query project '{}': {}", name, other),
        })?;
    Ok(project)
}

pub fn delete(name: String) -> anyhow::Result<bool> {
    let name = normalize_project_name(&name)?;

    // Only active projects can be deleted; soft-deleted rows report not found.
    let project = query_project(name.clone())?;
    let project_id = project
        .id
        .ok_or_else(|| anyhow::anyhow!("project '{}' not found", name))?;

    // Refuse to delete a project that still has active tasks: they would
    // become orphaned because the FK is not enforced (Q1).
    let conn = get_db()?;
    let active_tasks: i64 =
        conn.query_row(COUNT_ACTIVE_TASKS_SQL, (project_id,), |row| row.get(0))?;
    if active_tasks > 0 {
        return Err(agent::to_anyhow(AgentError::conflict(format!(
            "project '{}' has {} active task(s)",
            name, active_tasks
        ))));
    }

    let mut stmt = conn.prepare(DELETE_PROJECT_SQL)?;
    match stmt.execute((name.clone(),)) {
        Ok(rows) => {
            if rows == 0 {
                return Err(agent::to_anyhow(AgentError::not_found(format!(
                    "project '{}' not found",
                    name
                ))));
            }
        }
        Err(e) => {
            anyhow::bail!("failed to delete project '{}': {}", name, e);
        }
    }
    Ok(true)
}

pub fn get_current_project() -> anyhow::Result<Project> {
    let config = crate::config::get_config();
    if let Some(name) = config.current_project {
        query_project(name)
    } else {
        anyhow::bail!("No current project set. Use `tam project use <name>` to set a project.");
    }
}

pub fn edit(name: String, new_name: String) -> anyhow::Result<bool> {
    let name = normalize_project_name(&name)?;
    let new_name = normalize_project_name(&new_name)?;

    // Renaming a project to its own name is a no-op. This also covers the TUI
    // path, which calls the service directly (the CLI short-circuits earlier).
    if name == new_name {
        return Ok(true);
    }

    // The source must be an active project, otherwise report a friendly error
    // instead of a raw SQL failure (soft-deleted rows are not editable).
    query_project(name.clone())?;

    // `project.name` is UNIQUE across soft-deleted rows, so a target name can
    // collide with a deleted project: give a dedicated, actionable message.
    let conn = get_db()?;
    let target_state = conn
        .query_row(
            SELECT_PROJECT_INCLUDING_DELETED_SQL,
            (new_name.clone(),),
            |row| row.get::<_, bool>(0),
        )
        .optional()?;
    if let Some(is_deleted) = target_state {
        if is_deleted {
            return Err(agent::to_anyhow(AgentError::conflict(format!(
                "name '{}' belongs to a deleted project; run `tam project add {}` to restore it or choose another name",
                new_name, new_name
            ))));
        }
        return Err(agent::to_anyhow(AgentError::conflict(format!(
            "project '{}' already exists",
            new_name
        ))));
    }

    let mut stmt = conn.prepare(UPDATE_PROJECT_SQL)?;
    match stmt.execute((new_name.clone(), name.clone())) {
        Ok(rows) => {
            if rows == 0 {
                anyhow::bail!("project '{}' not found", name);
            }
        }
        Err(e) => {
            anyhow::bail!("failed to update project '{}': {}", name, e);
        }
    }
    Ok(true)
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn project_name_normalization_trims_and_rejects_blank() {
        assert_eq!(normalize_project_name("  alpha  ").unwrap(), "alpha");
        assert_eq!(normalize_project_name("beta").unwrap(), "beta");
        assert!(normalize_project_name("").is_err());
        assert!(normalize_project_name("   ").is_err());
        assert!(normalize_project_name("\t\n").is_err());
    }
}
