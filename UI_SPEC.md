# T-EMULATOR — UI Specification (Slint)

Hand this to whoever builds the UI. Stack: **Slint + Rust**, native GPU rendering, zero webview. Two window modes in ONE binary: `--manager` (default, windowed) and `--skin <console>` (fullscreen, borderless).

---

## 1. Design principles (non-negotiable)

- **Native feel:** real fonts (Inter/Noto Sans), real GPU animation (60fps+), no Electron look. Rounded corners tasteful (8–12px), not blobby.
- **Gamepad-first EVERYWHERE:** every screen navigable by dpad/stick. Focus rings visible. Keyboard (arrows/enter/esc) and gamepad are equal citizens; mouse secondary.
- **60fps motion:** all transitions 200–350ms, ease-out cubic. Never instant snaps. Slint `animate` on all property changes.
- **Dark-first:** base palette dark; every console skin re-tints it. No pure black (#0a0a0f range, not #000).
- **One accent per mode:** manager accent = neutral indigo; each console skin has its OWN accent (PS3 blue-white, GC purple, 360 green, Vita teal).

## 2. Palette / tokens

```
bg          #0e0e14
surface     #16161f
surface2    #1e1e2a
border      #2a2a38
text        #e8e8f0
text-dim    #8a8a9e
accent      #6e6eff   (manager)
accent-xmb  #4ea8ff   (PS3/PSP blue)
accent-gc   #6a4fff   (GC purple)
accent-360  #3ddc84   (360 green)
danger      #ff5a5a
radius      10px
font-ui     Inter / Noto Sans 13–14px body, 20–28px headings
font-clock  monospace-ish for XMB clock (real XMB shows clock top-right)
```

## 3. Screen inventory

### 3.1 MANAGER window (1200×750 default, resizable)

Left sidebar (240px) + main content.

```
┌─────────────────────────────────────────────┐
│  T-EMULATOR              🔍 search    ⚙       │
├──────────┬──────────────────────────────────┤
│ CONSOLES │   [active page content]          │
│  • PSP   │                                  │
│  • PS3   │                                  │
│  • GC/Wii│                                  │
│  + more  │                                  │
│          │                                  │
│ LIBRARY  │                                  │
│  Games   │                                  │
│  Updates │                                  │
│          │                                  │
│ SYSTEM   │                                  │
│  Emulators│                                 │
│  Firmware │                                 │
│  Controller│                                │
│  Settings │                                 │
└──────────┴──────────────────────────────────┘
```

**Pages:**

**a) Console picker (default home)** — the heart.
- Grid of large console cards (~260×160): console artwork/glow background, name, status badge (`Ready`, `Needs setup`, `Experimental`, `Not installed`)
- Hover/focus: card lifts (shadow + 1.03 scale), accent-tinted glow
- ENTER → full console-OS skin (§3.2)
- Bottom strip: "N emulators installed · M games found · updates available"

**b) Emulators page** — per-emulator row cards:
`icon | name | version installed → latest | [Update] [Settings] [Remove] | status dot`
- Top bar: "Check for updates" + "Update all" buttons
- Update state: inline progress bar inside the row (no modal!)

**c) Firmware page** — checklist:
`PS3 firmware: ✅ installed (4.91) · PSP: not needed · PS2 BIOS: ⚠ user file required [Browse…]`
- One-click "Download & install" for public firmware only
- Legal note line under each user-required item

**d) Controller page** — gamepad diagram (SVG), press-to-map, test area, per-emulator "apply to" chips. Visual button highlight when pressed.

**e) Library/Games** — all games across consoles, flat grid w/ console filter chips, cover art, "Play in <console>" button.

**f) Settings** — app prefs: theme accent, auto-update toggle, games root folders, animations on/off, launch-on-start.

**g) First-run wizard** (fullscreen, 4 steps): welcome → pick games folder → auto-detect hardware → summary "everything configured" → Enter XMB. One screen per step, big type, minimal text.

### 3.2 CONSOLE SKINS (fullscreen, borderless, Esc/Circle = exit to manager)

Each is a SEPARATE Slint component sharing a `SkinBase` (focus mgmt, nav sounds, back-stack).

**XMB (PSP/PS3/PS2 flavor) — flagship:**
```
        ← Settings  Users  Photos  [GAMES]  Video  Network →   ← categories scroll horizontally
                    │  t-emulator: focus column centered,
              🕐 clock+date top-right
                    │  vertical list under active category:
              🎮 God of War Ghost of Sparta
              🎮 Persona 3 Portable      ← focused item enlarges + shows info panel right side
              🎮 Patapon 3
   ~ wave background, animated continuously (two sine ribbons drifting) ~
```
- Wave: 2–3 translucent sine-ribbon layers, slow drift, color = accent-xmb; per-month gradient tint (real XMB behavior: hue shifts by month)
- Categories: horizontal scroll, focus snaps to center, others shrink+dim
- Items: vertical under focused category; focus = bigger icon + slide-in info panel (title, size, last played, "Press X to play")
- Sounds: nav tick, select, back — subtle, toggleable
- Boot sequence on entering skin: fade from black + coldboot animation (1–1.5s, skippable)

**GC menu skin:** channel-style tile grid, purple accent, the rotating-cube boot recreated (not ripped).
**Blades/NXE (360):** left-panel tabs + white tiles, green accent.
**Switch grid:** minimal, huge tiles, dark/light.
**PS5 home:** horizontal hero row + activity cards — build LAST (backend immature).

### 3.3 Shared components (build once)

- `FocusScope` — dpad/stick/arrows focus graph, wrap behavior
- `TileCard` — cover, title, status, focus-glow
- `InfoPanel` — game details slide-in
- `WaveBg` — parameterized sine-ribbon shader/vector (used by all Sony skins)
- `BootSplash` — console coldboot animation player
- `ProgressRow`, `Badge`, `Wizard`, `ControllerDiagram`, `UpdateToast`

## 4. Slint file layout

```
ui/
├── tokens.slint        # palette, fonts, radii (globals)
├── app.slint           # manager root window
├── pages/
│   ├── picker.slint
│   ├── emulators.slint
│   ├── firmware.slint
│   ├── controller.slint
│   ├── library.slint
│   └── settings.slint
├── wizard.slint
├── components/
│   ├── focus.slint  tilecard.slint  infopanel.slint
│   ├── wavebg.slint progressrow.slint  badge.slint
│   └── bootsplash.slint
└── skins/
    ├── xmb.slint        # takes theme param {accent, categories[]}
    ├── gcmenu.slint
    ├── blades.slint
    └── (stubs for switch/ps5/vita)
```

Skin component contract — every skin implements:
```slint
component ConsoleSkin {
    in property <[GameModel]> games;
    in property <[string]> categories;
    callback play_game(string);   // → Rust adapter.launch()
    callback exit_console();      // → back to manager
}
```

## 5. Hand-off priorities

Build order for whoever implements:
1. `tokens.slint` + `app.slint` + `picker.slint` skeleton (manager shell)
2. `wavebg.slint` + `xmb.slint` (the wave is THE asset — get it right first)
3. Focus system + gamepad input
4. Boot splash + transitions
5. Everything else pages

Wave reference for fidelity: port math from OpenXMB / xmb-web repos (in PLAN.md §3) — do NOT approximate by eye, the wave IS the product.
