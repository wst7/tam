use crate::{
    model::Task,
    print_info, print_success,
    services::{project, task},
};

pub fn add(title: String) -> anyhow::Result<bool> {
    task::add(title.clone(), None)?;
    print_success!("Task '{}' added successfully.", title);
    Ok(true)
}

pub fn list() -> anyhow::Result<bool> {
    let project = project::get_current_project()?;
    if project.id.is_none() {
        anyhow::bail!("No current project set. Use `tam project use <name>` to set a project.");
    }
    let project_id = project.id.unwrap();
    let tasks = task::list(project_id)?;
    if tasks.is_empty() {
        print_info!("No tasks found. Use `tam task add` to add a task.");
    } else {
        print_info!("Current project: {}", project.name);
        Task::print(&tasks);
    }
    Ok(true)
}

pub fn delete(id: String) -> anyhow::Result<bool> {
    task::delete(id.clone())?;
    print_success!("Task '{}' deleted successfully.", id);
    Ok(true)
}
pub fn start(id: String) -> anyhow::Result<bool> {
    task::start(id.clone())?;
    print_success!("Task '{}' started successfully.", id);
    Ok(true)
}
pub fn done(id: String) -> anyhow::Result<bool> {
    task::done(id.clone())?;
    print_success!("Task '{}' marked as done successfully.", id);
    Ok(true)
}
pub fn edit(id: String, title: String) -> anyhow::Result<bool> {
    task::edit(id.clone(), title.clone())?;
    print_success!("Task '{}' updated successfully.", id);
    Ok(true)
}
