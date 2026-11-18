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

    fn agent() -> ureq::Agent {
        ureq::Agent::config_builder()
            .http_status_as_error(false)
            .timeout_global(Some(std::time::Duration::from_secs(300)))
            .build()
            .into()
    }

    fn download_url(agent: &ureq::Agent) -> Result<String> {
        let mut resp = agent.get(RELEASE_API).header("User-Agent", "t-emulator").call()?;
        let body: serde_json::Value = resp.body_mut().read_json()?;
        body["assets"].as_array()
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
        let agent = Self::agent();
        let url = Self::download_url(&agent)?;
        tracing::info!("downloading {url}");

        let mut resp = agent.get(&url).header("User-Agent", "t-emulator").call()?;
        if resp.status().as_u16() != 200 {
            anyhow::bail!("download failed: HTTP {}", resp.status());
        }
        let total = resp.headers()
            .get("Content-Length")
            .and_then(|v| v.to_str().ok())
            .and_then(|s| s.parse::<u64>().ok())
            .unwrap_or(0);

        let bin = self.binary();
        let mut out = std::io::BufWriter::new(std::fs::File::create(&bin)?);
        let mut reader = resp.body_mut().as_reader();
        let mut chunk = [0u8; 256 * 1024];
        let mut got = 0u64;
        loop {
            let n = std::io::Read::read(&mut reader, &mut chunk)?;
            if n == 0 { break; }
            std::io::Write::write_all(&mut out, &chunk[..n])?;
            got += n as u64;
            if total > 0 { progress(got as f32 / total as f32); }
        }
        std::io::Write::flush(&mut out)?;

        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&bin, std::fs::Permissions::from_mode(0o755))?;
        tracing::info!("ppsspp installed ({} bytes)", got);
        progress(1.0);
        Ok(())
    }

    fn write_defaults(&self) -> Result<()> {
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
