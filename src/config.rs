use anyhow::Context;
use env_home::env_home_dir;
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

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

/// Validate that the config file can be loaded without initializing the global
/// `Lazy` singleton (D5: `--json` startup errors must be machine readable).
pub fn preflight_load() -> anyhow::Result<()> {
    Config::load().map(|_| ())
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
        Self::default_for_dir(&tam_dir)
    }
}

impl Config {
    /// Build the default config for an explicit `.tam` directory.
    /// Pure helper: performs no filesystem access, so it is safe to unit-test.
    fn default_for_dir(tam_dir: &Path) -> Self {
        Config {
            db_file: tam_dir.join(DB_FILE).display().to_string(),
            current_project: None,
            logfile_path: tam_dir.join(LOG_FILE_PATH).display().to_string(),
            logfile_pattern: tam_dir.join(LOG_FILE_PATTERN).display().to_string(),
            theme: "dark".to_string(),
        }
    }

    /// Build the default config for an explicit home directory (test-only).
    #[cfg(test)]
    fn default_for(home: &Path) -> Self {
        Self::default_for_dir(&home.join(TAM_DIR))
    }

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

    fn unit_home() -> std::path::PathBuf {
        std::env::temp_dir().join("tam-unit-home")
    }

    #[test]
    fn test_default_config_paths() {
        let home = unit_home();
        let tam_dir = home.join(TAM_DIR);
        let config = Config::default_for(&home);
        assert_eq!(config.db_file, tam_dir.join(DB_FILE).display().to_string());
        assert_eq!(
            config.logfile_path,
            tam_dir.join(LOG_FILE_PATH).display().to_string()
        );
        assert_eq!(
            config.logfile_pattern,
            tam_dir.join(LOG_FILE_PATTERN).display().to_string()
        );
        assert_eq!(config.current_project, None);
        assert_eq!(config.theme, "dark");
    }

    #[test]
    fn test_config_toml_round_trip() {
        let config = Config::default_for(&unit_home());
        let text = toml::to_string(&config).expect("config should serialize to TOML");
        let parsed: Config = toml::from_str(&text).expect("serialized config should parse back");
        assert_eq!(parsed, config);
    }

    #[test]
    fn test_config_deserialize_explicit_paths() {
        let text = r#"
db_file = "/tmp/custom/data.db3"
current_project = "alpha"
logfile_path = "/tmp/custom/logs/tam.log"
logfile_pattern = "/tmp/custom/logs/tam.{}.log"
theme = "light"
"#;
        let parsed: Config = toml::from_str(text).expect("explicit TOML should parse");
        assert_eq!(parsed.db_file, "/tmp/custom/data.db3");
        assert_eq!(parsed.current_project.as_deref(), Some("alpha"));
        assert_eq!(parsed.logfile_path, "/tmp/custom/logs/tam.log");
        assert_eq!(parsed.logfile_pattern, "/tmp/custom/logs/tam.{}.log");
        assert_eq!(parsed.theme, "light");
    }
}
