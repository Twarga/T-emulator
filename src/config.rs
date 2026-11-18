use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Serialize, Deserialize, Default)]
pub struct Config {
    pub games_dir: Option<PathBuf>,
    pub last_console: Option<String>,
    pub animations: bool,
}

impl Config {
    pub fn path() -> PathBuf {
        directories::ProjectDirs::from("com", "temulator", "t-emulator")
            .map(|d| d.config_dir().join("config.toml"))
            .unwrap_or_else(|| PathBuf::from("config.toml"))
    }
    pub fn load() -> Self {
        let p = Self::path();
        std::fs::read_to_string(&p)
            .ok()
            .and_then(|s| toml::from_str(&s).ok())
            .unwrap_or_default()
    }
    pub fn save(&self) -> anyhow::Result<()> {
        let p = Self::path();
        if let Some(parent) = p.parent() { std::fs::create_dir_all(parent)?; }
        std::fs::write(&p, toml::to_string(self)?)?;
        Ok(())
    }
}
