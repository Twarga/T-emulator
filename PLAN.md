# T-EMULATOR — Master Plan v1
*Unified emulator launcher: one app, every console's real OS, zero setup.*

Working name: **T-Emulator** (rename slot — final name TBD by author)

---

## 0. Vision & Constraints

- One desktop app (native, Slint/Rust — NO webview). Window = manager + console picker.
- Picking a console spawns a **fullscreen authentic OS recreation** (XMB, GC menu, PS5 home, Switch grid, blades…). User experiences "turning on a console," never an emulator window.
- Emulators are **unmodified stock binaries** launched headless (`--no-gui`, `-e`, fullscreen flags). No forks. No reverse engineering. Adapters only (~150 lines each).
- Manager side: auto-download/update from official GitHub releases, firmware wizard, per-emulator config writing, shared controller mapping, game scanning + metadata.
- **Legal line (hard, non-negotiable):** launcher + homebrew/freeware store only. No game downloads, no torrent integration, no key/firmware bundling except legally public files (Sony PS3 PUP is public on playstation.net — allowed to auto-fetch).

Dev machine: tlab (Ubuntu 24.04, 8c/30G/118G free, headless — build/CI there, test GUI on the ASUS laptop or X-forward).

---

## 1. Architecture

```
t-emulator/
├── shell/            # Slint app: console picker, settings, wizard, library mgr
├── skins/            # per-console fullscreen UI modules (Slint scenes)
│   ├── xmb/          # wave, t-emulator, gradient — serves PS3 + PSP (+ PS2 flavor)
│   ├── gcmenu/       # GameCube/Wii system menu
│   ├── switchgrid/   # Switch-style home (later)
│   └── ps5home/      # PS5 homescreen (later, when KytyPS5 matures)
├── adapters/         # one file per emulator: install/update/config/launch/compat
│   ├── ppsspp.rs
│   ├── rpcs3.rs
│   ├── dolphin.rs   # ...
├── core/             # game scanner, metadata db (sqlite), config writers,
│                     # download manager, controller abstraction, profiles
├── assets/           # fonts, sounds, icons per skin (ripped-style recreations, NOT Sony binaries)
└── profiles/         # per-game known-good config presets (community JSON)
```

**Adapter contract (the whole moat):**
```rust
trait EmulatorAdapter {
    fn install(&self) -> Result<Version>;        // fetch latest release
    fn update(&self) -> Result<Option<Version>>; // diff + swap
    fn config_dir(&self) -> Path;                // where it keeps ini/cfg
    fn write_defaults(&self, hw: &HwProfile);    // tuned config once
    fn write_game_profile(&self, game: &Game);   // per-game overrides
    fn launch(&self, game: &Path) -> Child;      // headless spawn
    fn firmware(&self) -> FirmwareStep;          // none|fetch(url)|user-required
    fn compat(&self) -> CompatLevel;             // stable|experimental
}
```

**Two-process model:**
- `t-emulator` = manager window (Slint native window). Handles install/update/settings/library.
- `t-emulator skin <console>` = same binary, fullscreen mode. Loads the skin scene, scans that console's games, launches via adapter, returns to picker on exit. Emulators' own game windows render on top — zero overhead.

---

## 2. Version Roadmap (with test games per version)

### v0.1 — PSP/XMB vertical slice
- Slint manager window: console grid (PSP tile only enabled), settings stub
- Fullscreen XMB skin (PSP flavor): wave bg, t-emulator, game tiles, gamepad+kbd nav
- PPSSPP adapter: download AppImage/zip from official releases, `ppsspp <game>` fullscreen launch
- Game scanner: watch folder `~/Games/PSP`, list ISOs on XMB
- **Test games (PPSSPP-perfect):** *God of War: Ghost of Sparta* (showcase, runs flawless), *Persona 3 Portable* (menu-heavy, good nav test), *Patapon 3* (light, fast boot test), *Crisis Core FFVII* (popular)
- Exit: Esc/Circle → back to picker. **Done = boot app → pick PSP → XMB wave → pick game → playing.**

### v0.2 — Manager core
- Download manager: GitHub release fetch, resume, verify, swap binary
- Update checker per emulator + "update all"
- Config writer: PPSSPP ini (Vulkan, 2x res, frameskip off, audio defaults from hw detect)
- Game scanner v2: SHA/ext detection, sqlite db, cover fetch (IGDB/Libretro thumbs)
- First-run wizard: pick games root folder → detect hardware → write defaults → done
- Controller: evdev/sdl mapping once → emit PPSSPP pad config

### v0.3 — RPCS3 (the flagship)
- `rpcs3` adapter: `--no-gui`, firmware wizard (auto-download official PUP from Sony's CDN, `rpcs3 --installfw`), config.yml writer (LLVM recompiler, Vulkan, res scale by GPU)
- XMB skin gains PS3 row/theme (same engine, PS3 palette + icons)
- Per-game profile system live (MGS4 needs tweaks — first real test of profiles/)
- **Test games:** *Puppeteer* (light, playable-first), *Demon's Souls* (RPCS3's flagship), *Infamous* (mid), *MGS4* (the crown jewel — hardest playable)
- Your laptop's ceiling marker: Infamous ~30fps = pass.

