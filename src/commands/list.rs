use chrono::{DateTime, Utc};
use tabled::{builder::Builder, settings::Style};

use crate::{
    query::{get_all_tasks, get_done_tasks, get_in_progress_tasks, get_todo_tasks},
    task::Task,
};

pub fn list_done() -> anyhow::Result<bool> {
    let tasks = get_done_tasks()?;
    print_table(&tasks);
    anyhow::Ok(true)
}

pub fn list_todo() -> anyhow::Result<bool> {
    let tasks = get_todo_tasks()?;
    print_table(&tasks);
    anyhow::Ok(true)
}

pub fn list_in_progress() -> anyhow::Result<bool> {
    let tasks = get_in_progress_tasks()?;
    print_table(&tasks);
    anyhow::Ok(true)
}

pub fn list_all() -> anyhow::Result<bool> {
    let tasks = get_all_tasks()?;
    print_table(&tasks);
    anyhow::Ok(true)
}

fn print_table(list: &[(usize, Task)]) {
    let mut builder = Builder::default();
    builder.push_record(["Index", "Title", "Status", "Created"]);
    for (index, task) in list {
        builder.push_record([
            (index).to_string(),
            task.title.clone(),
            task.status.to_string(),
            format_datetime(&task.created),
        ]);
    }

    let mut table = builder.build();
    table.with(Style::rounded());
    println!("{}", table);
}

fn format_datetime(datetime: &DateTime<Utc>) -> String {
    datetime.format("%Y-%m-%d %H:%M:%S").to_string()
}
