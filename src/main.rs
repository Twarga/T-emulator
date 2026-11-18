use slint::{ComponentHandle, ModelRc, VecModel};
use std::cell::{Cell, RefCell};
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use t_emulator::{adapters::ppsspp::Ppsspp, scanner, *};

mod sounds;

fn static_name(s: &str) -> &'static str {
    match s {
        "select" => "select",
        "back" => "back",
        _ => "tick",
    }
}

fn games_dir() -> PathBuf {
    let mut cfg = config::Config::load();
    if let Some(d) = cfg.games_dir.clone() {
        return d;
    }
    let d = directories::UserDirs::new()
        .map(|u| u.home_dir().join("Games").join("PSP"))
        .unwrap_or_else(|| PathBuf::from("Games/PSP"));
    let _ = std::fs::create_dir_all(&d);
    cfg.games_dir = Some(d.clone());
    let _ = cfg.save();
    d
}

fn main() -> anyhow::Result<()> {
    let app = App::new()?;
    let dir = games_dir();
    let games: Rc<RefCell<Vec<scanner::Game>>> = Rc::new(RefCell::new(scanner::scan(&dir)));
    let ready = Rc::new(Cell::new(Ppsspp::new().is_installed()));
    let installing = Arc::new(AtomicBool::new(false));
    let playing = Rc::new(Cell::new(false));

    app.set_consoles(ModelRc::new(VecModel::from(consoles())));
    app.set_categories(ModelRc::new(VecModel::from(categories(&games.borrow(), ready.get()))));
    let (c, d) = clock_strings();
    app.set_clock(c);
    app.set_date(d);

    // ── animated background + clock ──
    let t0 = std::time::Instant::now();
    let hue = wave::hue_for_month(month());
    let weak = app.as_weak();
    let timer = slint::Timer::default();
    let last_clock = Cell::new(0u64);
    timer.start(slint::TimerMode::Repeated, std::time::Duration::from_millis(50), move || {
        let Some(app) = weak.upgrade() else { return };
        let t = t0.elapsed().as_secs_f32();
        app.set_wave(slint::Image::from_rgba8(wave::render(384, 216, t, hue, 1.0)));
        let sec = t0.elapsed().as_secs();
        if sec != last_clock.get() && sec % 5 == 0 {
            last_clock.set(sec);
            let (c, d) = clock_strings();
            app.set_clock(c);
            app.set_date(d);
        }
    });

    app.on_sfx(|n| sounds::play(static_name(n.as_str())));
    app.on_quit(|| {
        let _ = slint::quit_event_loop();
    });

    // power on: only PSP is real in v0.1
    {
        let weak = app.as_weak();
        let installing = installing.clone();
        let ready = ready.clone();
        app.on_power_on(move |i| {
            let Some(app) = weak.upgrade() else { return };
            if !consoles()[i as usize].available {
                app.set_toast("PS3, GameCube and PS2 come in later versions".into());
                app.set_toast_progress(-1.0);
                app.set_toast_shown(true);
                hide_toast_later(app.as_weak(), 2600);
                sounds::play("back");
                return;
            }
            sounds::play("select");
            app.set_mode(1);
            if !ready.get() && !installing.swap(true, Ordering::SeqCst) {
                start_install(app.as_weak(), installing.clone());
            }
        });
    }

    // menu activation
    {
        let weak = app.as_weak();
        let games = games.clone();
        let ready = ready.clone();
        let installing = installing.clone();
        let playing = playing.clone();
        let dir = dir.clone();
        app.on_activate(move |cat, idx| {
            let Some(app) = weak.upgrade() else { return };
            sounds::play("select");
            match (cat, idx) {
                (1, i) => {
                    let g = games.borrow();
                    let Some(game) = g.get(i as usize) else { return };
                    let emu = Ppsspp::new();
                    if !emu.is_installed() {
                        toast(&app, "PSP emulator is still installing…", -1.0, 2600);
                        return;
                    }
                    if playing.get() {
                        return;
                    }
                    match emu.launch(&game.path) {
                        Ok(mut child) => {
                            playing.set(true);
                            let flag = Arc::new(AtomicBool::new(false));
                            let f2 = flag.clone();
                            std::thread::spawn(move || {
                                let _ = child.wait();
                                f2.store(true, Ordering::SeqCst);
                            });
                            poll_exit(flag, playing.clone());
                        }
                        Err(e) => toast(&app, &format!("Could not start PPSSPP: {e}"), -1.0, 4000),
                    }
                }
                (0, 0) => {
                    let _ = std::process::Command::new("xdg-open").arg(&dir).spawn();
                }
                (0, 1) => {
                    *games.borrow_mut() = scanner::scan(&dir);
                    app.set_categories(ModelRc::new(VecModel::from(categories(&games.borrow(), ready.get()))));
                    toast(&app, &format!("Found {} games", games.borrow().len()), -1.0, 1800);
                }
                (0, 2) => {
                    if !installing.swap(true, Ordering::SeqCst) {
                        start_install(app.as_weak(), installing.clone());
                    }
                }
                _ => {}
            }
        });
    }

    app.run()?;
    Ok(())
}

fn toast(app: &App, text: &str, progress: f32, ms: u64) {
    app.set_toast(text.into());
    app.set_toast_progress(progress);
    app.set_toast_shown(true);
    if ms > 0 {
        hide_toast_later(app.as_weak(), ms);
    }
}

fn hide_toast_later(weak: slint::Weak<App>, ms: u64) {
    slint::Timer::single_shot(std::time::Duration::from_millis(ms), move || {
        if let Some(app) = weak.upgrade() {
            app.set_toast_shown(false);
        }
    });
}

fn poll_exit(flag: Arc<AtomicBool>, playing: Rc<Cell<bool>>) {
    slint::Timer::single_shot(std::time::Duration::from_millis(400), move || {
        if flag.load(Ordering::SeqCst) {
            playing.set(false);
        } else {
            poll_exit(flag, playing);
        }
    });
}

fn start_install(weak: slint::Weak<App>, installing: Arc<AtomicBool>) {
    if let Some(app) = weak.upgrade() {
        toast(&app, "Downloading PSP emulator", 0.0, 0);
    }
    let (tx, rx) = std::sync::mpsc::channel::<Result<f32, String>>();
    std::thread::spawn(move || {
        let res = Ppsspp::new().install(|p| {
            let _ = tx.send(Ok(p));
        });
        let _ = tx.send(match res {
            Ok(()) => Ok(2.0),
            Err(e) => Err(e.to_string()),
        });
    });
    pump_install(weak, rx, installing);
}

fn pump_install(weak: slint::Weak<App>, rx: std::sync::mpsc::Receiver<Result<f32, String>>, installing: Arc<AtomicBool>) {
    slint::Timer::single_shot(std::time::Duration::from_millis(120), move || {
        let Some(app) = weak.upgrade() else { return };
        let mut done = false;
        while let Ok(msg) = rx.try_recv() {
            match msg {
                Ok(p) if p >= 2.0 => {
                    toast(&app, "PSP emulator ready", -1.0, 2200);
                    done = true;
                }
                Ok(p) => app.set_toast_progress(p),
                Err(e) => {
                    toast(&app, &format!("Install failed: {e}"), -1.0, 5000);
                    done = true;
                }
            }
        }
        if done {
            installing.store(false, Ordering::SeqCst);
        } else {
            pump_install(app.as_weak(), rx, installing);
        }
    });
}
