use crate::{
    config::{self, CONFIG_FILE, TAM_DIR},
    utils::{config_dir, get_dir_in_config},
};

pub fn config() -> anyhow::Result<bool> {
    let config = config::load()?;
    let cf_dir = config_dir()?;
    let config_file = cf_dir.join(CONFIG_FILE);
    println!("Tam config file: {:?}", config_file);
    let tam_dir = get_dir_in_config(TAM_DIR)?;
    println!("Tam tasks file: {:?}", tam_dir.join(config.task_file));
    println!("Tam theme file: {:?}", tam_dir.join(config.theme_file));
    Ok(true)
}
