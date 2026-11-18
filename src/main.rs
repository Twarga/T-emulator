mod config;
mod adapters;
mod scanner;
mod input;

use anyhow::Result;
use slint::{ModelRc, VecModel, SharedString, Color};
use adapters::EmulatorAdapter;

slint::include_modules!();

fn main() -> Result<()> {
    tracing_subscriber::fmt::init();
    let cfg = config::Config::load();

    let app = App::new()?;

    // consoles: only PSP enabled in v0.1
    let consoles = vec![
        ConsoleEntry { title: SharedString::from("PSP"), subtitle: SharedString::from("PlayStation Portable"), enabled: true, accent: Color::from_rgb_u8(0x4e, 0xa8, 0xff) },
        ConsoleEntry { title: SharedString::from("PS3"), subtitle: SharedString::from("PlayStation 3 — soon"), enabled: false, accent: Color::from_rgb_u8(0x4e, 0xa8, 0xff) },
        ConsoleEntry { title: SharedString::from("GC / Wii"), subtitle: SharedString::from("Nintendo — soon"), enabled: false, accent: Color::from_rgb_u8(0x6a, 0x4f, 0xff) },
        ConsoleEntry { title: SharedString::from("PS2"), subtitle: SharedString::from("PlayStation 2 — soon"), enabled: false, accent: Color::from_rgb_u8(0x4e, 0xa8, 0xff) },
    ];
    app.set_consoles(ModelRc::new(VecModel::from(consoles)));

    // scan games if dir configured
    if let Some(dir) = &cfg.games_dir {
        let games = scanner::psp::scan(dir);
        let models: Vec<GameModel> = games.iter().map(|g| GameModel {
            title: SharedString::from(g.title.clone()),
            path: SharedString::from(g.path.to_string_lossy().to_string()),
            size: SharedString::from(g.size.clone()),
            last_played: SharedString::from("—"),
        }).collect();
        app.set_games(ModelRc::new(VecModel::from(models)));
    }

    // wire callbacks
    app.on_power_on(|name| tracing::info!("power_on {name}"));
    app.on_play_game(move |path| {
        let path = std::path::PathBuf::from(path.as_str());
        let adapter = adapters::ppsspp::Ppsspp::new();
        match adapter.launch(&path) {
            Ok(_) => tracing::info!("launched {}", path.display()),
            Err(e) => tracing::error!("launch failed: {e}"),
        }
    });
    app.on_exit_console(|| tracing::info!("exit_console"));

    // gamepad input
    input::run(|ev| tracing::debug!("nav {ev:?}"));

    app.run()?;
    Ok(())
}
