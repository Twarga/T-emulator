use anyhow::Result;
use std::path::{Path, PathBuf};
use std::process::Child;

pub mod ppsspp;

pub trait EmulatorAdapter {
    fn is_installed(&self) -> bool;
    fn install(&self, progress: impl Fn(f32)) -> Result<()>;
    fn write_defaults(&self) -> Result<()>;
    fn launch(&self, game: &Path) -> Result<Child>;
    fn supports(&self, file: &Path) -> bool;
    fn binary_dir() -> PathBuf {
        directories::ProjectDirs::from("com", "temulator", "t-emulator")
            .map(|d| d.data_dir().join("emulators"))
            .unwrap_or_else(|| PathBuf::from(".emulators"))
    }
}
