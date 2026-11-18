//! Self-update: check GitHub for a newer release, download it, swap the
//! binary and relaunch. Runs fully in the background — the player never sees
//! an installer.

use std::path::PathBuf;

use anyhow::{Context, Result};

const API: &str = "https://api.github.com/repos/Twarga/T-emulator/releases/latest";

#[cfg(windows)]
const ASSET_END: &str = "windows-x86_64.zip";
#[cfg(not(windows))]
const ASSET_END: &str = "linux-x86_64.tar.gz";

#[cfg(windows)]
const EXE_NAME: &str = "t-emulator-windows-x86_64.exe";
#[cfg(not(windows))]
const EXE_NAME: &str = "t-emulator-linux-x86_64";

fn agent() -> ureq::Agent {
    ureq::Agent::config_builder()
        .http_status_as_error(false)
        .timeout_global(Some(std::time::Duration::from_secs(120)))
        .build()
        .into()
}

/// "v0.1.10" > "v0.1.9" — tag compare, ignores non-numeric suffixes.
fn newer(a: &str, b: &str) -> bool {
    let v = |s: &str| -> Vec<u64> {
        s.trim_start_matches('v')
            .split(|c: char| !c.is_ascii_digit())
            .filter(|p| !p.is_empty())
            .filter_map(|p| p.parse().ok())
            .collect()
    };
    v(a) > v(b)
}

/// If a newer release exists, download its binary to a temp file and return
/// the path. `None` = up to date or check failed.
pub fn check(current: &str) -> Option<PathBuf> {
    let agent = agent();
    let resp = agent.get(API).header("User-Agent", "t-emulator").call().ok()?;
    if resp.status().as_u16() != 200 {
        return None;
    }
    let body: serde_json::Value = resp.into_body().read_json().ok()?;
    let tag = body["tag_name"].as_str()?.to_string();
    if !newer(&tag, current) {
        return None;
    }
    let url = body["assets"].as_array()?.iter().find_map(|a| {
        let n = a["name"].as_str()?;
        n.ends_with(ASSET_END)
            .then(|| a["browser_download_url"].as_str().map(str::to_string))?
    })?;
    download(&agent, &url).ok()
}

fn download(agent: &ureq::Agent, url: &str) -> Result<PathBuf> {
    let mut resp = agent.get(url).header("User-Agent", "t-emulator").call()?;
    if resp.status().as_u16() != 200 {
        anyhow::bail!("download: HTTP {}", resp.status());
    }
    let dir = std::env::temp_dir().join("t-emulator-update");
    std::fs::create_dir_all(&dir)?;
    let archive = dir.join(if cfg!(windows) { "u.zip" } else { "u.tar.gz" });
    std::io::copy(&mut resp.body_mut().as_reader(), &mut std::fs::File::create(&archive)?)?;
    unpack(&archive, &dir)
}

#[cfg(not(windows))]
fn unpack(archive: &std::path::Path, dir: &PathBuf) -> Result<PathBuf> {
    let out = std::process::Command::new("tar")
        .args(["-xzf"])
        .arg(archive)
        .arg("-C")
        .arg(dir)
        .output()?;
    let bin = dir.join(EXE_NAME);
    if out.status.success() && bin.exists() {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&bin, std::fs::Permissions::from_mode(0o755))?;
        return Ok(bin);
    }
    anyhow::bail!("archive unpack failed")
}

#[cfg(windows)]
fn unpack(archive: &std::path::Path, dir: &PathBuf) -> Result<PathBuf> {
    let mut z = zip::ZipArchive::new(std::fs::File::open(archive)?)?;
    for i in 0..z.len() {
        let mut e = z.by_index(i)?;
        let Some(name) = e.enclosed_name() else { continue };
        if name.file_name().is_some_and(|n| n == EXE_NAME) {
            let bin = dir.join(EXE_NAME);
            std::io::copy(&mut e, &mut std::fs::File::create(&bin)?)?;
            return Ok(bin);
        }
    }
    anyhow::bail!("no {EXE_NAME} in archive")
}

/// Swap in the downloaded binary and start it. Caller quits right after.
pub fn apply_and_restart(new_bin: &std::path::Path) -> Result<()> {
    let cur = std::env::current_exe().context("current_exe")?;
    #[cfg(not(windows))]
    {
        std::fs::rename(new_bin, &cur).context("replace binary")?;
        std::process::Command::new(&cur).spawn().context("relaunch")?;
    }
    #[cfg(windows)]
    {
        // Windows can't overwrite a running exe: park the old one, move in the
        // new one, relaunch. The .old file is removed on next startup.
        let old = cur.with_extension("old.exe");
        let _ = std::fs::remove_file(&old);
        std::fs::rename(&cur, &old).context("park old exe")?;
        std::fs::copy(new_bin, &cur).context("place new exe")?;
        std::process::Command::new(&cur).spawn().context("relaunch")?;
        // best-effort cleanup for next run
        let _ = std::fs::remove_file(&old);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn semver_order() {
        assert!(newer("v0.2.0", "v0.1.0"));
        assert!(newer("v0.1.1", "v0.1.0"));
        assert!(newer("v1.0.0", "v0.9.9"));
        assert!(!newer("v0.1.0", "v0.1.0"));
        assert!(!newer("v0.1.0", "v0.1.9"));
    }
}
