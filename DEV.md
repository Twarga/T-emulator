# Dev notes

Build:        cargo build --release        (binary: target/release/t-emulator)
Run:          ./target/debug/t-emulator
Test:         cargo test                   (unit tests + key-driven UI nav, no display)
Screenshots:  cargo run --bin shot <screen> out.png [w h]
              screens: home home-tabs home-empty home-locked library library-search
                       library-empty downloads downloads-busy downloads-update
                       settings boot psp-games psp-settings psp-system psp-sheet
                       playing toast

State:        ~/.config/t-emulator/config.toml   (games_dir, sounds, fullscreen,
              update_check, recents, emu_versions)
Emulators:    ~/.local/share/t-emulator/emulators/ppsspp/PPSSPP.AppImage
PPSSPP conf:  ~/.config/ppsspp/PSP/SYSTEM/ppsspp.ini  (XMB settings write here
              via src/ini.rs — patches one key, preserves the file)
Games:        ~/Games/PSP  (.iso .cso .pbp)

Layout:       src/launcher.rs  data builders for launcher pages
              src/psp.rs       PSP shell: XMB content + ppsspp.ini schema
              src/ini.rs       surgical ini editor (BOM/comments/CRLF-safe)
              src/state.rs     St + Ev + refresh() — UI is a pure function of St
              src/adapters/    emulator adapters (ppsspp)
              src/pads.rs      gamepad auto-config: merges generic-pad binds
                               into controls.ini + ships SDL gamecontrollerdb
              src/covers.rs    box art lookup + libretro-thumbnails download
              ui/app.slint     root: modes, nav state machine, overlays
              ui/launcher/     launcher pages (theme, widgets, topbar, home,
                               library, downloads, settings)
              ui/psp.slint     PSP shell (XMB + option sheet)
