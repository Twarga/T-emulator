//! PSP shell: XMB content and the real PPSSPP settings behind it.
//! Settings read/write ~/.config/ppsspp/PSP/SYSTEM/ppsspp.ini, which PPSSPP
//! owns — we patch single keys and leave the rest of the file alone.

use crate::ini::Ini;
use crate::scanner::Game;
use crate::{XmbCategory, XmbItem};
use slint::{ModelRc, VecModel};
use std::path::PathBuf;

/// Where the AppImage build keeps its config (PPSSPP uses XDG_CONFIG_HOME).
pub fn ini_path() -> PathBuf {
    let base = std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(|| directories::BaseDirs::new().map(|b| b.config_dir().to_path_buf()))
        .unwrap_or_else(|| ".config".into());
    base.join("ppsspp").join("PSP").join("SYSTEM").join("ppsspp.ini")
}

// ── settings schema ────────────────────────────────────────────────────────

pub struct Opt {
    pub label: &'static str,
    /// The literal written to the ini (or the flag-bits value for Flag kind).
    pub value: &'static str,
}

pub enum Kind {
    /// Write `key = value` into [section].
    Direct,
    /// Boolean: write True/False.
    Bool,
    /// A sub-field of the iShowStatusFlags bitmask: read the int, mask bits.
    Flag { mask: i64 },
    /// Owned by our own config.toml, not the emulator's ini.
    Launcher,
}

pub struct Setting {
    pub id: &'static str,
    pub label: &'static str,
    pub hint: &'static str,
    pub section: &'static str,
    pub key: &'static str,
    pub kind: Kind,
    pub opts: &'static [Opt],
}

const ON_OFF: &[Opt] = &[Opt { label: "Off", value: "False" }, Opt { label: "On", value: "True" }];

pub const SETTINGS: &[Setting] = &[
    Setting {
        id: "backend",
        label: "Graphics backend",
        hint: "Vulkan usually runs best on this machine",
        section: "Graphics",
        key: "GraphicsBackend",
        kind: Kind::Direct,
        opts: &[
            Opt { label: "OpenGL", value: "0 (OPENGL)" },
            Opt { label: "Vulkan", value: "3 (VULKAN)" },
        ],
    },
    Setting {
        id: "resolution",
        label: "Rendering resolution",
        hint: "Higher looks sharper, costs GPU",
        section: "Graphics",
        key: "InternalResolution",
        kind: Kind::Direct,
        opts: &[
            Opt { label: "Auto", value: "0" },
            Opt { label: "1× PSP", value: "1" },
            Opt { label: "2× PSP", value: "2" },
            Opt { label: "3× PSP", value: "3" },
            Opt { label: "4× PSP", value: "4" },
            Opt { label: "5× PSP", value: "5" },
        ],
    },
    Setting {
        id: "texfilter",
        label: "Texture filtering",
        hint: "Auto lets each game decide",
        section: "Graphics",
        key: "TextureFiltering",
        kind: Kind::Direct,
        opts: &[
            Opt { label: "Auto", value: "1" },
            Opt { label: "Nearest", value: "2" },
            Opt { label: "Linear", value: "3" },
            Opt { label: "Auto (max quality)", value: "4" },
        ],
    },
    Setting {
        id: "aniso",
        label: "Anisotropic filtering",
        hint: "Sharpens textures seen at an angle",
        section: "Graphics",
        key: "AnisotropyLevel",
        kind: Kind::Direct,
        opts: &[
            Opt { label: "Off", value: "0" },
            Opt { label: "2×", value: "1" },
            Opt { label: "4×", value: "2" },
            Opt { label: "8×", value: "3" },
            Opt { label: "16×", value: "4" },
        ],
    },
    Setting {
        id: "frameskip",
        label: "Frame skipping",
        hint: "Skips frames to keep speed on weak GPUs",
        section: "Graphics",
        key: "FrameSkip",
        kind: Kind::Direct,
        opts: &[
            Opt { label: "Off", value: "0" },
            Opt { label: "1", value: "1" },
            Opt { label: "2", value: "2" },
            Opt { label: "3", value: "3" },
            Opt { label: "4", value: "4" },
        ],
    },
    Setting {
        id: "autoframeskip",
        label: "Auto frame skip",
        hint: "Skips only when needed to hold full speed",
        section: "Graphics",
        key: "AutoFrameSkip",
        kind: Kind::Bool,
        opts: ON_OFF,
    },
    Setting {
        id: "fps",
        label: "On-screen display",
        hint: "FPS and speed counters while playing",
        section: "Graphics",
        key: "iShowStatusFlags",
        kind: Kind::Flag { mask: 0b110 },
        opts: &[
            Opt { label: "Off", value: "0" },
            Opt { label: "FPS", value: "2" },
            Opt { label: "Speed", value: "4" },
            Opt { label: "FPS + Speed", value: "6" },
        ],
    },
    Setting {
        id: "volume",
        label: "Game volume",
        hint: "PPSSPP output level, 0–100 %",
        section: "Sound",
        key: "GameVolume",
        kind: Kind::Direct,
        opts: &[
            Opt { label: "Mute", value: "0" },
            Opt { label: "25 %", value: "25" },
            Opt { label: "50 %", value: "50" },
            Opt { label: "75 %", value: "75" },
            Opt { label: "100 %", value: "100" },
        ],
    },
    Setting {
        id: "fullscreen",
        label: "Fullscreen",
        hint: "Start games fullscreen (applies at launch)",
        section: "",
        key: "",
        kind: Kind::Launcher,
        opts: ON_OFF,
    },
];

