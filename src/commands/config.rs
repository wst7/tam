use crate::{config, utils::config_dir};



pub fn config() -> anyhow::Result<bool> {
  let config = config::load()?;
  println!("Config dir: {:?}", config_dir()?);
  println!("{:#?}", config);
  Ok(true)
}