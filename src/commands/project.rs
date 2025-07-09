use crate::config::get_config;
use crate::model::Project;
use crate::{print_info, print_success};
use crate::services::project;


pub fn add(name: String) -> anyhow::Result<bool> {
    let project = project::add(name)?;
    print_success!("Project '{}' added successfully.", project.name);
    print_info!("Use `tam project use {}` to switch to this project.", project.name);
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
        },
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
    let mut config = get_config();

    if let Some(current) = config.current_project {
        // TODO: 查询是否有tasks关联，如果有任务关联，则不能删除
        
        
        if current == name {
            config.current_project = None;
            match config.save() {
                Ok(_) => {
                    print_success!("Current project '{}' removed successfully.", current);  
                },
                Err(e) => {
                    eprintln!("Failed to remove current project: {}", e);
                    std::process::exit(1);
                }
            }
        }
    }
    project::delete(name)?;
    Ok(true)
}


pub fn edit(name: String, new_name: String) -> anyhow::Result<bool> {
    if name == new_name {
        print_info!("Project name is the same, no changes made.");
        return Ok(true);
    }
    let config = get_config();
    if let Some(current) = &config.current_project {
        if current == &name {
            // 如果当前项目被编辑了，则需要更新配置文件中的current_project
            let mut new_config = config.clone();
            new_config.current_project = Some(new_name.clone());
            match new_config.save() {
                Ok(_) => {
                    print_success!("Current project updated to '{}'.", new_name);
                },
                Err(e) => {
                    eprintln!("Failed to update current project: {}", e);
                    std::process::exit(1);
                }
            }
        }
    }

    
    project::edit(name.clone(), new_name.clone())?;
    print_success!("Project '{}' updated to '{}' successfully.", name, new_name);
    Ok(true)
}