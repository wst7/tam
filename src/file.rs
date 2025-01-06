use anyhow::{Context, Ok};
use dirs;
use std::fs::{self, File};
use std::io::Write;
use std::path::PathBuf;

use crate::task::Task;

fn tasks_file_path() -> anyhow::Result<PathBuf> {
    let home = dirs::home_dir()
        .with_context(|| format!("fail to get home dir"))?
        .join(".config/tasks.json");

    if !home.exists() {
        let mut file = File::create(&home)?;
        file.write(b"[]")?;
    }
    Ok(home)
}

pub fn read_tasks() -> anyhow::Result<Vec<Task>> {
    let tasks_path = tasks_file_path()?;

    let content = fs::read_to_string(tasks_path)?;
    let tasks: Vec<Task> = serde_json::from_str(&content)?;
    Ok(tasks)
}

pub fn write_tasks(tasks: Vec<Task>) -> anyhow::Result<bool> {
    let tasks_path = tasks_file_path()?;
    let content = serde_json::to_string(&tasks)?;
    fs::write(tasks_path, content)?;
    Ok(true)
}
