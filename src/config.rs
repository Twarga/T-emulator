use serde::{Deserialize, Serialize};
use std::path::PathBuf;

fn def_true() -> bool {
    true
}

#[derive(Serialize, Deserialize)]
pub struct Config {
    /// PSP games folder. `None` until first run, then set to ~/Games/PSP.
    pub games_dir: Option<PathBuf>,
    /// Interface sounds on/off.
    #[serde(default = "def_true")]
    pub sounds: bool,
    /// Pass --fullscreen to emulators when launching a game.
    #[serde(default = "def_true")]
    pub fullscreen: bool,
    /// Check for emulator updates when the launcher opens.
    #[serde(default = "def_true")]
    pub update_check: bool,
    /// Recently played games, newest first (path, unix timestamp).
    #[serde(default)]
    pub recents: Vec<(PathBuf, u64)>,
    /// Emulator version tags recorded at install time, e.g. "v1.20.4".
    #[serde(default)]
    pub emu_versions: std::collections::BTreeMap<String, String>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            games_dir: None,
            sounds: true,
            fullscreen: true,
            update_check: true,
            recents: Vec::new(),
            emu_versions: Default::default(),
        }
    }
}

fn dirs() -> Option<directories::ProjectDirs> {
    directories::ProjectDirs::from("com", "temulator", "t-emulator")
}

impl Config {
    fn path() -> PathBuf {
        dirs()
            .map(|d| d.config_dir().join("config.toml"))
            .unwrap_or_else(|| "config.toml".into())
    }
    pub fn load() -> Self {
        std::fs::read_to_string(Self::path())
            .ok()
            .and_then(|s| toml::from_str(&s).ok())
            .unwrap_or_default()
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
        dirs()
            .map(|d| d.data_dir().join("emulators"))
            .unwrap_or_else(|| ".emulators".into())
    }

    /// Records a play event; returns nothing. Keeps at most 24 entries.
    pub fn played(&mut self, path: PathBuf) {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        self.recents.retain(|(p, _)| p != &path);
        self.recents.insert(0, (path, now));
        self.recents.truncate(24);
    }

    pub fn ago(ts: u64) -> String {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(ts);
        let s = now.saturating_sub(ts);
        if s < 3_600 {
            "just now".into()
        } else if s < 86_400 {
            format!("{} h ago", s / 3_600)
        } else if s < 86_400 * 30 {
            format!("{} d ago", s / 86_400)
        } else {
            "a while ago".into()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recents_dedupes_and_orders() {
        let mut c = Config::default();
        c.played("/a.iso".into());
        c.played("/b.iso".into());
        c.played("/a.iso".into());
        assert_eq!(c.recents.len(), 2);
        assert_eq!(c.recents[0].0, PathBuf::from("/a.iso"));
        assert_eq!(c.recents[1].0, PathBuf::from("/b.iso"));
    }
}
