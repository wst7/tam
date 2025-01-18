use crate::{
    query::{add_task, update_task, update_task_status},
    print_success,
    task::{Task, TaskStatus},
};

/// add new task
pub fn add(title: String) -> anyhow::Result<bool> {
    let mut task = Task::default();
    task.set_title(title.clone());
    add_task(task)?;
    print_success!("Task: {} is add successfully", title);
    anyhow::Ok(true)
}

/// update task title
pub fn update(index: usize, title: String) -> anyhow::Result<bool> {
    update_task(index, title.clone())?;
    print_success!("Task: {} is update successfully", title);
    anyhow::Ok(true)
}

/// start task
pub fn start(indexes: &[usize]) -> anyhow::Result<bool> {
    update_task_status(indexes, TaskStatus::InProgress)?;
    print_success!("Task: {:?} is in progress", indexes);
    anyhow::Ok(true)
}

// complete task
pub fn done(indexes: &[usize]) -> anyhow::Result<bool> {
    update_task_status(indexes, TaskStatus::Done)?;
    print_success!("Task: {:?} is in done", indexes);
    anyhow::Ok(true)
}

// remove tasks
pub fn remove(indexes: &[usize]) -> anyhow::Result<bool> {
    update_task_status(indexes, TaskStatus::Delete)?;
    print_success!("Task: {:?} is in removed", indexes);
    anyhow::Ok(true)
}
