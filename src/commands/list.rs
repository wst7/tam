use chrono::{DateTime, Utc};
use tabled::{builder::Builder, settings::Style};

use crate::{
    file::read_tasks,
    task::{Task, TaskStatus},
};

pub fn list_done() -> anyhow::Result<bool> {
    let tasks = filter_tasks(|(_, task)| task.status == TaskStatus::Done)?;
    print_table(&tasks);
    anyhow::Ok(true)
}

pub fn list_todo() -> anyhow::Result<bool> {
    let tasks = filter_tasks(|(_, task)| task.status == TaskStatus::Todo)?;
    print_table(&tasks);
    anyhow::Ok(true)
}

pub fn list_in_progress() -> anyhow::Result<bool> {
    let tasks = filter_tasks(|(_, task)| task.status == TaskStatus::InProgress)?;
    print_table(&tasks);
    anyhow::Ok(true)
}

pub fn list_all() -> anyhow::Result<bool> {
    let tasks = filter_tasks(|_| true)?;
    print_table(&tasks);
    anyhow::Ok(true)
}

fn print_table(list: &[(usize, Task)]) {
    let mut builder = Builder::default();
    builder.push_record(["Index", "Title", "Status", "Created"]);
    for (index, task) in list {
        builder.push_record([
            (index + 1).to_string(),
            task.title.clone(),
            task.status.to_string(),
            format_datetime(&task.created),
        ]);
    }

    let mut table = builder.build();
    table.with(Style::rounded());
    println!("{}", table);
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

fn format_datetime(datetime: &DateTime<Utc>) -> String {
    datetime.format("%Y-%m-%d %H:%M:%S").to_string()
}
