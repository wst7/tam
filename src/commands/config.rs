use crate::config::get_config;

pub fn config() -> anyhow::Result<bool> {
    let c = get_config();

    println!("config {:#?}", c);
    Ok(true)
}
