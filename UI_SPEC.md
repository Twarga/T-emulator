# T-EMULATOR — UI Specification (Slint)

Stack: **Slint + Rust**, native rendering, zero webview. ONE binary, one window.
Modes: `0` launcher → `1` power-on veil → `2` console shell.

---

## 1. Architecture

The app is a **launcher with shells**, not a manager with skins.

- **Launcher** — neutral identity: warm graphite, one ember accent, IBM Plex Sans/Mono.
  Four sections in a top tab bar: **Home · Library · Downloads · Settings**.
- **Shell** — per-console module owning its look, font, background, and logic.
  Entering one = powering on that console (boot veil → its real menu).
  PSP = XMB: procedural wave, white-on-color, Inter.
- The launcher stays deliberately colorless so each shell's color is the console's own.

Isolation rule: console shell code lives in `src/psp.rs` + `ui/psp.slint` and
knows PPSSPP specifics; launcher code (`src/launcher.rs`, `ui/launcher/*`) knows
nothing about any emulator's config.

## 2. Navigation model

Keyboard + gamepad first; mouse works as a bonus (clicks activate directly).

- **Tab bar zone** (`zone 0`): `←/→` or `Tab`/`Shift+Tab` switch sections,
  `↓`/`Enter` enters content, `Esc` quits.
- **Content zone** (`zone 1`): arrows move spatially, `Enter` activates,
  `Esc`/`Tab` lifts back to the tab bar.
- **Library search** (`zone 2`): `/` opens, type to filter live,
  `Esc` clears, `Enter`/`↓` jumps into results.
- **PSP shell**: `←/→` categories, `↑/↓` items, `Enter` activate
  (settings rows open an option sheet: `↑/↓` choose, `Enter` apply, `Esc` back).
- **Gamepad** (gilrs): dpad+stick → arrows, A → Enter, B → Esc, LB/RB → Backtab/Tab.
- While a game runs, an overlay eats every key.

Nav is a pure state machine in `ui/app.slint::nav()` — `tests/flow.rs` drives
it with real key events, no display needed.

## 3. Launcher identity

```
bg        #161411   surface #1e1a15   surface2 #262019   line #3a3229
text      #ede7dd   dim     #a89e90   faint    #6f675c
accent    #ff5a2a   (ember — focus rings + primary actions ONLY)
ok        #7dc98f   (installed/ready states)
radii     12px cards, 6px chips
fonts     IBM Plex Sans (UI), IBM Plex Mono (labels/paths/versions)
covers    typographic tiles — one of 6 muted tones picked by title hash,
          giant first-letter ghost, console tag top-left
```

- **Home**: `CONTINUE` strip (up to 3 recents) + `CONSOLES` tile row
  (4 up: PSP ready, PS3/GC/PS2 locked, greyed "Coming later").
- **Library**: responsive grid of cover tiles, `/` search pill, empty state
  with "Open games folder".
- **Downloads**: emulator rows with states Install → Update → Ready
  (progress bar inline while Busy) + planned rows marked "Later".
- **Settings**: labeled rows — games folder (zenity/kdialog picker), rescan,
  sounds / fullscreen / update-check toggles, version.

## 4. PSP shell

- Procedural `wave.rs` render (month-tinted hue, ~20 fps, only while a shell
  is on screen), XMB crossbar at `anchor-x ≈ 23.5%` / `cross-y ≈ 28.5%`.
- Categories: **Settings · Games · System** (icons: sliders/gamepad/gear).
- Settings rows carry a right-aligned **value** column backed by the real
  `~/.config/ppsspp/PSP/SYSTEM/ppsspp.ini` — backend, resolution, texture
  filtering, anisotropy, frame skip, auto-skip, OSD flags, volume
  (Fullscreen lives in our config). Writes go through `ini.rs`, which patches
  single keys and preserves the rest of the file (BOM, comments, order).
- Option sheet: dark floating list beside the focused row, check on current.

## 5. Data flow

All lists are structs in `ui/types.slint`; Rust owns every value via
`state::refresh()` — the UI is a pure function of `state::St`. Background work
(install progress, update check, game exit, folder picker, gamepad) arrives as
`Ev` events on one channel, drained by an 80 ms timer on the UI thread.

## 6. Verification

- `cargo test` — unit tests (ini patcher, settings flags, scanner, config)
  + `tests/flow.rs` (key-driven nav, headless).
- `cargo run --bin shot <screen>` — renders any screen to PNG headless:
  `home home-tabs home-empty home-locked library library-search library-empty
   downloads downloads-busy downloads-update settings boot
   psp-games psp-settings psp-system psp-sheet playing toast`
