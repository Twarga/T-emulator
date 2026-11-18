//! Drives the real UI component with real key events, no display needed.
use slint::platform::software_renderer::{MinimalSoftwareWindow, RepaintBufferType};
use slint::platform::{Platform, WindowAdapter, WindowEvent};
use slint::{ComponentHandle, PhysicalSize};
use std::cell::RefCell;
use std::rc::Rc;
use t_emulator::state::{demo, refresh};
use t_emulator::*;

struct P(Rc<MinimalSoftwareWindow>);
impl Platform for P {
    fn create_window_adapter(&self) -> Result<Rc<dyn WindowAdapter>, slint::PlatformError> {
        Ok(self.0.clone())
    }
}

fn press(win: &MinimalSoftwareWindow, text: slint::SharedString) {
    win.dispatch_event(WindowEvent::KeyPressed { text: text.clone() });
    win.dispatch_event(WindowEvent::KeyReleased { text });
}

fn k(k: slint::platform::Key) -> slint::SharedString {
    k.into()
}

#[test]
fn full_navigation_flow() {
    let win = MinimalSoftwareWindow::new(RepaintBufferType::NewBuffer);
    slint::platform::set_platform(Box::new(P(win.clone()))).unwrap();
    win.set_size(PhysicalSize::new(1280, 800));

    let app = App::new().unwrap();
    let st = RefCell::new(demo());
    refresh(&app, &st.borrow());
    app.show().unwrap();
    slint::platform::update_timers_and_animations();

    use slint::platform::Key;
    let (right, left, down, up, enter, esc, tab) = (
        k(Key::RightArrow),
        k(Key::LeftArrow),
        k(Key::DownArrow),
        k(Key::UpArrow),
        k(Key::Return),
        k(Key::Escape),
        k(Key::Tab),
    );

    let picked = Rc::new(RefCell::new(Vec::new()));
    let activated = Rc::new(RefCell::new(Vec::new()));
    let chosen = Rc::new(RefCell::new(Vec::new()));
    let searched = Rc::new(RefCell::new(Vec::new()));
    let quit = Rc::new(RefCell::new(false));
    let p = picked.clone();
    app.on_home_pick(move |i| p.borrow_mut().push(i));
    let a = activated.clone();
    app.on_psp_activate(move |c, i| a.borrow_mut().push((c, i)));
    let ch = chosen.clone();
    app.on_psp_choice(move |i| ch.borrow_mut().push(i));
    let se = searched.clone();
    app.on_search_key(move |s| {
        se.borrow_mut().push(s.to_string());
        true
    });
    let q = quit.clone();
    app.on_quit(move || *q.borrow_mut() = true);

    // ── tabs row: arrows switch sections ──
    assert_eq!(app.get_zone(), 0);
    press(&win, right.clone());
    assert_eq!(app.get_tab(), 1);
    press(&win, right.clone());
    press(&win, right.clone());
    press(&win, right.clone());
    assert_eq!(app.get_tab(), 3, "tab must clamp");
    press(&win, tab.clone());
    assert_eq!(app.get_tab(), 0, "Tab wraps to Home");

    // ── home: down lands on first recent, then consoles ──
    press(&win, down.clone());
    assert_eq!(app.get_zone(), 1);
    assert_eq!(app.get_home_focus(), 0, "first recent focused");
    press(&win, down.clone());
    assert_eq!(app.get_home_focus(), 2, "moves to first console (after 2 recents)");
    press(&win, right.clone());
    assert_eq!(app.get_home_focus(), 3);
    for _ in 0..9 {
        press(&win, right.clone());
    }
    assert_eq!(app.get_home_focus(), 5, "clamps at last console");

    // Enter powers on the focused console
    press(&win, left.clone());
    press(&win, left.clone());
    press(&win, enter.clone());
    assert_eq!(&*picked.borrow(), &[3], "home-pick fires with focus index");

    // Esc lifts back to tabs
    press(&win, esc.clone());
    assert_eq!(app.get_zone(), 0);

    // ── library: search + grid nav ──
    app.set_tab(1);
    press(&win, down.clone());
    assert_eq!(app.get_zone(), 1);
    press(&win, "/".into());
    assert_eq!(app.get_zone(), 2, "slash opens search");
    press(&win, "p".into());
    assert_eq!(&*searched.borrow(), &["p".to_string()]);
    press(&win, esc.clone());
    assert_eq!(app.get_zone(), 1);
    assert_eq!(searched.borrow().last().map(String::as_str), Some("clear"));
    press(&win, right.clone());
    assert_eq!(app.get_lib_idx(), 1);
    press(&win, down.clone());
    assert!(app.get_lib_idx() >= 4, "down jumps a whole row");
    press(&win, up.clone());
    press(&win, up.clone());
    assert_eq!(app.get_zone(), 0, "up past first row returns to tabs");

    // ── PSP shell: xmb nav + option sheet ──
    app.set_mode(2);
    app.set_psp_cat(1);
    app.set_psp_item(0);
    press(&win, down.clone());
    press(&win, down.clone());
    assert_eq!(app.get_psp_item(), 2);
    press(&win, enter.clone());
    assert_eq!(&*activated.borrow(), &[(1, 2)]);
    press(&win, right.clone());
    assert_eq!(app.get_psp_cat(), 2);
    assert_eq!(app.get_psp_item(), 0, "category switch resets item");

    // sheet: arrows move selection, Enter applies
    app.set_psp_opts(slint::ModelRc::new(slint::VecModel::from(
        vec!["A".into(), "B".into(), "C".into()],
    )));
    app.set_psp_sheet(true);
    app.set_psp_sheet_sel(0);
    press(&win, down.clone());
    press(&win, down.clone());
    assert_eq!(app.get_psp_sheet_sel(), 2);
    press(&win, enter.clone());
    assert_eq!(&*chosen.borrow(), &[2]);

    // Esc closes the shell
    app.set_psp_sheet(false);
    press(&win, esc.clone());
    assert_eq!(app.get_mode(), 0);

    // Esc on the tab bar quits
    press(&win, esc);
    assert!(*quit.borrow());
}
