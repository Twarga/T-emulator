pub mod adapters;
pub mod config;
pub mod scanner;
pub mod wave;

slint::include_modules!();

use slint::{ModelRc, SharedString, VecModel};

pub fn consoles() -> Vec<ConsoleInfo> {
    let c = |name: &str, full: &str, status: &str, kind: &str, ok: bool| ConsoleInfo {
        name: name.into(),
        full_name: full.into(),
        status: status.into(),
        kind: kind.into(),
        available: ok,
    };
    vec![
        c("PSP", "PlayStation Portable", "Ready", "psp", true),
        c("PS3", "PlayStation 3", "Coming soon", "ps3", false),
        c("GameCube", "Nintendo GameCube & Wii", "Coming soon", "gc", false),
        c("PS2", "PlayStation 2", "Coming soon", "ps2", false),
    ]
}

fn item(title: &str, sub: &str, icon: &str) -> XmbItem {
    XmbItem {
        title: title.into(),
        subtitle: sub.into(),
        art: Default::default(),
        has_art: false,
        icon: icon.into(),
    }
}

pub fn categories(games: &[scanner::Game], emulator_ready: bool) -> Vec<XmbCategory> {
    let game_items: Vec<XmbItem> = if games.is_empty() {
        vec![item("No games found", "Put .iso / .cso files in your PSP folder", "folder")]
    } else {
        games.iter().map(|g| item(&g.title, &format!("PSP  ·  {}", g.size), "disc")).collect()
    };
    let m = |v: Vec<XmbItem>| ModelRc::new(VecModel::from(v));
    vec![
        XmbCategory {
            name: "Settings".into(),
            icon: "gear".into(),
            items: m(vec![
                item("Games folder", "Where T-Emulator looks for PSP games", "folder"),
                item("Rescan games", "Look for new games now", "refresh"),
                item(
                    if emulator_ready { "Update PSP emulator" } else { "Install PSP emulator" },
                    "Downloads the latest PPSSPP from the official source",
                    "download",
                ),
            ]),
        },
        XmbCategory { name: "Games".into(), icon: "gamepad".into(), items: m(game_items) },
        XmbCategory {
            name: "Music".into(),
            icon: "music".into(),
            items: m(vec![item("Coming later", "Music is not part of v0.1", "lock")]),
        },
    ]
}

/// Local wall clock without pulling in a timezone crate.
pub fn clock_strings() -> (SharedString, SharedString) {
    use chrono::{Datelike, Local, Timelike};
    let n = Local::now();
    (
        format!("{:02}:{:02}", n.hour(), n.minute()).into(),
        format!("{}, {} {}", n.format("%A"), n.day(), n.format("%B")).into(),
    )
}

pub fn month() -> u32 {
    use chrono::{Datelike, Local};
    Local::now().month()
}
