//! Drives the real UI component with real key events, no display needed.
use slint::platform::software_renderer::{MinimalSoftwareWindow, RepaintBufferType};
use slint::platform::{Platform, WindowAdapter, WindowEvent};
use slint::{ComponentHandle, ModelRc, PhysicalSize, VecModel};
use std::cell::RefCell;
use std::rc::Rc;
use t_emulator::{scanner::Game, *};

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

#[test]
fn full_navigation_flow() {
    let win = MinimalSoftwareWindow::new(RepaintBufferType::NewBuffer);
    slint::platform::set_platform(Box::new(P(win.clone()))).unwrap();
    win.set_size(PhysicalSize::new(1280, 800));

    let app = App::new().unwrap();
    let games: Vec<Game> = ["A", "B", "C"]
        .iter()
        .map(|t| Game { title: t.to_string(), path: format!("/{t}.iso").into(), size: "1 MB".into() })
        .collect();
    app.set_consoles(ModelRc::new(VecModel::from(consoles())));
    app.set_categories(ModelRc::new(VecModel::from(categories(&games, true))));

    let powered = Rc::new(RefCell::new(Vec::new()));
    let activated = Rc::new(RefCell::new(Vec::new()));
    let p = powered.clone();
    app.on_power_on(move |i| p.borrow_mut().push(i));
    let a = activated.clone();
    app.on_activate(move |c, i| a.borrow_mut().push((c, i)));
    app.show().unwrap();
    slint::platform::update_timers_and_animations();

    use slint::platform::Key;
    let k = |k: Key| -> slint::SharedString { k.into() };
    let (right, left, down, enter, esc) =
        (k(Key::RightArrow), k(Key::LeftArrow), k(Key::DownArrow), k(Key::Return), k(Key::Escape));

    // launcher: clamp at both ends
    press(&win, left.clone());
    assert_eq!(app.get_console_idx(), 0);
    for _ in 0..9 {
        press(&win, right.clone());
    }
    assert_eq!(app.get_console_idx(), 3, "must clamp at last console");
    for _ in 0..9 {
        press(&win, left.clone());
    }
    assert_eq!(app.get_console_idx(), 0);

    // power on PSP
    press(&win, enter.clone());
    assert_eq!(&*powered.borrow(), &[0]);

    // power-on animation: Enter skips it
    app.set_mode(1);
    press(&win, enter.clone());
    assert_eq!(app.get_boot(), 1.0, "Enter must skip the boot animation");

    // menu: starts on Games, move down twice, clamp, activate
    app.set_mode(2);
    assert_eq!(app.get_cat_idx(), 1);
    press(&win, down.clone());
    press(&win, down.clone());
    press(&win, down.clone());
    press(&win, down);
    assert_eq!(app.get_item_idx(), 2, "must clamp at last game");
    press(&win, enter);
    assert_eq!(&*activated.borrow(), &[(1, 2)]);

    // changing category resets the item cursor
    press(&win, left.clone());
    assert_eq!(app.get_cat_idx(), 0);
    assert_eq!(app.get_item_idx(), 0);

    // Esc returns to the launcher
    press(&win, esc);
    assert_eq!(app.get_mode(), 0);
}
