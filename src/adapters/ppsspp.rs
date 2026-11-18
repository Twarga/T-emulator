use super::EmulatorAdapter;
use anyhow::{Context, Result};
use std::path::{Path, PathBuf};
use std::process::{Child, Command};

pub struct Ppsspp { dir: PathBuf }

impl Ppsspp {
    pub fn new() -> Self {
        Self { dir: Self::binary_dir().join("ppsspp") }
    }
    fn binary(&self) -> PathBuf { self.dir.join("PPSSPPSDL") }
}

impl EmulatorAdapter for Ppsspp {
    fn is_installed(&self) -> bool { self.binary().exists() }

    fn install(&self, progress: impl Fn(f32)) -> Result<()> {
        std::fs::create_dir_all(&self.dir)?;
        // TODO: real release fetch — Phase 3 task; for now, expect user drop-in
        progress(1.0);
        Ok(())
    }

    fn write_defaults(&self) -> Result<()> {
        // ppsspp.ini is patched after first launch, not authored from scratch
        Ok(())
    }

    fn launch(&self, game: &Path) -> Result<Child> {
        Command::new(self.binary())
            .arg("--fullscreen")
            .arg(game)
            .spawn()
            .context("launching PPSSPP")
    }

    fn supports(&self, file: &Path) -> bool {
        matches!(
            file.extension().and_then(|e| e.to_str()).map(|e| e.to_lowercase()).as_deref(),
            Some("iso") | Some("cso") | Some("pbp")
        )
    }
}
