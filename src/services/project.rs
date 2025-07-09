use crate::sql::{
    CREATE_PROJECT_SQL, DELETE_PROJECT_SQL, SELECT_PROJECTS_SQL, SELECT_PROJECT_SQL,
    UPDATE_PROJECT_SQL,
};
use crate::{db::get_db, model::Project};
use chrono::{DateTime, NaiveDateTime, Utc};

pub fn add(name: String) -> anyhow::Result<Project> {
    let conn = get_db()?;
    let mut stmt = conn.prepare(CREATE_PROJECT_SQL)?;
    match stmt.execute((name.clone(),)) {
        Ok(_) => {}
        Err(e) => {
            if e.to_string().contains("UNIQUE constraint failed") {
                anyhow::bail!("project {} already exists", name);
            } else {
                anyhow::bail!("failed to add project {}: {}", name, e);
            }
        }
    }
    Ok(Project::new(name))
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
                NaiveDateTime::parse_from_str(&row.get::<_, String>(2)?, "%Y-%m-%d %H:%M:%S")
                    .map_err(|e| {
                        rusqlite::Error::FromSqlConversionFailure(
                            2,
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
    let conn = get_db()?;
    let mut stmt = conn.prepare(SELECT_PROJECT_SQL)?;
    let project = stmt
        .query_row((name.clone(),), |row| {
            let id: i32 = row.get(0)?;
            let name: String = row.get(1)?;

            Ok(Project {
                id: Some(id),
                name,
                created: None,
                updated: None,
            })
        })
        .map_err(|e| {
            if e.to_string().contains("no rows returned") {
                anyhow::anyhow!("project {} not found", name)
            } else {
                anyhow::anyhow!("failed to query project {}: {}", name, e)
            }
        })?;
    Ok(project)
}

pub fn delete(name: String) -> anyhow::Result<bool> {
    let conn = get_db()?;
    let mut stmt = conn.prepare(DELETE_PROJECT_SQL)?;
    match stmt.execute((name.clone(),)) {
        Ok(rows) => {
            if rows == 0 {
                anyhow::bail!("project {} not found", name);
            }
        }
        Err(e) => {
            anyhow::bail!("failed to delete project {}: {}", name, e);
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
    let conn = get_db()?;
    let mut stmt = conn.prepare(UPDATE_PROJECT_SQL)?;
    match stmt.execute((new_name.clone(), name.clone())) {
        Ok(rows) => {
            if rows == 0 {
                anyhow::bail!("Project '{}' not found.", name);
            }
        }
        Err(e) => {
            anyhow::bail!("Failed to update project '{}': {}", name, e);
        }
    }
    Ok(true)
}