fn find(id: &str) -> Option<&'static Setting> {
    SETTINGS.iter().find(|s| s.id == id)
}

/// Current option index for a setting, reading ppsspp.ini (or launcher config
/// for Launcher-kind settings passed via `launcher_value`).
pub fn current(id: &str, ini: &Ini, launcher_value: Option<&str>) -> usize {
    let Some(s) = find(id) else { return 0 };
    let raw = match &s.kind {
        Kind::Launcher => launcher_value.map(str::to_string),
        Kind::Flag { mask } => ini.get(s.section, s.key).and_then(|v| {
            v.parse::<i64>().ok().map(|n| (n & mask).to_string())
        }),
        _ => ini.get(s.section, s.key),
    };
    raw.and_then(|v| s.opts.iter().position(|o| o.value == v))
        .unwrap_or_else(|| {
            // unmatched stored value: show it rather than lie
            usize::MAX
        })
}

/// Label for the value column.
pub fn value_label(id: &str, ini: &Ini, launcher_value: Option<&str>) -> String {
    let Some(s) = find(id) else { return String::new() };
    let i = current(id, ini, launcher_value);
    if i != usize::MAX {
        return s.opts[i].label.to_string();
    }
    match &s.kind {
        Kind::Launcher => launcher_value.unwrap_or("—").to_string(),
        _ => ini
            .get(s.section, s.key)
            .map(|v| format!("{v} (custom)"))
            .unwrap_or_else(|| "default".into()),
    }
}

/// Apply choice `opt` for setting `id`. Writes the ini unless Launcher kind.
/// Returns the ini so the caller can persist once per batch.
pub fn apply(id: &str, opt: usize, ini: &mut Ini) {
    let Some(s) = find(id) else { return };
    let Some(o) = s.opts.get(opt) else { return };
    match &s.kind {
        Kind::Launcher => {}
        Kind::Bool | Kind::Direct => ini.set(s.section, s.key, o.value),
        Kind::Flag { mask } => {
            let cur = ini
                .get(s.section, s.key)
                .and_then(|v| v.parse::<i64>().ok())
                .unwrap_or(0);
            let bits = o.value.parse::<i64>().unwrap_or(0);
            let next = (cur & !mask) | (bits & mask);
            ini.set(s.section, s.key, &next.to_string());
        }
    }
}

