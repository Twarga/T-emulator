//! Headless screenshot tool: renders any screen to a PNG with the software renderer.
//! usage: shot <screen> <out.png> [width height]
//! screens: home home-tabs home-empty library library-search downloads settings
//!          boot psp-games psp-settings psp-sheet playing toast
use slint::platform::software_renderer::{MinimalSoftwareWindow, RepaintBufferType};
use slint::platform::{Platform, WindowAdapter};
use slint::{ComponentHandle, PhysicalSize, Rgb8Pixel, SharedPixelBuffer};
use std::cell::RefCell;
use std::rc::Rc;
use t_emulator::state::{demo, model, refresh, ErrAct, ErrCardData};
use t_emulator::*;

struct P(Rc<MinimalSoftwareWindow>);
impl Platform for P {
    fn create_window_adapter(&self) -> Result<Rc<dyn WindowAdapter>, slint::PlatformError> {
        Ok(self.0.clone())
    }
}

fn main() {
    let a: Vec<String> = std::env::args().collect();
    let screen = a.get(1).map(String::as_str).unwrap_or("home");
    let out = a.get(2).cloned().unwrap_or_else(|| "/tmp/shot.png".into());
    let w: u32 = a.get(3).and_then(|s| s.parse().ok()).unwrap_or(1600);
    let h: u32 = a.get(4).and_then(|s| s.parse().ok()).unwrap_or(900);

    let win = MinimalSoftwareWindow::new(RepaintBufferType::NewBuffer);
    slint::platform::set_platform(Box::new(P(win.clone()))).unwrap();
    win.set_size(PhysicalSize::new(w, h));

    let app = App::new().unwrap();
    let st = Rc::new(RefCell::new(demo()));
    refresh(&app, &st.borrow());
    app.set_clock("21:47".into());
    app.set_date("Wednesday, 18 November".into());
    app.set_empty_dir("/home/dan/Games/PSP".into());
    app.set_wave(slint::Image::from_rgba8(wave::render(512, 288, 14.0, wave::hue_for_month(11), 1.0)));

    match screen {
        "home" => {
            app.set_zone(1);
            app.set_home_focus(2); // first console (PSP)
        }
        "home-tabs" => {
            app.set_zone(0);
            app.set_tab(1);
        }
        "home-empty" => {
            let mut b = st.borrow_mut();
            b.cfg.recents.clear();
            b.games.clear();
            drop(b);
            refresh(&app, &st.borrow());
            app.set_zone(1);
            app.set_home_focus(0);
        }
        "home-locked" => {
            app.set_zone(1);
            app.set_home_focus(4); // PS2 tile
        }
        "library" => {
            app.set_tab(1);
            app.set_zone(1);
            app.set_lib_idx(5);
        }
        "library-search" => {
            app.set_tab(1);
            app.set_zone(2);
            let mut b = st.borrow_mut();
            b.query = "pat".into();
            drop(b);
            app.set_query("pat".into());
            refresh(&app, &st.borrow());
        }
        "library-empty" => {
            app.set_tab(1);
            app.set_zone(1);
            let mut b = st.borrow_mut();
            b.games.clear();
            drop(b);
            refresh(&app, &st.borrow());
        }
        "downloads" => {
            app.set_tab(2);
            app.set_zone(1);
            app.set_dl_idx(0);
        }
        "err" => {
            app.set_zone(1);
            let mut b = st.borrow_mut();
            b.err = Some(ErrCardData::new(
                "No Vulkan driver",
                "PPSSPP is set to Vulkan but this system has no Vulkan driver. Games may crash or not start.",
                &[(ErrAct::SwitchGl, "Use OpenGL"), (ErrAct::LaunchAnyway, "Play anyway"), (ErrAct::Dismiss, "Dismiss")],
            ));
            drop(b);
            refresh(&app, &st.borrow());
        }
        "err-missing" => {
            app.set_tab(2);
            app.set_zone(1);
            let mut b = st.borrow_mut();
            b.err = Some(ErrCardData::new(
                "PSP emulator not installed",
                "PPSSPP is needed to run PSP games. Install it from the Downloads tab.",
                &[(ErrAct::Downloads, "Downloads"), (ErrAct::Dismiss, "Dismiss")],
            ));
            drop(b);
            refresh(&app, &st.borrow());
        }
        "downloads-busy" => {
            app.set_tab(2);
            app.set_zone(1);
            let mut b = st.borrow_mut();
            b.emu.installing = true;
            b.emu.progress = 0.62;
            drop(b);
            refresh(&app, &st.borrow());
        }
        "downloads-update" => {
            app.set_tab(2);
            app.set_zone(1);
            let mut b = st.borrow_mut();
            b.emu.latest = Some("v1.21.0".into());
            drop(b);
            refresh(&app, &st.borrow());
        }
        "settings" => {
            app.set_tab(3);
            app.set_zone(1);
            app.set_set_idx(3);
        }
        "boot" => {
            app.set_boot_name("PSP".into());
            app.set_mode(1);
            app.set_boot(0.35);
        }
        "psp-games" => {
            app.set_boot_name("PSP".into());
            app.set_mode(2);
            app.set_psp_cat(1);
            app.set_psp_item(1);
        }
        "psp-settings" => {
            app.set_boot_name("PSP".into());
            app.set_mode(2);
            app.set_psp_cat(0);
            app.set_psp_item(2);
        }
        "psp-system" => {
            app.set_boot_name("PSP".into());
            app.set_mode(2);
            app.set_psp_cat(2);
            app.set_psp_item(1);
        }
        "psp-sheet" => {
            app.set_boot_name("PSP".into());
            app.set_mode(2);
            app.set_psp_cat(0);
            app.set_psp_item(1);
            app.set_psp_sheet_title("Rendering resolution".into());
            app.set_psp_opts(model(vec![
                "Auto".into(),
                "1× PSP".into(),
                "2× PSP".into(),
                "3× PSP".into(),
                "4× PSP".into(),
                "5× PSP".into(),
            ]));
            app.set_psp_sheet_cur(2);
            app.set_psp_sheet_sel(3);
            app.set_psp_sheet(true);
        }
        "playing" => {
            app.set_boot_name("PSP".into());
            app.set_mode(2);
            app.set_psp_cat(1);
            app.set_playing_title("God of War Ghost of Sparta".into());
        }
        "toast" => {
            app.set_mode(2);
            app.set_psp_cat(1);
            app.set_toast("Downloading PSP emulator".into());
            app.set_toast_progress(0.62);
            app.set_toast_shown(true);
        }
        other => panic!("unknown screen {other}"),
    }

    app.show().unwrap();
    slint::platform::update_timers_and_animations();
    let mut buf = SharedPixelBuffer::<Rgb8Pixel>::new(w, h);
    win.draw_if_needed(|r| {
        r.render(buf.make_mut_slice(), w as usize);
    });
    save(&buf, w, h, &out);
    println!("saved {out} ({w}x{h})");
}
fn miniz(raw: &[u8]) -> Vec<u8> {
    let mut z = vec![0x78, 0x01];
    let mut off = 0;
    while off < raw.len() {
        let n = (raw.len() - off).min(65535);
        let chunk = &raw[off..off + n];
        off += n;
        z.push(if off == raw.len() { 1 } else { 0 });
        z.extend_from_slice(&(n as u16).to_le_bytes());
        z.extend_from_slice(&(!(n as u16)).to_le_bytes());
        z.extend_from_slice(chunk);
    }
    let mut s1 = 1u32;
    let mut s2 = 0u32;
    for &x in raw {
        s1 = (s1 + x as u32) % 65521;
        s2 = (s2 + s1) % 65521;
    }
    z.extend_from_slice(&(s2 << 16 | s1).to_be_bytes());
    z
}
fn save(buf: &SharedPixelBuffer<Rgb8Pixel>, w: u32, h: u32, path: &str) {
    let px = buf.as_bytes();
    let mut png = Vec::with_capacity((w * h * 3) as usize + 8);
    png.extend_from_slice(&[137, 80, 78, 71, 13, 10, 26, 10]);
    let ihdr = [
        w.to_be_bytes().as_slice(),
        h.to_be_bytes().as_slice(),
        &[8u8, 2, 0, 0, 0],
    ]
    .concat();
    write_chunk(&mut png, b"IHDR", &ihdr);
    let mut raw = Vec::with_capacity((w * h * 3 + h) as usize);
    for y in 0..h as usize {
        raw.push(0);
        raw.extend_from_slice(&px[y * w as usize * 3..(y + 1) * w as usize * 3]);
    }
    write_chunk(&mut png, b"IDAT", &miniz(&raw));
    write_chunk(&mut png, b"IEND", &[]);
    std::fs::write(path, &png).unwrap();
}

fn write_chunk(png: &mut Vec<u8>, name: &[u8; 4], data: &[u8]) {
    png.extend_from_slice(&(data.len() as u32).to_be_bytes());
    png.extend_from_slice(name);
    png.extend_from_slice(data);
    let crc = crc32(name.iter().chain(data.iter()).copied());
    png.extend_from_slice(&crc.to_be_bytes());
}

fn crc32(data: impl Iterator<Item = u8>) -> u32 {
    let mut c = 0xFFFF_FFFFu32;
    for b in data {
        c ^= b as u32;
        for _ in 0..8 {
            c = if c & 1 == 1 { 0xEDB8_8320 ^ (c >> 1) } else { c >> 1 };
        }
    }
    !c
}



