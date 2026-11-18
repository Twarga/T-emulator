use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Serialize, Deserialize, Default)]
pub struct Config {
    pub games_dir: Option<PathBuf>,
}

fn dirs() -> Option<directories::ProjectDirs> {
    directories::ProjectDirs::from("com", "temulator", "t-emulator")
}

impl Config {
    fn path() -> PathBuf {
        dirs().map(|d| d.config_dir().join("config.toml")).unwrap_or_else(|| "config.toml".into())
    }
    pub fn load() -> Self {
        std::fs::read_to_string(Self::path()).ok().and_then(|s| toml::from_str(&s).ok()).unwrap_or_default()
    }
    pub fn save(&self) -> anyhow::Result<()> {
        let p = Self::path();
        if let Some(parent) = p.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(p, toml::to_string(self)?)?;
        Ok(())
    }
    pub fn emulators_dir() -> PathBuf {
        dirs().map(|d| d.data_dir().join("emulators")).unwrap_or_else(|| ".emulators".into())
    }
}
