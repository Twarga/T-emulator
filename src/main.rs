use slint::{ComponentHandle, Model, SharedString};
use std::cell::{Cell, RefCell};
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::mpsc::{channel, Sender};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use t_emulator::adapters::ppsspp::Ppsspp;
use t_emulator::config::Config;
use t_emulator::covers;
use t_emulator::pads;
use t_emulator::launcher::{self, EmuInfo};
use t_emulator::state::{model, refresh, toast, Ev, St};
use t_emulator::{ini::Ini, psp, scanner, *};

mod sounds;

fn save(st: &mut St) {
    let _ = st.cfg.save();
}

fn start_install(st: &mut St, tx: &Sender<Ev>) {
    if st.installing.swap(true, Ordering::SeqCst) {
        return;
    }
    st.emu.installing = true;
    let tx = tx.clone();
    let flag = Arc::clone(&st.installing);
    std::thread::spawn(move || {
        let emu = Ppsspp::new();
        let txp = tx.clone();
        let r = emu.install(move |f| {
            let _ = txp.send(Ev::Progress(f));
        });
        let _ = tx.send(Ev::Installed(r.map_err(|e| e.to_string())));
        flag.store(false, Ordering::SeqCst);
    });
}

fn pick_games_folder(tx: Sender<Ev>) {
    std::thread::spawn(move || {
        // zenity first, kdialog as fallback; both already on the system.
        let out = std::process::Command::new("zenity")
            .args(["--file-selection", "--directory", "--title=Choose your games folder"])
            .output()
            .ok()
            .filter(|o| o.status.success())
            .or_else(|| {
                std::process::Command::new("kdialog")
                    .args(["--getexistingdirectory", &dirs_home()])
                    .output()
                    .ok()
                    .filter(|o| o.status.success())
            });
        let dir = out
            .and_then(|o| String::from_utf8(o.stdout).ok())
            .map(|s| PathBuf::from(s.trim()));
        let _ = tx.send(Ev::FolderPicked(dir));
    });
}

fn dirs_home() -> String {
    directories::BaseDirs::new()
        .map(|b| b.home_dir().display().to_string())
        .unwrap_or_default()
}

