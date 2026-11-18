//! Procedural XMB-style background: month-tinted gradient + drifting silk sheets.
//! Rendered at low resolution on the CPU and scaled up smoothly by the UI,
//! which is what gives the wave its soft, out-of-focus look.

use slint::{Rgba8Pixel, SharedPixelBuffer};
use std::f32::consts::PI;

/// Base hue (degrees) per month, Jan..Dec. Cycles the whole wheel like the real XMB.
const MONTH_HUE: [f32; 12] = [
    225.0, 205.0, 175.0, 145.0, 95.0, 50.0, 25.0, 350.0, 320.0, 285.0, 255.0, 235.0,
];

pub fn hue_for_month(month: u32) -> f32 {
    MONTH_HUE[((month.max(1) - 1) % 12) as usize]
}

fn hsv(h: f32, s: f32, v: f32) -> [f32; 3] {
    let h = ((h % 360.0) + 360.0) % 360.0 / 60.0;
    let i = h.floor();
    let f = h - i;
    let p = v * (1.0 - s);
    let q = v * (1.0 - s * f);
    let t = v * (1.0 - s * (1.0 - f));
    match i as i32 {
        0 => [v, t, p],
        1 => [q, v, p],
        2 => [p, v, t],
        3 => [p, q, v],
        4 => [t, p, v],
        _ => [v, p, q],
    }
}

fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

struct Sheet {
    base: f32,
    amp: [f32; 2],
    freq: [f32; 2],
    speed: [f32; 2],
    phase: [f32; 2],
    thick: f32,
    thick_amp: f32,
    thick_freq: f32,
    thick_speed: f32,
    gain: f32,
}

const SHEETS: [Sheet; 3] = [
    Sheet {
        base: 0.60, amp: [0.085, 0.030], freq: [0.9, 2.1], speed: [0.050, -0.080], phase: [0.0, 1.3],
        thick: 0.20, thick_amp: 0.10, thick_freq: 1.2, thick_speed: 0.045, gain: 1.00,
    },
    Sheet {
        base: 0.68, amp: [0.070, 0.026], freq: [1.1, 2.6], speed: [-0.040, 0.070], phase: [2.1, 0.4],
        thick: 0.24, thick_amp: 0.12, thick_freq: 0.9, thick_speed: -0.050, gain: 0.80,
    },
    Sheet {
        base: 0.52, amp: [0.060, 0.022], freq: [0.7, 1.9], speed: [0.035, 0.060], phase: [4.0, 2.6],
        thick: 0.15, thick_amp: 0.08, thick_freq: 1.5, thick_speed: 0.055, gain: 0.60,
    },
];

pub fn render(w: u32, h: u32, t: f32, hue: f32, intensity: f32) -> SharedPixelBuffer<Rgba8Pixel> {
    let mut buf = SharedPixelBuffer::<Rgba8Pixel>::new(w, h);
    let px = buf.make_mut_slice();
    let (wf, hf) = (w as f32, h as f32);

    // gradient stops: dark top -> bright lower-middle -> slightly darker bottom
    let top = hsv(hue, 0.85, 0.16 * intensity.max(0.4));
    let mid = hsv(hue + 12.0, 0.72, 0.58 * intensity);
    let bot = hsv(hue + 24.0, 0.80, 0.30 * intensity);
    let tint = hsv(hue + 8.0, 0.35, 1.0);

    // per-column sheet extents
    let mut tops = vec![[0.0f32; 3]; w as usize];
    let mut bots = vec![[0.0f32; 3]; w as usize];
    for x in 0..w as usize {
        let nx = x as f32 / wf;
        for (k, s) in SHEETS.iter().enumerate() {
            let c = s.base
                + s.amp[0] * (2.0 * PI * (s.freq[0] * nx + s.speed[0] * t + s.phase[0])).sin()
                + s.amp[1] * (2.0 * PI * (s.freq[1] * nx + s.speed[1] * t + s.phase[1])).sin();
            let th = (s.thick
                + s.thick_amp * (2.0 * PI * (s.thick_freq * nx + s.thick_speed * t + s.phase[1])).sin())
            .max(0.015);
            tops[x][k] = c - th * 0.5;
            bots[x][k] = c + th * 0.5;
        }
    }

    for y in 0..h as usize {
        let ny = y as f32 / hf;
        // 3-stop vertical gradient
        let (c0, c1, f) = if ny < 0.62 {
            (top, mid, (ny / 0.62).powf(1.25))
        } else {
            (mid, bot, (ny - 0.62) / 0.38)
        };
        let base = [lerp(c0[0], c1[0], f), lerp(c0[1], c1[1], f), lerp(c0[2], c1[2], f)];

        for x in 0..w as usize {
            let nx = x as f32 / wf;
            // soft horizontal falloff so the left/right edges breathe
            let side = 0.85 + 0.15 * (PI * nx).sin();
            let mut rgb = [base[0] * side, base[1] * side, base[2] * side];
            let mut add = 0.0f32;
            for (k, s) in SHEETS.iter().enumerate() {
                let (t0, b0) = (tops[x][k], bots[x][k]);
                let u = (ny - t0) / (b0 - t0);
                let body = if (0.0..1.0).contains(&u) { (PI * u).sin().powf(1.7) } else { 0.0 };
                let e_top = (-((ny - t0) / 0.0065).powi(2)).exp();
                let e_bot = (-((ny - b0) / 0.0120).powi(2)).exp();
                add += s.gain * (0.115 * body + 0.46 * e_top + 0.24 * e_bot);
            }
            let a = (add * intensity).min(0.95);
            for c in 0..3 {
                rgb[c] = rgb[c] * (1.0 - a * 0.35) + tint[c] * a;
            }
            // vignette
            let vx = (nx - 0.5) * 2.0;
            let vy = (ny - 0.5) * 2.0;
            let vig = 1.0 - 0.28 * (vx * vx * 0.6 + vy * vy * 0.4);
            let o = &mut px[y * w as usize + x];
            *o = Rgba8Pixel {
                r: (rgb[0] * vig * 255.0).clamp(0.0, 255.0) as u8,
                g: (rgb[1] * vig * 255.0).clamp(0.0, 255.0) as u8,
                b: (rgb[2] * vig * 255.0).clamp(0.0, 255.0) as u8,
                a: 255,
            };
        }
    }
    buf
}
