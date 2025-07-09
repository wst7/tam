use std::{fs, path};

use crate::sql::{
    CREATE_PROJECT_TABLE_SQL, CREATE_PROJECT_TABLE_TRIGGER_SQL, CREATE_TASK_TABLE_SQL,
    CREATE_TASK_TABLE_TRIGGER_SQL,
};
use rusqlite::Connection;

use crate::config::get_config;

pub fn get_db() -> anyhow::Result<Connection> {
    let c = get_config();

    // If the database file dir does not exist, create it
    if let Some(parent) = path::Path::new(&c.db_file).parent() {
        fs::create_dir_all(parent)?;
    }
    // Open the database file, it will be created if it does not exist
    let conn = Connection::open(c.db_file)?;
    return Ok(conn);
}

pub fn init_db() -> anyhow::Result<bool> {
    let conn = get_db()?;
    conn.execute(CREATE_PROJECT_TABLE_SQL, ())?;
    conn.execute(CREATE_PROJECT_TABLE_TRIGGER_SQL, ())?;
    conn.execute(CREATE_TASK_TABLE_SQL, ())?;
    conn.execute(CREATE_TASK_TABLE_TRIGGER_SQL, ())?;
    Ok(true)
}

#[cfg(test)]
mod test {
    use super::*;
}
