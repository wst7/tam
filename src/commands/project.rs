use crate::config::get_config;
use crate::model::Project;
use crate::services::project;
use crate::{print_info, print_success};

pub fn add(name: String) -> anyhow::Result<bool> {
    let (project, restored) = project::add(name)?;
    if restored {
        print_success!("Project '{}' restored successfully.", project.name);
    } else {
        print_success!("Project '{}' added successfully.", project.name);
    }
    print_info!(
        "Use `tam project use {}` to switch to this project.",
        project.name
    );
    Ok(true)
}

pub fn list() -> anyhow::Result<bool> {
    let projects = project::list()?;
    if projects.is_empty() {
        println!("No projects found. use `tam project add` to add a project.");
    } else {
        Project::print(&projects);
    }
    Ok(true)
}
pub fn use_project(name: String) -> anyhow::Result<bool> {
    let project = project::query_project(name)?;
    let mut config = get_config();
    config.current_project = Some(project.name.clone());
    match config.save() {
        Ok(_) => {
            print_success!("Project '{}' switched successfully.", project.name);
        }
        Err(e) => {
            eprintln!("Failed to change current project: {}", e);
            std::process::exit(1);
        }
    }
    Ok(true)
}

pub fn get_current_project() -> anyhow::Result<bool> {
    let project = project::get_current_project()?;
    print_info!("Current project: {}", project.name.green().to_string());
    Ok(true)
}

pub fn delete(name: String) -> anyhow::Result<bool> {
    let name = name.trim().to_string();
    if name.is_empty() {
        anyhow::bail!("project name cannot be empty");
    }

    // Database first: the guard (active tasks) and the soft delete must
    // succeed before the config is touched.
    project::delete(name.clone())?;

    // Keep the config in sync only when the deleted project is the current
    // one. The delete is not rolled back if the config cannot be saved:
    // report the recovery command instead.
    let mut config = get_config();
    if config.current_project.as_deref() == Some(name.as_str()) {
        config.current_project = None;
        if let Err(e) = config.save() {
            anyhow::bail!(
                "project '{}' was deleted, but clearing the current project from the config failed: {}; run `tam project use <existing project>` to reset the current project",
                name,
                e
            );
        }
    }

    print_success!("Project '{}' deleted successfully.", name);
    Ok(true)
}

pub fn edit(name: String, new_name: String) -> anyhow::Result<bool> {
    let name = name.trim().to_string();
    let new_name = new_name.trim().to_string();
    if name.is_empty() || new_name.is_empty() {
        anyhow::bail!("project name cannot be empty");
    }
    if name == new_name {
        print_info!("Project name is the same, no changes made.");
        return Ok(true);
    }

    // Database first: never move `current_project` before the rename succeeded.
    project::edit(name.clone(), new_name.clone())?;

    // Only after the rename is committed, keep the config in sync when the
    // edited project is the current one. The database change is not rolled
    // back if the config cannot be saved: report the recovery command instead.
    let mut config = get_config();
    if config.current_project.as_deref() == Some(name.as_str()) {
        config.current_project = Some(new_name.clone());
        if let Err(e) = config.save() {
            anyhow::bail!(
                "project '{}' was renamed to '{}' in the database, but updating the config failed: {}; run `tam project use {}` to update the current project",
                name,
                new_name,
                e,
                new_name
            );
        }
    }

    print_success!("Project '{}' updated to '{}' successfully.", name, new_name);
    Ok(true)
}
