# Dev notes

Build:        cargo build --release        (binary: target/release/t-emulator)
Run:          ./t-emulator
Test:         cargo test                   (drives the real UI with key events, no display)
Screenshots:  cargo build --bin shot && ./target/debug/shot <launcher|launcher2|boot|boot2|xmb|xmb-settings|toast> out.png [w h]
              (writes out.png.ppm; the UI is rendered headless so any screen can be inspected without a display)

State:        ~/.config/t-emulator/config.toml
Emulators:    ~/.local/share/t-emulator/emulators/ppsspp/
Games:        ~/Games/PSP  (.iso .cso .pbp)
