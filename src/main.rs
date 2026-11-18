mod config;
mod adapters;
mod scanner;
mod input;
mod sounds;

use anyhow::Result;
use slint::{ModelRc, VecModel, SharedString, Color};
use adapters::EmulatorAdapter;
use std::path::PathBuf;

slint::include_modules!();

fn home() -> PathBuf {
    directories::BaseDirs::new().map(|d| d.home_dir().to_path_buf())
        .unwrap_or_else(|| PathBuf::from("."))
}

fn ensure_defaults(cfg: &mut config::Config) {
    if cfg.games_dir.is_none() {
        let d = home().join("Games/PSP");
        let _ = std::fs::create_dir_all(&d);
        cfg.games_dir = Some(d);
        let _ = cfg.save();
    }
}

fn load_games(dir: &std::path::Path) -> Vec<GameModel> {
    scanner::psp::scan(dir).iter().map(|g| GameModel {
        title: SharedString::from(g.title.clone()),
        path: SharedString::from(g.path.to_string_lossy().to_string()),
        size: SharedString::from(g.size.clone()),
        last_played: SharedString::from("—"),
    }).collect()
}

fn main() -> Result<()> {
    tracing_subscriber::fmt::init();
    let mut cfg = config::Config::load();
    ensure_defaults(&mut cfg);

    let app = App::new()?;

    let consoles = vec![
        ConsoleEntry { title: SharedString::from("PSP"), subtitle: SharedString::from("PlayStation Portable"), enabled: true, accent: Color::from_rgb_u8(0x4e, 0xa8, 0xff) },
        ConsoleEntry { title: SharedString::from("PS3"), subtitle: SharedString::from("PlayStation 3 — soon"), enabled: false, accent: Color::from_rgb_u8(0x4e, 0xa8, 0xff) },
        ConsoleEntry { title: SharedString::from("GC / Wii"), subtitle: SharedString::from("Nintendo — soon"), enabled: false, accent: Color::from_rgb_u8(0x6a, 0x4f, 0xff) },
        ConsoleEntry { title: SharedString::from("PS2"), subtitle: SharedString::from("PlayStation 2 — soon"), enabled: false, accent: Color::from_rgb_u8(0x4e, 0xa8, 0xff) },
    ];
    app.set_consoles(ModelRc::new(VecModel::from(consoles)));

    let games_dir = cfg.games_dir.clone().unwrap();
    app.set_games(ModelRc::new(VecModel::from(load_games(&games_dir))));

    // power-on → ensure PPSSPP installed (download if missing), then enter skin
    let weak = app.as_weak();
    let games_dir_c = games_dir.clone();
    app.on_power_on(move |name| {
        sounds::select();
        tracing::info!("power_on {name}");
        if name != "PSP" { return; }
        let adapter = adapters::ppsspp::Ppsspp::new();
        if !adapter.is_installed() {
            let weak2 = weak.clone();
            let gdir = games_dir_c.clone();
            std::thread::spawn(move || {
                if let Err(e) = adapter.install(|_| {}) {
                    tracing::error!("ppsspp install failed: {e}");
                } else {
                    tracing::info!("ppsspp installed");
                    let _ = weak2.upgrade_in_event_loop(move |app| {
                        app.set_games(ModelRc::new(VecModel::from(load_games(&gdir))));
                    });
                }
            });
        }
    });

    // launch game → PPSSPP → on exit, XMB is still there
    app.on_play_game(move |path| {
        let path = PathBuf::from(path.as_str());
        let adapter = adapters::ppsspp::Ppsspp::new();
        match adapter.launch(&path) {
            Ok(mut child) => {
                sounds::select();
                tracing::info!("launched {}", path.display());
                std::thread::spawn(move || { let _ = child.wait(); });
            }
            Err(e) => tracing::error!("launch failed: {e}"),
        }
    });

    app.on_exit_console(|| { sounds::back(); tracing::info!("exit_console") });

    // gamepad input → UI
    let weak = app.as_weak();
    input::run(move |ev| {
        let _ = weak.upgrade_in_event_loop(move |app| {
            if matches!(ev, input::NavEvent::Back) { app.invoke_exit_console(); }
        });
    });

    app.run()?;
    Ok(())
}