### v0.4 — Dolphin (GameCube/Wii)
- `dolphin` adapter: `dolphin-emu -e <game>` (or batch), Dolphin.ini writer
- **gcmenu skin**: GameCube/Wii system menu — channels-style tiles, the iconic cube boot animation (recreated, not ripped)
- Second skin proves the skin-engine abstraction
- **Test games:** *Wind Waker* (GC-menu nostalgia + light), *Twilight Princess*, *Mario Kart Wii* (couch test), *Super Mario Sunshine*

### v0.5 — The middle consoles
- **PCSX2** (PS2): adapter + reuse XMB skin w/ PS2 flavor — PCSX2 is the other "big one," `-batch`/`--nogui` launch. Test: *Shadow of the Colossus*, *GoW 2*, *Metal Gear Solid 3* (needs user-supplied BIOS — wizard handles "point me at your dump" step)
- **DuckStation** (PS1): adapter, boot-bios wizard. Test: *MGS1*, *FFVII*, *Tony Hawk 2*
- **Xenia** (360): adapter + **blades/NXE skin** (new skin). Test: *RDR1*, *Gears 2*, *Halo Reach*

### v1.0 — Ship
- Settings per emulator inside manager (mapped to each one's config files)
- Global controller profile → per-emulator translation
- Per-game profiles cloud/community repo
- Homebrew store tab (legal games only — itch/homebrew repos)
- Update-all, backup configs, import existing emulator installs
- Polish pass: animations, boot sounds, transitions

### Post-1.0 (tracked, not built until backends mature)
- **KytyPS5** + ps5home skin — when Demon's Souls-class runs stable on mid GPUs
- **Eglantine/Eden-NX2** + switchgrid skin — when OoT-remake-class games are stable
- Vita3K + LiveArea skin; Lime3DS; Flycast; mGBA/melonDS (retro row)

---

## 3. Full repo/asset inventory

**Borrow/study (license-checked):**

| Repo | License | Use |
|---|---|---|
| phenom64/OpenXMB | OSS | XMB engine logic: wave math, t-emulator layout, motion curves — port to Slint or reference |
| TheGammaSqueeze/xmb-web | OSS | Pixel-accurate XMB spec: gradients, cold-boot seq |
| dekiller82/VitaXMB | OSS | PSP-flavor XMB launcher behavior, sound/motion |
| WaveLauncher | OSS | XMB nav model on launcher |
| GabrielKLBr/RPCS3-XMBLauncher | **MIT** | Forkable PS3+XMB wiring for v0.3 |
| HiroTex/OSD-XMB (+ GorGylka HD) | GPL-3.0 | PS2-XMB variant reference |
| JohnnyCollado/PlayFieldPortal | OSS | PSP-XMB frontend patterns |
| menonparik/xmb-on-web | OSS | Simple XMB reference |

**Backends (download, never fork):**
| Emulator | Source | Launch flag |
|---|---|---|
| PPSSPP | ppsspp.org / GitHub releases | `ppsspp <game>` fullscreen flag |
| RPCS3 | rpcs3.net releases | `rpcs3 --no-gui <game>` |
| Dolphin | dolphin-emu.org | `dolphin-emu -b -e <game>` |
| PCSX2 | pcsx2.net | `pcsx2 -batch --nogui <game>` |
| DuckStation | github duckstation | `duckstation -batch <game>` |
| Xenia | xenia.jp GitHub | `xenia <game>` |
| KytyPS5/Eglantine/Vita3K | GitHub | (later, flags TBD per adapter) |

**Metadata/covers:** IGDB API, Libretro-thumbnails, PSN metadata endpoints (scraping).

---

## 4. Tech stack (locked)

- **Language:** Rust
- **UI:** Slint (native GPU UI, both manager window + fullscreen skins — one stack)
- **DB:** SQLite (games, profiles, emulator states)
- **Downloads:** reqwest + github release APIs
- **Controller:** gilrs
- **Config parsing:** ini/serde_yaml/toml per emulator's format
- **Build:** cargo workspace; CI on tlab; GUI tested on local laptop

---

## 5. Risks

1. **Skin fidelity vs. copyright** — recreate UIs (fonts, icons, wave) from scratch; don't ship Sony's ripped assets. Lookalike = fine; assets = not.
2. **Scope creep** — 18 consoles listed; only 3 in v1.0's skin work, rest are adapters+generic tile. Never build a skin for an emulator whose backend can't play yet (PS5/Switch2).
3. **Headless flags change** — isolate every flag in adapters; version-pin emulator releases tested against.
4. **tlab is headless** — all UI dev/test happens on laptop; tlab = build box/CI only.
5. **The one thing that kills it** — adding a game-download store. Stay clean.
