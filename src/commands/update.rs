use crate::{
    file::{read_tasks, write_tasks},
    print_success,
    task::{Task, TaskStatus},
};

/// add new task
pub fn add(title: String) -> anyhow::Result<bool> {
    let mut task = Task::default();
    task.set_title(title.clone());
    let mut tasks = read_tasks()?;
    tasks.push(task.clone());
    write_tasks(tasks)?;
    print_success!("Task: {} is add successfully", title);
    anyhow::Ok(true)
}

/// update task title
pub fn update(index: usize, title: String) -> anyhow::Result<bool> {
    let mut tasks = read_tasks()?;
    let task = match tasks.get_mut(index - 1) {
        Some(task) => task,
        None => anyhow::bail!("not find task by index: {}", index),
    };

    task.set_title(title.clone());
    write_tasks(tasks)?;
    print_success!("Task: {} is update successfully", title);
    anyhow::Ok(true)
}

/// start task
pub fn start(indexes: &[usize]) -> anyhow::Result<bool> {
    let mut tasks = read_tasks()?;
    for index in indexes {
        let task = match tasks.get_mut(index - 1) {
            Some(task) => task,
            None => anyhow::bail!("not find task by index: {}", index),
        };

        task.set_status(crate::task::TaskStatus::InProgress);
        let title = task.clone().title;
        print_success!("Task: {} is in progress", title);
    }
    write_tasks(tasks)?;

    anyhow::Ok(true)
}

// complete task
pub fn done(indexes: &[usize]) -> anyhow::Result<bool> {
    let mut tasks = read_tasks()?;
    for index in indexes {
        let task = match tasks.get_mut(index - 1) {
            Some(task) => task,
            None => anyhow::bail!("not find task by index: {}", index),
        };
        let title = task.clone().title;
        task.set_status(crate::task::TaskStatus::Done);
        print_success!("Task: {} is done", title);
    }
    write_tasks(tasks)?;
    anyhow::Ok(true)
}

// remove tasks
pub fn remove(indexes: &[usize]) -> anyhow::Result<bool> {
    let mut tasks = read_tasks()?;
    for index in indexes {
        let task = match tasks.get_mut(*index - 1) {
            Some(task) => task,
            None => anyhow::bail!("not find task by index: {}", index),
        };
        let title = task.clone().title;
        print_success!("Task: {} is remove successfully", title);
        task.set_status(TaskStatus::Delete);
    }
    write_tasks(tasks)?;
    anyhow::Ok(true)
}
