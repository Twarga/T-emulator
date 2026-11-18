use anyhow::{Context, Result};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, Command};

const RELEASE_API: &str = "https://api.github.com/repos/hrydgard/ppsspp/releases/latest";

/// Latest release info: (tag_name, appimage url, size bytes).
pub struct Release {
    pub tag: String,
    pub url: String,
}

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

    pub fn latest_release(agent: &ureq::Agent) -> Result<Release> {
        let mut resp = agent.get(RELEASE_API).header("User-Agent", "t-emulator").call()?;
        if resp.status().as_u16() != 200 {
            anyhow::bail!("release API: HTTP {}", resp.status());
        }
        let body: serde_json::Value = resp.body_mut().read_json()?;
        let url = body["assets"]
            .as_array()
            .and_then(|a| {
                a.iter().find(|x| {
                    x["name"].as_str().map(|n| n.contains("x86_64") && n.ends_with("AppImage")).unwrap_or(false)
                })
            })
            .and_then(|x| x["browser_download_url"].as_str().map(str::to_string))
            .context("no x86_64 AppImage in latest release")?;
        let tag = body["tag_name"].as_str().unwrap_or("").to_string();
        Ok(Release { tag, url })
    }

    /// Version of the installed binary, e.g. "v1.20.4". Slow (AppImage cold
    /// start + its own net check) — only call off the UI thread.
    pub fn probe_version(&self) -> Option<String> {
        let out = Command::new(self.binary()).arg("--version").output().ok()?;
        let text = String::from_utf8_lossy(&out.stdout);
        text.split_whitespace()
            .find(|w| w.starts_with('v') && w[1..].chars().next().is_some_and(|c| c.is_ascii_digit()))
            .map(str::to_string)
    }

    /// Downloads to a temp file and renames on success, so a half-finished
    /// download is never mistaken for an installed emulator.
    /// Returns the release tag installed.
    pub fn install(&self, progress: impl Fn(f32)) -> Result<String> {
        std::fs::create_dir_all(&self.dir)?;
        let agent = Self::agent();
        let rel = Self::latest_release(&agent)?;
        let mut resp = agent.get(&rel.url).header("User-Agent", "t-emulator").call()?;
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
        Ok(rel.tag)
    }

    pub fn launch(&self, game: &Path, fullscreen: bool) -> Result<Child> {
        let mut cmd = Command::new(self.binary());
        if fullscreen {
            cmd.arg("--fullscreen");
        }
        cmd.arg(game).spawn().context("starting PPSSPP")
    }
}
