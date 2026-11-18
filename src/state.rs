//! Shared app state + the one place that pushes it into the UI.
//! Kept in the library so the screenshot tool builds the same screens.

use crate::config::Config;
use crate::ini::Ini;
use crate::launcher::{self, EmuInfo};
use crate::{psp, scanner, App};
use slint::{ModelRc, VecModel};
use std::path::PathBuf;
use std::sync::atomic::AtomicBool;
use std::sync::mpsc::Sender;
use std::sync::Arc;

#[derive(Clone)]
pub struct ErrCardData {
    pub title: String,
    pub msg: String,
    pub opts: Vec<String>,
    pub acts: Vec<ErrAct>,
}

#[derive(Clone, Copy, PartialEq)]
pub enum ErrAct {
    Rescan,
    Downloads,
    SwitchGl,
    LaunchAnyway,
    Dismiss,
}

impl ErrCardData {
    pub fn new(title: &str, msg: &str, acts: &[(ErrAct, &str)]) -> Self {
        Self {
            title: title.into(),
            msg: msg.into(),
            opts: acts.iter().map(|(_, l)| l.to_string()).collect(),
            acts: acts.iter().map(|(a, _)| *a).collect(),
        }
    }
}

pub enum Ev {
    Progress(f32),
    Installed(Result<String, String>),
    Latest(Option<String>),
    VersionProbe(Option<String>),
    GameExit,
    FolderPicked(Option<PathBuf>),
    Key(char),
    ToastHide,
    CoversDone,
    Pad(Option<String>),
}

pub struct St {
    pub cfg: Config,
    pub dir: PathBuf,
    pub games: Vec<scanner::Game>,
    pub emu: EmuInfo,
    pub installing: Arc<AtomicBool>,
    pub playing: bool,
    pub query: String,
    /// setting id of the open PSP option sheet
    pub psp_setting: String,
    /// currently connected gamepad name, if any
    pub pad_name: Option<String>,
    /// modal error card (title, message, option labels, action ids)
    pub err: Option<ErrCardData>,
}

pub fn model<T: Clone + 'static>(v: Vec<T>) -> ModelRc<T> {
    ModelRc::new(VecModel::from(v))
}

pub fn toast(app: &App, text: &str, progress: f32, tx: &Sender<Ev>, ms: u64) {
    app.set_toast(text.into());
    app.set_toast_progress(progress);
    app.set_toast_shown(true);
    let tx = tx.clone();
    std::thread::spawn(move || {
        std::thread::sleep(std::time::Duration::from_millis(ms));
        let _ = tx.send(Ev::ToastHide);
    });
}

/// Push every list/value the UI renders from current state.
pub fn refresh(app: &App, st: &St) {
    app.set_consoles(model(launcher::console_tiles(
        st.games.len(),
        st.emu.installed,
        st.emu.installing,
    )));
    app.set_recents(model(launcher::recent_cards(&st.cfg, &st.games)));
    app.set_games(model(launcher::game_tiles(&st.games, &st.query)));
    app.set_emus(model(launcher::emu_rows(&st.emu)));
    app.set_settings(model(launcher::settings_rows(&st.cfg, &st.dir.display().to_string(), st.pad_name.as_deref())));
    let (et, em, eo) = match &st.err {
        Some(e) => (e.title.clone(), e.msg.clone(), e.opts.clone()),
        None => (String::new(), String::new(), Vec::new()),
    };
    app.set_err_title(et.into());
    app.set_err_msg(em.into());
    app.set_err_opts(model(eo));
    app.set_attn(matches!(st.emu.state(), launcher::EmuState::Install | launcher::EmuState::Update));

    let ini = Ini::load(&psp::ini_path()).unwrap_or_default();
    let ver = st.emu.version.clone().unwrap_or_else(|| "installed".into());
    app.set_psp_categories(model(psp::categories(
        &st.games,
        st.emu.installed,
        &ver,
        &st.dir.display().to_string(),
        &ini,
        st.cfg.fullscreen,
    )));
}

/// Fake state for the screenshot tool — no disk, no network.
pub fn demo() -> St {
    let mk = |t: &str, s: &str| scanner::Game {
        title: t.into(),
        path: PathBuf::from(format!("/games/{t}.iso")),
        size: s.into(),
    };
    let mut cfg = Config::default();
    cfg.games_dir = Some("/home/dan/Games/PSP".into());
    cfg.pad_name = Some("Generic USB Gamepad".into());
    cfg.recents = vec![
        (PathBuf::from("/games/Crisis Core Final Fantasy VII.iso"), now() - 5_400),
        (PathBuf::from("/games/Patapon 3.iso"), now() - 200_000),
    ];
    St {
        cfg,
        dir: "/home/dan/Games/PSP".into(),
        games: vec![
            mk("God of War Ghost of Sparta", "1.6 GB"),
            mk("Shin Megami Tensei - Persona 3 Portable", "1.1 GB"),
            mk("Patapon 3", "620 MB"),
            mk("Crisis Core Final Fantasy VII", "1.7 GB"),
            mk("Monster Hunter Freedom Unite", "880 MB"),
            mk("Daxter", "1.0 GB"),
            mk("Patapon 2", "500 MB"),
            mk("LocoRoco 2", "1.3 GB"),
            mk("Final Fantasy Tactics - The War of the Lions", "230 MB"),
        ],
        emu: EmuInfo {
            installed: true,
            installing: false,
            progress: 0.0,
            version: Some("v1.20.4".into()),
            latest: Some("v1.20.4".into()),
            check_done: true,
        },
        installing: Arc::new(AtomicBool::new(false)),
        playing: false,
        query: String::new(),
        psp_setting: String::new(),
        pad_name: Some("Generic USB Gamepad".into()),
        err: None,
    }
}

fn now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}
