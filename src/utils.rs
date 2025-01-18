use anyhow::Context;
use std::{fs, path::PathBuf};

#[macro_export]
macro_rules! print_success {
    ($msg:expr) => {
      use colored::Colorize;
      println!("{} {}", "SUCCESS".on_green().bold(), $msg);
    };
    ($fmt:expr, $($arg:tt)*) => {
      use colored::Colorize;
      println!("{} {}", "SUCCESS".on_green().bold(), format!($fmt, $($arg)*));
    };
}

#[macro_export]
macro_rules! print_error {
    ($msg:expr) => {
      use colored::Colorize;
      println!("{} {}", "ERROR".on_red().bold(), $msg);
    };
    ($fmt:expr, $($arg:tt)*) => {
      use colored::Colorize;
      println!("{} {}", "ERROR".on_red().bold(), format!($fmt, $($arg)*));
    };
}
pub fn config_dir() -> anyhow::Result<PathBuf> {
    let config_dir = dirs::config_dir().with_context(|| format!("fail to get config dir"))?;
    if !config_dir.exists() {
        fs::create_dir_all(&config_dir).with_context(|| format!("fail to create config dir"))?;
    }
    Ok(config_dir)
}
pub fn get_dir_in_config(dir: &str) -> anyhow::Result<PathBuf> {
    let dir_path = config_dir()?.join(dir);
    if !dir_path.exists() {
        fs::create_dir_all(&dir_path).with_context(|| format!("fail to create tam dir"))?;
    }
    Ok(dir_path)
}
