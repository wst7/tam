use std::path::PathBuf;
use std::io::Write;

use crate::utils::{config_dir, get_dir_in_config};
use anyhow::{bail, Context};
use serde::{Deserialize, Serialize};

const CONFIG_FILE: &str = "tam.config.toml";
const TAM_DIR: &str = "tam";
const DEFAULT_TASKS_FILE: &str = "tasks.json";
const DEFAULT_TAM_THEME: &str = "theme.toml";

#[derive(Debug, Deserialize, Serialize)]
pub struct Config {
    task_file: String,
    theme_file: String,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            task_file: DEFAULT_TASKS_FILE.to_string(),
            theme_file: DEFAULT_TAM_THEME.to_string(),
        }
    }
}

impl Config {
    fn init() -> anyhow::Result<bool> {
        match Self::load() {
            Ok(_) => Ok(true),
            Err(_) => {
                let config = Config::default();
                config.save()?;
                Ok(true)
            }
        }
    }
    pub fn load_or_default() -> anyhow::Result<Self> {
        match Self::load() {
            Ok(config) => Ok(config),
            Err(_) => {
                let config = Config::default();
                config.save()?;
                Ok(config)
            }
        }
    }
    fn load() -> anyhow::Result<Self> {
        let config_dir = config_dir()?;
        let config_file = config_dir.join(CONFIG_FILE);
        if !config_file.exists() {
            bail!("config file not exists");
        }
        let content = std::fs::read_to_string(config_file)?;
        let config = toml::from_str(&content)?;
        Ok(config)
    }
    fn save(&self) -> anyhow::Result<bool> {
        let config_dir = config_dir()?;
        let config_file = config_dir.join(CONFIG_FILE);
        let content = toml::to_string(&self)?;
        std::fs::write(config_file, content)?;
        Ok(true)
    }
}

pub fn init() -> anyhow::Result<bool> {
    Config::init().with_context(|| "fail to init config")
}

pub fn load() -> anyhow::Result<Config> {
    Config::load_or_default().with_context(|| "fail to load config")
}

pub fn get_tasks_file() -> anyhow::Result<PathBuf> {
    let config = Config::load_or_default()?;
    let tam_dir = get_dir_in_config(TAM_DIR)?;
    let tasks_file = tam_dir.join(config.task_file);
    if !tasks_file.exists() {
        let mut file = std::fs::File::create(&tasks_file)?;
        file.write_all(b"[]")?;

    }
    Ok(tasks_file)
}

pub fn get_theme_file() -> anyhow::Result<PathBuf> {
    let config = Config::load_or_default()?;
    let tam_dir = get_dir_in_config(TAM_DIR)?;
    let theme_file = tam_dir.join(config.theme_file);
    if!theme_file.exists() {
        std::fs::File::create(&theme_file)?;
    }
    Ok(theme_file)
}