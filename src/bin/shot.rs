//! Headless screenshot tool: renders any screen to a PNG with the software renderer.
//! usage: shot <launcher|boot|xmb|xmb-settings> <out.png> [width height]
use slint::platform::software_renderer::{MinimalSoftwareWindow, RepaintBufferType};
use slint::platform::{Platform, WindowAdapter};
use slint::{ComponentHandle, ModelRc, PhysicalSize, Rgb8Pixel, SharedPixelBuffer, VecModel};
use std::rc::Rc;
use t_emulator::{scanner::Game, *};

struct P(Rc<MinimalSoftwareWindow>);
impl Platform for P {
    fn create_window_adapter(&self) -> Result<Rc<dyn WindowAdapter>, slint::PlatformError> {
        Ok(self.0.clone())
    }
}

fn main() {
    let a: Vec<String> = std::env::args().collect();
    let screen = a.get(1).map(String::as_str).unwrap_or("launcher");
    let out = a.get(2).cloned().unwrap_or_else(|| "/tmp/shot.png".into());
    let w: u32 = a.get(3).and_then(|s| s.parse().ok()).unwrap_or(1600);
    let h: u32 = a.get(4).and_then(|s| s.parse().ok()).unwrap_or(900);

    let win = MinimalSoftwareWindow::new(RepaintBufferType::NewBuffer);
    slint::platform::set_platform(Box::new(P(win.clone()))).unwrap();
    win.set_size(PhysicalSize::new(w, h));

    let app = App::new().unwrap();
    let games: Vec<Game> = ["God of War Ghost of Sparta", "Persona 3 Portable", "Patapon 3", "Crisis Core Final Fantasy VII", "Monster Hunter Freedom Unite"]
        .iter()
        .map(|t| Game { title: t.to_string(), path: "/x".into(), size: "1.4 GB".into() })
        .collect();
    app.set_consoles(ModelRc::new(VecModel::from(consoles())));
    app.set_categories(ModelRc::new(VecModel::from(categories(&games, true))));
    app.set_clock("21:47".into());
    app.set_date("Wednesday, 18 November".into());

    let t = 14.0;
    let (ww, wh) = (512, 288);
    app.set_wave(slint::Image::from_rgba8(wave::render(ww, wh, t, wave::hue_for_month(11), 1.0)));

    match screen {
        "launcher" => {}
        "launcher2" => app.set_console_idx(1),
        "boot" => {
            app.set_mode(1);
            app.set_boot(0.35);
        }
        "boot2" => {
            app.set_mode(1);
            app.set_boot(0.8);
        }
        "xmb" => {
            app.set_mode(2);
            app.set_cat_idx(1);
            app.set_item_idx(1);
        }
        "xmb-settings" => {
            app.set_mode(2);
            app.set_cat_idx(0);
            app.set_item_idx(0);
        }
        "toast" => {
            app.set_mode(2);
            app.set_cat_idx(1);
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
    let img = image_from(&buf, w, h);
    img.save(&out).unwrap();
    println!("saved {out} ({w}x{h})");
}

fn image_from(buf: &SharedPixelBuffer<Rgb8Pixel>, w: u32, h: u32) -> PngOut {
    PngOut { w, h, data: buf.as_bytes().to_vec() }
}

struct PngOut {
    w: u32,
    h: u32,
    data: Vec<u8>,
}
impl PngOut {
    fn save(&self, path: &str) -> std::io::Result<()> {
        // write a binary PPM; converted to PNG by the caller with Pillow
        let mut f = std::io::BufWriter::new(std::fs::File::create(format!("{path}.ppm"))?);
        use std::io::Write;
        write!(f, "P6\n{} {}\n255\n", self.w, self.h)?;
        f.write_all(&self.data)
    }
}
