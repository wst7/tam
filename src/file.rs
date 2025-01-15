use anyhow::{Context, Ok};
use dirs;
use std::fs::{self, File};
use std::io::Write;
use std::path::PathBuf;

use crate::task::{Task, TaskStatus};

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

fn read_tasks() -> anyhow::Result<Vec<Task>> {
    let tasks_path = tasks_file_path()?;

    let content = fs::read_to_string(tasks_path)?;
    let tasks: Vec<Task> = serde_json::from_str(&content)?;
    Ok(tasks)
}

fn write_tasks(tasks: Vec<Task>) -> anyhow::Result<bool> {
    let tasks_path = tasks_file_path()?;
    let content = serde_json::to_string(&tasks)?;
    fs::write(tasks_path, content)?;
    Ok(true)
}

pub fn add_task(task: Task) -> anyhow::Result<bool> {
    let mut tasks = read_tasks()?;
    tasks.push(task.clone());
    write_tasks(tasks)?;
    anyhow::Ok(true)
}

pub fn update_task(index: usize, title: String) -> anyhow::Result<bool> {
    let mut tasks = read_tasks()?;
    let task = match tasks.get_mut(index - 1) {
        Some(task) => task,
        None => anyhow::bail!("not find task by index: {}", index),
    };

    task.set_title(title.clone());
    write_tasks(tasks)?;
    anyhow::Ok(true)
}

pub fn update_task_status(indexes: &[usize], status: TaskStatus) -> anyhow::Result<bool> {
    let mut tasks = read_tasks()?;
    for index in indexes {
        let task = match tasks.get_mut(index - 1) {
            Some(task) => task,
            None => anyhow::bail!("not find task by index: {}", index),
        };

        task.set_status(status.clone());
    }
    write_tasks(tasks)
}

fn filter_tasks<F>(f: F) -> anyhow::Result<Vec<(usize, Task)>>
where
    F: Fn(&(usize, Task)) -> bool,
{
    let tasks = read_tasks()?;
    let tasks: Vec<(usize, Task)> = tasks
        .into_iter()
        .enumerate()
        .filter(f)
        .filter(|(_, task)| task.status != TaskStatus::Delete)
        .collect();
    anyhow::Ok(tasks)
}

pub fn get_done_tasks() -> anyhow::Result<Vec<(usize, Task)>> {
    filter_tasks(|(_, task)| task.status == TaskStatus::Done)
}

pub fn get_todo_tasks() -> anyhow::Result<Vec<(usize, Task)>> {
    filter_tasks(|(_, task)| task.status == TaskStatus::Todo)
}

pub fn get_in_progress_tasks() -> anyhow::Result<Vec<(usize, Task)>> {
    filter_tasks(|(_, task)| task.status == TaskStatus::InProgress)
}

pub fn get_all_tasks() -> anyhow::Result<Vec<(usize, Task)>> {
    filter_tasks(|_| true)
}
