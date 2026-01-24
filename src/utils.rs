use anyhow::anyhow;
use std::path::PathBuf;

pub type AppResult<T> = Result<T, anyhow::Error>;
pub type PlayerId = uuid::Uuid;

pub fn store_path(filename: &str) -> AppResult<PathBuf> {
    let dirs = directories::ProjectDirs::from("org", "frittura", "asterion")
        .ok_or(anyhow!("Failed to get directories"))?;
    let config_dirs = dirs.config_dir();
    if !config_dirs.exists() {
        std::fs::create_dir_all(config_dirs)?;
    }
    let path = config_dirs.join(filename);
    Ok(path)
}
