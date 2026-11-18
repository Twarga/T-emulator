//! Tiny UI sounds, played on one long-lived audio thread so a tick never blocks the UI.
use rodio::{Decoder, OutputStream, Sink};
use std::io::Cursor;
use std::sync::mpsc::{channel, Sender};
use std::sync::OnceLock;

static TX: OnceLock<Sender<&'static str>> = OnceLock::new();

fn bytes(name: &str) -> &'static [u8] {
    match name {
        "select" => include_bytes!("../assets/sounds/select.wav"),
        "back" => include_bytes!("../assets/sounds/back.wav"),
        _ => include_bytes!("../assets/sounds/tick.wav"),
    }
}

pub fn play(name: &'static str) {
    let tx = TX.get_or_init(|| {
        let (tx, rx) = channel::<&'static str>();
        std::thread::spawn(move || {
            let Ok((_stream, handle)) = OutputStream::try_default() else { return };
            while let Ok(n) = rx.recv() {
                if let (Ok(sink), Ok(dec)) = (Sink::try_new(&handle), Decoder::new(Cursor::new(bytes(n)))) {
                    sink.append(dec);
                    sink.detach();
                }
            }
        });
        tx
    });
    let _ = tx.send(name);
}
