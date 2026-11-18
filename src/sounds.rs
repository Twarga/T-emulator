use rodio::{Decoder, OutputStream, Sink};
use std::io::Cursor;

const TICK: &[u8] = include_bytes!("../assets/sounds/tick.wav");
const SELECT: &[u8] = include_bytes!("../assets/sounds/select.wav");
const BACK: &[u8] = include_bytes!("../assets/sounds/back.wav");

fn play(bytes: &'static [u8]) {
    let Ok((stream, handle)) = OutputStream::try_default() else { return };
    let Ok(sink) = Sink::try_new(&handle) else { return };
    if let Ok(dec) = Decoder::new(Cursor::new(bytes)) {
        sink.append(dec);
        sink.detach();
        std::mem::forget(stream);
    }
}

pub fn tick()   { play(TICK); }
pub fn select() { play(SELECT); }
pub fn back()   { play(BACK); }
