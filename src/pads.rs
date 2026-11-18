//! Gamepad auto-configuration: make a plugged-in pad work in every emulator
//! we ship, without the user opening PPSSPP's control settings.
//!
//! Two layers:
//! - `controls.ini` — PPSSPP's own map. A cheap pad shows up there as device
//!   `10-*` (raw buttons 188..208 = BUTTON_1..21, dpad 4000..4003, analog
//!   4024..4027). PPSSPP auto-writes this on first pad detection; we pre-fill
//!   the same bindings so it works from the first launch, and merge (never
//!   overwrite) if the user already customized.
//! - `gamecontrollerdb.txt` — SDL's pad db. Emulators that take the SDL path
//!   (and future cores) get it via SDL_GAMECONTROLLERCONFIG_FILE.

use std::path::{Path, PathBuf};

use crate::config::Config;
use crate::ini::Ini;

const DB: &str = include_str!("../assets/gamecontrollerdb.txt");

fn controls_path() -> PathBuf {
    // PPSSPP AppImage keeps config in ~/.config/ppsspp
    let home = std::env::var("HOME").unwrap_or_default();
    Path::new(&home).join(".config/ppsspp/PSP/SYSTEM/controls.ini")
}

/// Generic-pad bindings: PSP control -> extra `device-key` codes appended to
/// whatever is already there. Mirrors the auto-map PPSSPP writes for generic
/// pads, so behavior matches what PPSSPP itself would do.
const PAD_BINDS: &[(&str, &str)] = &[
    ("Up", "10-4001"),
    ("Down", "10-4000"),
    ("Left", "10-4003"),
    ("Right", "10-4002"),
    ("Circle", "10-190"),
    ("Cross", "10-189"),
    ("Square", "10-191"),
    ("Triangle", "10-188"),
    ("Start", "10-197"),
    ("Select", "10-196"),
    ("L", "10-194"),
    ("R", "10-195"),
    ("An.Up", "10-4025"),
    ("An.Down", "10-4024"),
    ("An.Left", "10-4027"),
    ("An.Right", "10-4026"),
    ("Pause", "10-199"),
];

/// Append `code` to a `key = a,b,c` list if absent.
fn merge_value(cur: &str, code: &str) -> String {
    if cur.split(',').any(|p| p.trim() == code) {
        return cur.to_string();
    }
    if cur.trim().is_empty() {
        code.to_string()
    } else {
        format!("{cur},{code}")
    }
}

/// Ensure every pad binding exists in controls.ini. Returns true if changed.
pub fn ensure_controls(path: &Path) -> bool {
    let mut ini = Ini::load(path).unwrap_or_default();
    let mut changed = false;
    for (key, code) in PAD_BINDS {
        let cur = ini.get("ControlMapping", key).unwrap_or_default();
        let merged = merge_value(&cur, code);
        if merged != cur {
            ini.set("ControlMapping", key, &merged);
            changed = true;
        }
    }
    if changed {
        if let Some(dir) = path.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        let _ = ini.save(path);
    }
    changed
}

/// Write the SDL controller db next to our data dir if stale/missing.
pub fn ensure_sdl_db() -> PathBuf {
    let p = Config::emulators_dir()
        .parent()
        .map(|d| d.join("gamecontrollerdb.txt"))
        .unwrap_or_else(|| "gamecontrollerdb.txt".into());
    let stale = std::fs::read_to_string(&p).map(|s| s != DB).unwrap_or(true);
    if stale {
        if let Some(dir) = p.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        let _ = std::fs::write(&p, DB);
    }
    p
}

/// Called on pad connect and at startup: make all emulators pad-ready.
/// Returns a human summary of what was configured.
pub fn ensure_all(_pad_name: &str) -> Vec<String> {
    let mut did = Vec::new();
    if ensure_controls(&controls_path()) {
        did.push("PSP (PPSSPP) bindings written".to_string());
    }
    ensure_sdl_db();
    if did.is_empty() {
        did.push("PSP (PPSSPP) already configured".to_string());
    }
    did
}

/// Env vars a launched emulator should inherit so SDL pads resolve.
pub fn sdl_db_env() -> Option<(&'static str, PathBuf)> {
    let p = ensure_sdl_db();
    p.exists().then_some(("SDL_GAMECONTROLLERCONFIG_FILE", p))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn merge_is_append_only_and_idempotent() {
        assert_eq!(merge_value("1-19,10-19", "10-4001"), "1-19,10-19,10-4001");
        assert_eq!(merge_value("1-19,10-4001", "10-4001"), "1-19,10-4001");
        assert_eq!(merge_value("", "10-4001"), "10-4001");
    }

    #[test]
    fn ensure_controls_fills_missing_without_touching_user_binds() {
        let dir = std::env::temp_dir().join(format!("pads-{}", std::process::id()));
        let p = dir.join("controls.ini");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(&p, "[ControlMapping]\nCross = 1-54,9-100\n").unwrap();
        assert!(ensure_controls(&p));
        let txt = std::fs::read_to_string(&p).unwrap();
        assert!(txt.contains("Cross = 1-54,9-100,10-189"), "{txt}");
        assert!(txt.contains("Up = 10-4001"), "{txt}");
        // second run is a no-op
        assert!(!ensure_controls(&p));
        let _ = std::fs::remove_dir_all(&dir);
    }
}