// ── XMB content ────────────────────────────────────────────────────────────

fn item(id: &str, title: &str, sub: &str, value: &str, icon: &str) -> XmbItem {
    XmbItem {
        id: id.into(),
        title: title.into(),
        subtitle: sub.into(),
        value: value.into(),
        art: Default::default(),
        has_art: false,
        icon: icon.into(),
    }
}

const SETTING_ICONS: &[(&str, &str)] = &[
    ("backend", "chip"),
    ("resolution", "monitor"),
    ("texfilter", "photo"),
    ("aniso", "photo"),
    ("frameskip", "gauge"),
    ("autoframeskip", "gauge"),
    ("fps", "gauge"),
    ("volume", "volume"),
    ("fullscreen", "expand"),
];

pub fn categories(
    games: &[Game],
    emu_installed: bool,
    emu_version: &str,
    games_dir: &str,
    ini: &Ini,
    fullscreen: bool,
) -> Vec<XmbCategory> {
    let m = |v: Vec<XmbItem>| ModelRc::new(VecModel::from(v));

    let settings: Vec<XmbItem> = SETTINGS
        .iter()
        .map(|s| {
            let lv = if matches!(s.kind, Kind::Launcher) {
                Some(if fullscreen { "True" } else { "False" })
            } else {
                None
            };
            let icon = SETTING_ICONS
                .iter()
                .find(|(id, _)| *id == s.id)
                .map(|(_, i)| *i)
                .unwrap_or("sliders");
            item(s.id, s.label, s.hint, &value_label(s.id, ini, lv), icon)
        })
        .collect();

    let game_items: Vec<XmbItem> = if games.is_empty() {
        vec![item(
            "none",
            "No games found",
            "Put .iso / .cso / .pbp files in your PSP folder",
            "",
            "folder",
        )]
    } else {
        games
            .iter()
            .enumerate()
            .map(|(i, g)| {
                let mut it = item(&format!("g{i}"), &g.title, &format!("PSP  ·  {}", g.size), "", "disc");
                if let Some(img) = crate::covers::path_for(&g.title)
                    .and_then(|p| slint::Image::load_from_path(&p).ok())
                {
                    it.art = img;
                    it.has_art = true;
                }
                it
            })
            .collect()
    };

    let system = vec![
        item(
            "emu",
            "PSP emulator",
            if emu_installed { "Installed — Enter to update" } else { "Not installed — Enter to install" },
            if emu_installed { emu_version } else { "" },
            "chip",
        ),
        item("folder", "Games folder", games_dir, "", "folder"),
        item("rescan", "Rescan games", "Look for new games now", "", "refresh"),
        item("exit", "Back to consoles", "Power off this system", "", "power"),
    ];

    vec![
        XmbCategory { name: "Settings".into(), icon: "sliders".into(), items: m(settings) },
        XmbCategory { name: "Games".into(), icon: "gamepad".into(), items: m(game_items) },
        XmbCategory { name: "System".into(), icon: "gear".into(), items: m(system) },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn flag_roundtrip_preserves_other_bits() {
        let mut ini = Ini::default();
        ini.set("Graphics", "iShowStatusFlags", "9"); // bit0 + bit3 (battery)
        apply("fps", 1, &mut ini); // FPS = bits 0b10
        assert_eq!(ini.get("Graphics", "iShowStatusFlags").as_deref(), Some("11"));
        apply("fps", 0, &mut ini);
        assert_eq!(ini.get("Graphics", "iShowStatusFlags").as_deref(), Some("9"));
    }

    #[test]
    fn backend_value_label_matches() {
        let mut ini = Ini::default();
        ini.set("Graphics", "GraphicsBackend", "3 (VULKAN)");
        assert_eq!(value_label("backend", &ini, None), "Vulkan");
        ini.set("Graphics", "GraphicsBackend", "1 (DIRECT3D11)");
        assert_eq!(value_label("backend", &ini, None), "1 (DIRECT3D11) (custom)");
    }
}
