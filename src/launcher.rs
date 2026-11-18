//! Data builders for the launcher screens. Pure functions over app state —
//! the Slint side only renders what these return.

use crate::config::Config;
use crate::scanner::Game;
use crate::{ConsoleInfo, EmuRow, GameTile, RecentCard, SetRow};

/// Ember-adjacent cover tones (indexes into Lt.tones in the UI).
pub const TONES: usize = 6;

fn tone(title: &str) -> i32 {
    let h: u32 = title.bytes().fold(5381, |h, b| h.wrapping_mul(33) ^ b as u32);
    (h % TONES as u32) as i32
}

pub fn console_tiles(ngames: usize, installed: bool, installing: bool) -> Vec<ConsoleInfo> {
    let c = |name: &str, full: &str, status: String, kind: &str, ok: bool| ConsoleInfo {
        name: name.into(),
        full_name: full.into(),
        status: status.into(),
        kind: kind.into(),
        available: ok,
    };
    let psp_status = if installing {
        "Setting up…".to_string()
    } else if !installed {
        "Set up on first launch".to_string()
    } else {
        format!("Ready · {ngames} games")
    };
    vec![
        c("PSP", "PlayStation Portable", psp_status, "psp", true),
        c("PS3", "PlayStation 3", "Coming later".into(), "ps3", false),
        c("GameCube", "GameCube & Wii".into(), "Coming later".into(), "gc", false),
        c("PS2", "PlayStation 2".into(), "Coming later".into(), "ps2", false),
    ]
}

pub fn game_tiles(games: &[Game], query: &str) -> Vec<GameTile> {
    let q = query.trim().to_lowercase();
    games
        .iter()
        .enumerate()
        .filter(|(_, g)| q.is_empty() || g.title.to_lowercase().contains(&q))
        .map(|(i, g)| GameTile {
            idx: i as i32,
            title: g.title.clone().into(),
            console: "PSP".into(),
            sub: g.size.clone().into(),
            tone: tone(&g.title),
        })
        .collect()
}

/// Most recent plays that still resolve to a scanned game, newest first.
/// Returns (tile, real game index) pairs capped at 3.
pub fn recent_cards(cfg: &Config, games: &[Game]) -> Vec<RecentCard> {
    let mut out = Vec::new();
    for (path, ts) in &cfg.recents {
        if out.len() >= 3 {
            break;
        }
        if let Some((i, g)) = games.iter().enumerate().find(|(_, g)| &g.path == path) {
            out.push(RecentCard {
                idx: i as i32,
                title: g.title.clone().into(),
                ago: Config::ago(*ts).into(),
                tone: tone(&g.title),
            });
        }
    }
    out
}

#[derive(Clone, Copy, PartialEq)]
pub enum EmuState {
    Install,
    Update,
    Ready,
    Busy,
    Planned,
}

pub struct EmuInfo {
    pub installed: bool,
    pub installing: bool,
    pub progress: f32,
    pub version: Option<String>,
    pub latest: Option<String>,
    pub check_done: bool,
}

impl EmuInfo {
    pub fn state(&self) -> EmuState {
        if self.installing {
            EmuState::Busy
        } else if !self.installed {
            EmuState::Install
        } else if let (Some(v), Some(l)) = (&self.version, &self.latest) {
            if v != l { EmuState::Update } else { EmuState::Ready }
        } else {
            EmuState::Ready
        }
    }
}

pub fn emu_rows(emu: &EmuInfo) -> Vec<EmuRow> {
    let r = |name: &str, detail: String, state: EmuState, action: &str, progress: f32| EmuRow {
        name: name.into(),
        detail: detail.into(),
        state: state as i32,
        action: action.into(),
        progress,
    };
    let (detail, action) = match emu.state() {
        EmuState::Install => ("Not installed".into(), "Install"),
        EmuState::Update => (
            format!(
                "{} → {} available",
                emu.version.as_deref().unwrap_or("installed"),
                emu.latest.as_deref().unwrap_or("")
            ),
            "Update",
        ),
        EmuState::Ready => {
            let v = emu.version.as_deref().unwrap_or("installed");
            let tail = if emu.latest.is_none() && emu.check_done {
                ""
            } else if emu.latest.is_none() {
                " · checking…"
            } else {
                " · up to date"
            };
            (format!("{v}{tail}"), "Installed")
        }
        EmuState::Busy => ("Downloading PPSSPP…".into(), ""),
        EmuState::Planned => unreachable!(),
    };
    vec![
        r("PPSSPP — PSP", detail, emu.state(), action, emu.progress),
        r("RPCS3 — PS3", "Coming later".into(), EmuState::Planned, "", 0.0),
        r("Dolphin — GameCube & Wii", "Coming later".into(), EmuState::Planned, "", 0.0),
        r("PCSX2 — PS2", "Coming later".into(), EmuState::Planned, "", 0.0),
    ]
}

pub fn settings_rows(cfg: &Config, dir: &str) -> Vec<SetRow> {
    let r = |id: &str, label: &str, hint: &str, kind: &str, value: &str, on: bool| SetRow {
        id: id.into(),
        label: label.into(),
        hint: hint.into(),
        kind: kind.into(),
        value: value.into(),
        on,
    };
    vec![
        r("folder", "Games folder", "Choose where T-Emulator looks for PSP games", "value", dir, false),
        r("rescan", "Rescan library", "Look for new games now", "action", "", false),
        r("sounds", "Interface sounds", "Menu ticks and confirms", "toggle", "", cfg.sounds),
        r("fullscreen", "Start games fullscreen", "Passed to the emulator at launch", "toggle", "", cfg.fullscreen),
        r("update-check", "Check for emulator updates", "When the launcher opens", "toggle", "", cfg.update_check),
        r("about", "Version", "T-Emulator", "info", env!("CARGO_PKG_VERSION"), false),
    ]

}
