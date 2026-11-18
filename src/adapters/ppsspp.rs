use anyhow::{Context, Result};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, Command};

const RELEASE_API: &str = "https://api.github.com/repos/hrydgard/ppsspp/releases/latest";

pub struct Ppsspp {
    dir: PathBuf,
}

impl Ppsspp {
    pub fn new() -> Self {
        Self { dir: crate::config::Config::emulators_dir().join("ppsspp") }
    }

    pub fn binary(&self) -> PathBuf {
        self.dir.join("PPSSPP.AppImage")
    }

    pub fn is_installed(&self) -> bool {
        self.binary().exists()
    }

    fn agent() -> ureq::Agent {
        ureq::Agent::config_builder()
            .http_status_as_error(false)
            .timeout_global(Some(std::time::Duration::from_secs(600)))
            .build()
            .into()
    }

    fn download_url(agent: &ureq::Agent) -> Result<String> {
        let mut resp = agent.get(RELEASE_API).header("User-Agent", "t-emulator").call()?;
        let body: serde_json::Value = resp.body_mut().read_json()?;
        body["assets"]
            .as_array()
            .and_then(|a| {
                a.iter().find(|x| {
                    x["name"].as_str().map(|n| n.contains("x86_64") && n.ends_with("AppImage")).unwrap_or(false)
                })
            })
            .and_then(|x| x["browser_download_url"].as_str().map(str::to_string))
            .context("no x86_64 AppImage in latest release")
    }

    /// Downloads to a temp file and renames on success, so a half-finished
    /// download is never mistaken for an installed emulator.
    pub fn install(&self, progress: impl Fn(f32)) -> Result<()> {
        std::fs::create_dir_all(&self.dir)?;
        let agent = Self::agent();
        let url = Self::download_url(&agent)?;
        let mut resp = agent.get(&url).header("User-Agent", "t-emulator").call()?;
        if resp.status().as_u16() != 200 {
            anyhow::bail!("download failed: HTTP {}", resp.status());
        }
        let total = resp
            .headers()
            .get("Content-Length")
            .and_then(|v| v.to_str().ok())
            .and_then(|s| s.parse::<u64>().ok())
            .unwrap_or(0);

        let tmp = self.dir.join("PPSSPP.AppImage.part");
        let mut out = std::io::BufWriter::new(std::fs::File::create(&tmp)?);
        let mut reader = resp.body_mut().as_reader();
        let mut chunk = vec![0u8; 256 * 1024];
        let mut got = 0u64;
        loop {
            let n = reader.read(&mut chunk)?;
            if n == 0 {
                break;
            }
            out.write_all(&chunk[..n])?;
            got += n as u64;
            if total > 0 {
                progress(got as f32 / total as f32);
            }
        }
        out.flush()?;
        drop(out);

        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&tmp, std::fs::Permissions::from_mode(0o755))?;
        std::fs::rename(&tmp, self.binary())?;
        Ok(())
    }

    pub fn launch(&self, game: &Path) -> Result<Child> {
        Command::new(self.binary()).arg("--fullscreen").arg(game).spawn().context("starting PPSSPP")
    }
}
