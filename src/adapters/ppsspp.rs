use super::EmulatorAdapter;
use anyhow::{Context, Result};
use std::path::{Path, PathBuf};
use std::process::{Child, Command};

const RELEASE_API: &str = "https://api.github.com/repos/hrydgard/ppsspp/releases/latest";

pub struct Ppsspp { dir: PathBuf }

impl Ppsspp {
    pub fn new() -> Self {
        Self { dir: Self::binary_dir().join("ppsspp") }
    }
    fn binary(&self) -> PathBuf { self.dir.join("PPSSPP.AppImage") }

    fn download_url() -> Result<String> {
        let resp: serde_json::Value = ureq::get(RELEASE_API)
            .header("User-Agent", "t-emulator")
            .call()?
            .body_mut().read_json()?;
        resp["assets"].as_array()
            .and_then(|a| a.iter().find(|x| {
                x["name"].as_str().map(|n| n.contains("x86_64") && n.ends_with("AppImage")).unwrap_or(false)
            }))
            .and_then(|x| x["browser_download_url"].as_str().map(|s| s.to_string()))
            .context("x86_64 AppImage not found in release")
    }
}

impl EmulatorAdapter for Ppsspp {
    fn is_installed(&self) -> bool { self.binary().exists() }

    fn install(&self, progress: impl Fn(f32)) -> Result<()> {
        std::fs::create_dir_all(&self.dir)?;
        let url = Self::download_url()?;
        tracing::info!("downloading {url}");
        let mut resp = ureq::get(&url).header("User-Agent", "t-emulator").call()?;
        let total = resp.headers().get("Content-Length").and_then(|v| v.to_str().ok()).and_then(|s| s.parse::<u64>().ok()).unwrap_or(0);
        let mut buf = Vec::new();
        let mut reader = resp.body_mut().as_reader();
        let mut chunk = [0u8; 64 * 1024];
        let mut got = 0u64;
        loop {
            let n = std::io::Read::read(&mut reader, &mut chunk)?;
            if n == 0 { break; }
            buf.extend_from_slice(&chunk[..n]);
            got += n as u64;
            if total > 0 { progress(got as f32 / total as f32); }
        }
        let bin = self.binary();
        std::fs::write(&bin, &buf)?;
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&bin, std::fs::Permissions::from_mode(0o755))?;
        progress(1.0);
        Ok(())
    }

    fn write_defaults(&self) -> Result<()> {
        Ok(())  // patch after first launch when ini exists
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
