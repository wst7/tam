use chrono::{DateTime, Utc};
use tabled::{builder::Builder, settings::Style};

use crate::{
    file::read_tasks,
    task::{Task, TaskStatus},
};

pub fn list_done() -> anyhow::Result<bool> {
    let tasks = filter_tasks(|task| task.status == TaskStatus::Done)?;
    print_table(&tasks);
    anyhow::Ok(true)
}

pub fn list_todo() -> anyhow::Result<bool> {
    let tasks = filter_tasks(|task| task.status == TaskStatus::Todo)?;
    print_table(&tasks);
    anyhow::Ok(true)
}

pub fn list_in_progress() -> anyhow::Result<bool> {
    let tasks = filter_tasks(|task| task.status == TaskStatus::InProgress)?;
    print_table(&tasks);
    anyhow::Ok(true)
}

pub fn list_all() -> anyhow::Result<bool> {
    let tasks = filter_tasks(|_| true)?;
    print_table(&tasks);
    anyhow::Ok(true)
}

fn print_table(tasks: &[Task]) {
    let mut builder = Builder::default();
    builder.push_record(["Index", "Title", "Status", "Created"]);
    for (index, task) in tasks.iter().enumerate() {
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

fn filter_tasks<F>(f: F) -> anyhow::Result<Vec<Task>>
where
    F: Fn(&Task) -> bool,
{
    let tasks = read_tasks()?;
    let tasks: Vec<Task> = tasks
        .into_iter()
        .filter(f)
        .filter(|task| task.status != TaskStatus::Delete)
        .collect();
    anyhow::Ok(tasks)
}

fn format_datetime(datetime: &DateTime<Utc>) -> String {
    datetime.format("%Y-%m-%d %H:%M:%S").to_string()
}
