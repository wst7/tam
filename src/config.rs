use anyhow::Context;
use env_home::env_home_dir;
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

pub const TAM_DIR: &str = ".tam";
pub const CONFIG_FILE: &str = "tam.config.toml";

pub const LOG_FILE_PATH: &str = "logs/tam.log";
pub const LOG_FILE_PATTERN: &str = "logs/tam.{}.log";
pub const DB_FILE: &str = "tam.db3";

static CONFIG: Lazy<Config> = Lazy::new(|| {
    let config = match Config::load() {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Failed to load config: {}", e);
            std::process::exit(1);
        }
    };
    config
});

pub fn get_config() -> Config {
    CONFIG.clone()
}

#[derive(Debug, Deserialize, Serialize, Clone, PartialEq)]
pub struct Config {
    pub db_file: String,
    pub current_project: Option<String>,
    pub logfile_path: String,
    pub logfile_pattern: String,
    pub theme: String,
}

impl Default for Config {
    fn default() -> Self {
        let tam_dir = match Self::config_dir() {
            Ok(p) => p,
            Err(e) => {
                eprintln!("Failed to tam dir: {}", e);
                std::process::exit(1);
            }
        };
        let db_file = tam_dir.join(DB_FILE);
        let log_file = tam_dir.join(LOG_FILE_PATH);
        let log_file_pattern = tam_dir.join(LOG_FILE_PATTERN);

        Config {
            db_file: db_file.display().to_string(),
            current_project: None,
            logfile_path: log_file.display().to_string(),
            logfile_pattern: log_file_pattern.display().to_string(),
            theme: "dark".to_string(),
        }
    }
}

impl Config {
    fn load() -> anyhow::Result<Self> {
        let config_dir = Self::config_dir()?;
        let config_file = config_dir.join(CONFIG_FILE);
        if !config_file.exists() {
            log::info!("config file not exists, create default config");
            let config = Self::default();
            config.save()?;
            return Ok(config);
        }
        let content = std::fs::read_to_string(config_file)?;
        let config = toml::from_str(&content)?;
        Ok(config)
    }
    fn config_dir() -> anyhow::Result<PathBuf> {
        let home_dir = env_home_dir().with_context(|| format!("fail to get home dir"))?;
        let tam_dir = home_dir.join(TAM_DIR);
        if !tam_dir.exists() {
            fs::create_dir_all(&tam_dir).with_context(|| format!("fail to create tam dir"))?;
        }
        Ok(tam_dir)
    }
    pub fn save(&self) -> anyhow::Result<bool> {
        let config_dir = Self::config_dir()?;
        let config_file = config_dir.join(CONFIG_FILE);
        let content = toml::to_string(&self)?;
        std::fs::write(config_file, content)?;
        Ok(true)
    }
    pub fn set_theme(&mut self, theme: String) {
        self.theme = theme;
        match self.save() {
            Ok(_) => (),
            Err(err) => {
                log::info!("set theme error: {}", err);
            }
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_config_dir() {
        let result = Config::config_dir();
        assert!(result.is_ok());
        assert_eq!(
            result.unwrap().display().to_string(),
            "/Users/wstreet7/.tam".to_string()
        );
    }
    #[test]
    fn test_config_dir_error_path() {
        let result = Config::config_dir();
        assert!(result.is_ok());
        assert_ne!(
            result.unwrap().display().to_string(),
            "/Users/xxx/.tam".to_string()
        );
    }

    #[test]
    fn test_load() {
        let result = Config::load();
        assert!(result.is_ok());
        let mut c = result.unwrap();
        c.current_project = None;
        assert_eq!(c, Config::default());
    }

    #[test]
    fn test_edit_config() {
        let mut config = Config::default();
        config.current_project = Some(String::from("myproject"));
        let result = config.save();
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), true)
    }
    #[test]
    fn test_load_edit_config() {
        let result = Config::load();
        assert!(result.is_ok());

        let mut c = result.unwrap();
        c.current_project = Some(String::from("myproject2"));
        assert_ne!(c, Config::default());
    }
}
