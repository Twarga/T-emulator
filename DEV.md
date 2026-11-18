# T-Emulator — Dev / Run Notes

## Build (tlab)
ssh tlab 'source ~/.cargo/env && cd ~/t-emulator && cargo build --release'
scp tlab:~/t-emulator/target/release/t-emulator ./t-emulator

## Run (laptop)
./t-emulator-v0.1
- Pick PSP → Enter (bootsplash → XMB wave → games)
- Games live in `~/Games/PSP/` (.iso/.cso/.pbp)
- First power-on auto-downloads PPSSPP AppImage (~40MB) in background
- Esc in XMB = back to picker

## State
- Config: `~/.config/t-emulator/config.toml`
- Emulator: `~/.local/share/t-emulator/emulators/ppsspp/`