fn main() -> anyhow::Result<()> {
    let app = App::new()?;
    let mut cfg = Config::load();
    let dir = cfg.games_dir.clone().unwrap_or_else(|| {
        let d = directories::UserDirs::new()
            .map(|u| u.home_dir().join("Games").join("PSP"))
            .unwrap_or_else(|| PathBuf::from("Games/PSP"));
        let _ = std::fs::create_dir_all(&d);
        d
    });
    cfg.games_dir = Some(dir.clone());
    let _ = cfg.save();

    let installed = Ppsspp::new().is_installed();
    let version = cfg.emu_versions.get("ppsspp").cloned();
    let pad_name0 = cfg.pad_name.clone();
    let st = Rc::new(RefCell::new(St {
        cfg,
        dir,
        games: Vec::new(),
        emu: EmuInfo {
            installed,
            installing: false,
            progress: 0.0,
            version,
            latest: None,
            check_done: false,

        },
        installing: Arc::new(AtomicBool::new(false)),
        playing: false,
        query: String::new(),
        psp_setting: String::new(),
        pad_name: pad_name0,
    }));
    st.borrow_mut().games = {
        let dir = st.borrow().dir.clone();
        scanner::scan(&dir)
    };

    let (tx, rx) = channel::<Ev>();
    

    refresh(&app, &st.borrow());
    let (c, d) = clock_strings();
    app.set_clock(c);
    app.set_date(d);
    app.set_empty_dir(st.borrow().dir.display().to_string().into());

    // background: rescan + update check + version probe
    {
        let tx = tx.clone();
        std::thread::spawn(move || {
            let emu = Ppsspp::new();
            if emu.is_installed() {
                let _ = tx.send(Ev::VersionProbe(emu.probe_version()));
            }
            let agent = ureq::Agent::config_builder()
                .http_status_as_error(false)
                .timeout_global(Some(std::time::Duration::from_secs(15)))
                .build()
                .into();
            let latest = Ppsspp::latest_release(&agent).ok().map(|r| r.tag);
            let _ = tx.send(Ev::Latest(latest));
        });
    }

    // ── box art: download covers for games missing one ──
    {
        let tx = tx.clone();
        let titles: Vec<String> = st.borrow().games.iter().map(|g| g.title.clone()).collect();
        std::thread::spawn(move || {
            if !covers::fetch_missing(&titles).is_empty() {
                let _ = tx.send(Ev::CoversDone);
            }
        });
    }

    // ── timers: clock + wave (wave only while inside a shell) ──
    let t0 = std::time::Instant::now();
    let hue = wave::hue_for_month(month());
    let weak = app.as_weak();
    let last_clock = Cell::new(0u64);
    let frame = Cell::new(0u64);
    let timer = slint::Timer::default();
    timer.start(slint::TimerMode::Repeated, std::time::Duration::from_millis(50), move || {
        let Some(app) = weak.upgrade() else { return };
        let t = t0.elapsed().as_secs_f32();
        if app.get_mode() != 0 {
            let f = (t * 20.0) as u64;
            if f != frame.get() {
                frame.set(f);
                app.set_wave(slint::Image::from_rgba8(wave::render(384, 216, t, hue, 1.0)));
            }
        }
        let sec = t0.elapsed().as_secs();
        if sec != last_clock.get() && sec % 5 == 0 {
            last_clock.set(sec);
            let (c, d) = clock_strings();
            app.set_clock(c);
            app.set_date(d);
        }
    });

    // ── background event pump ──
    {
        let weak = app.as_weak();
        let st = st.clone();
        let txp = tx.clone();
        let pump = slint::Timer::default();
        pump.start(slint::TimerMode::Repeated, std::time::Duration::from_millis(80), move || {
            let Some(app) = weak.upgrade() else { return };
            while let Ok(ev) = rx.try_recv() {
                let mut b = st.borrow_mut();
                match ev {
                    Ev::Progress(f) => {
                        b.emu.progress = f;
                    }
                    Ev::Installed(Ok(tag)) => {
                        b.emu.installing = false;
                        b.emu.installed = true;
                        if !tag.is_empty() {
                            b.emu.version = Some(tag.clone());
                            b.cfg.emu_versions.insert("ppsspp".into(), tag);
                            let _ = b.cfg.save();
                        }
                        toast(&app, "PSP emulator installed", -1.0, &txp, 2200);
                    }
                    Ev::Installed(Err(e)) => {
                        b.emu.installing = false;
                        toast(&app, &format!("Install failed: {e}"), -1.0, &txp, 4000);
                    }
                    Ev::Latest(l) => {
                        b.emu.check_done = true;
                        b.emu.latest = l;
                    }
                    Ev::VersionProbe(v) => {
                        if b.emu.version.is_none() {
                            if let Some(v) = v {
                                b.cfg.emu_versions.insert("ppsspp".into(), v.clone());
                                let _ = b.cfg.save();
                                b.emu.version = Some(v);
                            }
                        }
                    }
                    Ev::GameExit => {
                        b.playing = false;

                        app.set_playing_title("".into());
                    }
                    Ev::FolderPicked(Some(d)) => {
                        b.dir = d.clone();
                        b.cfg.games_dir = Some(d);
                        let _ = b.cfg.save();
                        b.games = scanner::scan(&b.dir);
                        let titles: Vec<String> =
                            b.games.iter().map(|g| g.title.clone()).collect();
                        let txf = txp.clone();
                        std::thread::spawn(move || {
                            if !covers::fetch_missing(&titles).is_empty() {
                                let _ = txf.send(Ev::CoversDone);
                            }
                        });
                        app.set_empty_dir(b.dir.display().to_string().into());
                        toast(&app, "Games folder updated", -1.0, &txp, 2000);
                    }
                    Ev::FolderPicked(None) => {}
                    Ev::Key(c) => {
                        let s: SharedString = c.to_string().into();
                        app.window().dispatch_event(slint::platform::WindowEvent::KeyPressed {
                            text: s.clone(),
                        });
                        app.window().dispatch_event(slint::platform::WindowEvent::KeyReleased {
                            text: s,
                        });
                    }
                    Ev::CoversDone => {} // refresh() below re-reads cover paths
                    Ev::Pad(name) => {
                        let was = b.pad_name.clone();
                        b.pad_name = name.clone();
                        b.cfg.pad_name = name.clone();
                        let _ = b.cfg.save();
                        match (&was, &name) {
                            (None, Some(n)) => toast(&app, &format!("{n} connected — configured for PSP"), -1.0, &txp, 2600),
                            (Some(_), None) => toast(&app, "Gamepad disconnected", -1.0, &txp, 1800),
                            _ => {}
                        }
                    }
                    Ev::ToastHide => app.set_toast_shown(false),
                }
                refresh(&app, &b);
            }
        });
    }

    // ── gamepad → synthetic keys ──
    {
        let tx = tx.clone();
        std::thread::spawn(move || {
            let Ok(mut g) = gilrs::Gilrs::new() else { return };
            let map = |b: gilrs::Button| -> Option<char> {
                use gilrs::Button::*;
                Some(match b {
                    DPadUp => '\u{F700}',
                    DPadDown => '\u{F701}',
                    DPadLeft => '\u{F702}',
                    DPadRight => '\u{F703}',
                    South => '\u{000a}',
                    East => '\u{001b}',
                    LeftTrigger => '\u{0019}',
                    RightTrigger => '\t',
                    _ => return None,
                })
            };
            let mut repeat = std::collections::HashMap::new();
            let mut last = std::time::Instant::now();
            // announce pads already connected
            if let Some((_, p)) = g.gamepads().next() {
                let name = p.name().to_string();
                t_emulator::pads::ensure_all(&name);
                let _ = tx.send(Ev::Pad(Some(name)));
            }
            loop {
                while let Some(e) = g.next_event() {
                    use gilrs::EventType::*;
                    match e.event {
                        ButtonPressed(b, _) | ButtonRepeated(b, _) => {
                            if let Some(c) = map(b) {
                                let _ = tx.send(Ev::Key(c));
                            }
                        }
                        Connected => {
                            let name = g.gamepad(e.id).name().to_string();
                            t_emulator::pads::ensure_all(&name);
                            let _ = tx.send(Ev::Pad(Some(name)));
                        }
                        Disconnected => {
                            let _ = tx.send(Ev::Pad(None));
                        }
                        _ => {}
                    }
                }
                // left stick → arrows at 4 Hz
                if last.elapsed().as_millis() > 250 {
                    last = std::time::Instant::now();
                    if let Some(gp) = g.gamepads().next().map(|(_, p)| p) {
                        let (x, y) = (gp.value(gilrs::Axis::LeftStickX), gp.value(gilrs::Axis::LeftStickY));
                        let c = if x < -0.5 {
                            Some('\u{F702}')
                        } else if x > 0.5 {
                            Some('\u{F703}')
                        } else if y > 0.5 {
                            Some('\u{F700}')
                        } else if y < -0.5 {
                            Some('\u{F701}')
                        } else {
                            None
                        };
                        if let Some(c) = c {
                            if repeat.get(&c).map(|t: &std::time::Instant| t.elapsed().as_millis() > 250).unwrap_or(true) {
                                repeat.insert(c, std::time::Instant::now());
                                let _ = tx.send(Ev::Key(c));
                            }
                        } else {
                            repeat.clear();
                        }
                    }
                }
                std::thread::sleep(std::time::Duration::from_millis(60));
            }
        });
    }

    {
        let st = st.clone();
        app.on_sfx(move |n| {
            if st.borrow().cfg.sounds {
                sounds::play(n.as_str());
            }
        });
    }

    app.on_quit(|| {
        let _ = slint::quit_event_loop();
    });

    // clicking a top tab: jump there (Slint already focuses the bar)
    {
        let weak = app.as_weak();
        app.on_pick_tab(move |i| {
            let Some(app) = weak.upgrade() else { return };
            app.set_tab(i);
            app.set_zone(0);
            sounds::play("tick");
        });
    }

    // power-on / recent resume from Home
    {
        let weak = app.as_weak();
        let st = st.clone();
        let tx = tx.clone();
        app.on_home_pick(move |f| {
            let Some(app) = weak.upgrade() else { return };
            let mut b = st.borrow_mut();
            let rec = launcher::recent_cards(&b.cfg, &b.games);
            let f = f as usize;
            if f < rec.len() {
                let gi = rec[f].idx as usize;
                launch_game(&app, &mut b, gi, &tx);
                return;
            }
            let ci = f - rec.len();
            let tiles = launcher::console_tiles(b.games.len(), b.emu.installed, b.emu.installing);
            let Some(t) = tiles.get(ci) else { return };
            if !t.available {
                sounds::play("back");
                toast(&app, "PS3, GameCube and PS2 come in later versions", -1.0, &tx, 2600);
                return;
            }
            sounds::play("select");
            app.set_boot_name(t.name.clone());
            app.set_boot(0.0);
            app.set_mode(1);
            if !b.emu.installed && !b.emu.installing {
                start_install(&mut b, &tx);
                toast(&app, "Downloading PSP emulator…", -1.0, &tx, 3000);
            }
        });
    }

    // library launch
    {
        let weak = app.as_weak();
        let st = st.clone();
        let tx = tx.clone();
        app.on_lib_pick(move |gi| {
            let Some(app) = weak.upgrade() else { return };
            let mut b = st.borrow_mut();
            launch_game(&app, &mut b, gi as usize, &tx);
        });
    }

    // downloads rows
    {
        let weak = app.as_weak();
        let st = st.clone();
        let tx = tx.clone();
        app.on_emu_act(move |i| {
            let Some(app) = weak.upgrade() else { return };
            let mut b = st.borrow_mut();
            if i != 0 {
                return; // planned rows do nothing
            }
            match b.emu.state() {
                launcher::EmuState::Install | launcher::EmuState::Update => {
                    sounds::play("select");
                    start_install(&mut b, &tx);
                    refresh(&app, &b);
                }
                _ => {}
            }
        });
    }

    // settings rows
    {
        let weak = app.as_weak();
        let st = st.clone();
        let tx = tx.clone();
        app.on_set_act(move |i| {
            let Some(app) = weak.upgrade() else { return };
            let mut b = st.borrow_mut();
            let rows = launcher::settings_rows(&b.cfg, &b.dir.display().to_string(), b.pad_name.as_deref());
            let Some(row) = rows.get(i as usize) else { return };
            match row.id.as_str() {
                "folder" => pick_games_folder(tx.clone()),
                "rescan" => {
                    b.games = scanner::scan(&b.dir);
                    toast(&app, &format!("Found {} games", b.games.len()), -1.0, &tx, 1800);
                }
                "sounds" => {
                    b.cfg.sounds = !b.cfg.sounds;
                    save(&mut b);
                }
                "fullscreen" => {
                    b.cfg.fullscreen = !b.cfg.fullscreen;
                    save(&mut b);
                }
                "update-check" => {
                    b.cfg.update_check = !b.cfg.update_check;
                    save(&mut b);
                }
                "pad-auto" => {
                    b.cfg.pad_auto = !b.cfg.pad_auto;
                    save(&mut b);
                    if b.cfg.pad_auto {
                        let name = b.pad_name.clone().unwrap_or_else(|| "pad".into());
                        pads::ensure_all(&name);
                        toast(&app, "Gamepad bindings applied", -1.0, &tx, 1800);
                    }
                }
                _ => {}
            }
            refresh(&app, &b);
        });
    }

    app.on_open_folder({
        let st = st.clone();
        move || {
            let _ = std::process::Command::new("xdg-open").arg(&st.borrow().dir).spawn();
        }
    });

    // search typing
    {
        let weak = app.as_weak();
        let st = st.clone();
        app.on_search_key(move |k| {
            let Some(app) = weak.upgrade() else { return true };
            let mut b = st.borrow_mut();
            let s = k.as_str();
            if s == "clear" {
                b.query.clear();
            } else if s == "\u{8}" {
                b.query.pop();
            } else if s.chars().count() == 1 && s.chars().next().is_some_and(|c| !c.is_control()) {
                b.query.push_str(s);
            } else {
                return true; // consume arrows/Tab while searching
            }
            app.set_query(b.query.clone().into());
            app.set_lib_idx(0);
            app.set_lib_scroll(0.0);
            refresh(&app, &b);
            true
        });
    }

    // PSP XMB activation
    {
        let weak = app.as_weak();
        let st = st.clone();
        let tx = tx.clone();
        app.on_psp_activate(move |cat, item| {
            let Some(app) = weak.upgrade() else { return };
            let mut b = st.borrow_mut();
            let ini = Ini::load(&psp::ini_path()).unwrap_or_default();
            let ver = b.emu.version.clone().unwrap_or_else(|| "installed".into());
            let cats = psp::categories(&b.games, b.emu.installed, &ver, &b.dir.display().to_string(), &ini, b.cfg.fullscreen);
            let Some(id) = cats
                .get(cat as usize)
                .and_then(|c| c.items.row_data(item as usize))
                .map(|it| it.id.to_string())
            else {
                return;
            };
            match id.as_str() {
                "none" => toast(&app, "Put .iso / .cso / .pbp files in your PSP folder", -1.0, &tx, 2600),
                "emu" => {
                    if !b.emu.installing {
                        sounds::play("select");
                        start_install(&mut b, &tx);
                        refresh(&app, &b);
                    }
                }
                "folder" => {
                    let _ = std::process::Command::new("xdg-open").arg(&b.dir).spawn();
                }
                "rescan" => {
                    b.games = scanner::scan(&b.dir);
                    toast(&app, &format!("Found {} games", b.games.len()), -1.0, &tx, 1800);
                    refresh(&app, &b);
                }
                "exit" => {
                    app.set_mode(0);
                    sounds::play("back");
                }
                _ => {
                    if let Some(rest) = id.strip_prefix('g') {
                        if let Ok(gi) = rest.parse::<usize>() {
                            launch_game(&app, &mut b, gi, &tx);
                        }
                        return;
                    }
                    // a settings row → open the option sheet
                    if let Some(s) = psp::SETTINGS.iter().find(|s| s.id == id) {
                        let lv = if matches!(s.kind, psp::Kind::Launcher) {
                            Some(if b.cfg.fullscreen { "True" } else { "False" })
                        } else {
                            None
                        };
                        let cur = psp::current(&id, &ini, lv);
                        b.psp_setting = id.clone();
                        app.set_psp_opts(model(s.opts.iter().map(|o| SharedString::from(o.label)).collect()));
                        app.set_psp_sheet_title(s.label.into());
                        app.set_psp_sheet_cur(if cur == usize::MAX { -1 } else { cur as i32 });
                        app.set_psp_sheet_sel(if cur == usize::MAX { 0 } else { cur as i32 });
                        app.set_psp_sheet(true);
                        sounds::play("select");
                    }
                }
            }
        });
    }

    // option sheet choice
    {
        let weak = app.as_weak();
        let st = st.clone();
        let tx = tx.clone();
        app.on_psp_choice(move |opt| {
            let Some(app) = weak.upgrade() else { return };
            let mut b = st.borrow_mut();
            let id = b.psp_setting.clone();
            if id.is_empty() {
                app.set_psp_sheet(false);
                return;
            }
            if id == "fullscreen" {
                b.cfg.fullscreen = opt == 1;
                let _ = b.cfg.save();
            } else {
                let path = psp::ini_path();
                let mut ini = Ini::load(&path).unwrap_or_default();
                psp::apply(&id, opt as usize, &mut ini);
                match ini.save(&path) {
                    Ok(()) => {}
                    Err(e) => toast(&app, &format!("Could not write ppsspp.ini: {e}"), -1.0, &tx, 3200),
                }
            }
            app.set_psp_sheet(false);
            sounds::play("select");
            refresh(&app, &b);
        });
    }

    app.run()?;
    Ok(())
}

fn launch_game(app: &App, st: &mut St, gi: usize, tx: &Sender<Ev>) {
    let emu = Ppsspp::new();
    let Some(g) = st.games.get(gi).cloned() else { return };
    if !emu.is_installed() {
        if !st.emu.installing {
            toast(app, "PSP emulator is not installed — see the Downloads tab", -1.0, tx, 2800);
        } else {
            toast(app, "PSP emulator is still installing…", -1.0, tx, 2200);
        }
        return;
    }
    if st.playing {
        return;
    }
    match emu.launch(&g.path, st.cfg.fullscreen) {
        Ok(mut child) => {
            st.playing = true;

            app.set_playing_title(g.title.clone().into());
            st.cfg.played(g.path.clone());
            let _ = st.cfg.save();
            sounds::play("select");
            let tx = tx.clone();
            std::thread::spawn(move || {
                let _ = child.wait();
                let _ = tx.send(Ev::GameExit);
            });
        }
        Err(e) => toast(app, &format!("Could not start PPSSPP: {e}"), -1.0, tx, 4000),
    }
}
