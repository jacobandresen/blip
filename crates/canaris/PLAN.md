# Canaris — Game Design & Implementation Plan

## Overview

Canaris is an arcade adaptation of the 1985 Danish DOS game *Kaptajn Kaper i Kattegat* (Captain Caper in the Kattegat) by Peter Ole Frederiksen. The player is a privateer captain with a royal letter of marque, tasked with raiding English merchant ships in the Kattegat strait. The game blends side-scrolling naval navigation, cannon combat, boarding actions, and harbour resource management.

Built on the **blip** framework (Rust + macroquad), targeting both native desktop and browser (WASM).

---

## Source References

- Original game: https://github.com/kb-dk/KaptajnKaper
- Game description (Danish): https://da.wikipedia.org/wiki/Kaptajn_Kaper_i_Kattegat

---

## Blip Framework Gaps

The following features are needed but absent from blip core. All are bridged locally in canaris without modifying blip.

| Feature | Gap | Local Solution |
|---|---|---|
| Horizontal scrolling | No camera/viewport system | `cam_x: f32` offset subtracted from every world-X before drawing |
| Sprite animation | No frame cycling helper | `anim_frame: u8` + `anim_t: f32` per entity; separate textures per frame |
| Source-rect drawing | `draw_texture` has no sub-region support | Private wrapper calling `blip::macroquad::texture::draw_texture_ex` with `DrawTextureParams { source: Some(Rect) }` |
| Extended key input | Only arrows/WASD/Space exposed | Import `blip::macroquad::input::KeyCode`; local constants `KEY_BOARD = KeyCode::E`, `KEY_ENTER = KeyCode::Enter`, `KEY_1–4` |
| Multiple music tracks | Single-track API | `play_music(&assets.X)` on each state transition (blip already stops previous) |

---

## Game State Machine

```
Title ──[any key]──► Sea ──[enemy near]──► Combat
                      ▲                       │
                      │◄── enemy sunk ────────┘
                      │◄── retreat timer ──────┘
                      │
                     Sea ──[reach port]──► Port ──[SET SAIL]──► Sea
                      │
                     Sea ──[hull=0]──► Dead ──[lives>0]──► Sea
                                          └──[lives=0]──► GameOver ──[any key]──► Title
```

States: `Title | Sea | Combat | Boarding | Port | Dead | GameOver`

---

## Core Data Model

```
Game
 ├── player: PlayerShip        (world_x, y, hull, crew, gold, cannons, food, reload_t)
 ├── cam_x: f32                (camera world-X offset)
 ├── enemies: [EnemyShip; 4]   (world_x, y, hull, crew, gold_loot, engaged, reload_t)
 ├── cannonballs: [Cannonball; 8]  (x, y, vx, vy, player: bool)
 ├── explosions: [Explosion; 12]   (x, y, ttl, scale)
 ├── slots: [BoardingSlot; 6]  (boarding minigame: owner, hp)
 └── port_menu: PortMenu       (cursor, message)
```

Window: **480 × 540**. World width: **4 × screen = 1920** (wraps seamlessly).

---

## Asset List

### Sprites (PNG, RGBA8, procedurally generated)

| Asset | Size | Description |
|---|---|---|
| `player_ship_a/b.png` | 48×32 | Player privateer, 2-frame sail anim, cyan hull |
| `enemy_ship_a/b.png` | 48×32 | Enemy merchant, 2-frame, brown hull + English flag |
| `cannonball.png` | 8×8 | Dark sphere with highlight |
| `explosion.png` | 32×32 | Orange/yellow burst |
| `port_bg.png` | 480×512 | Harbour scene (fill_rect primitives) |
| `sea_wave.png` | 120×40 | Tiling wave strip |
| `crew_figure.png` | 12×20 | Silhouette (tinted CYAN or RED at draw time) |

### Sounds (WAV, 16-bit PCM mono 44.1 kHz, procedurally generated)

| Asset | Duration | Description |
|---|---|---|
| `cannon_fire.wav` | 120ms | Noise burst, low-end |
| `explosion.wav` | 300ms | Noise decay |
| `splash.wav` | 80ms | Low-freq miss |
| `hull_hit.wav` | 60ms | 150 Hz square thud |
| `boarding_clash.wav` | 40ms | 800 Hz metallic ping |
| `coin_jingle.wav` | 240ms | 3-note arpeggio D5→F#5→A5 |
| `life_lost.wav` | 400ms | Descending glissando 440→110 Hz |
| `sea_music.wav` | ~8s loop | Minor key, square-wave bass, 110 bpm |
| `combat_music.wav` | ~4s loop | Tense, dissonant, 140 bpm |
| `port_music.wav` | ~6s loop | Warm major key, 90 bpm |

---

## Implementation Passes

### Pass 1 — Scaffold & Assets
*Goal: `cargo run -p canaris` opens a window. All asset byte arrays embedded and non-zero.*

- [x] Add `"crates/canaris"` to workspace `Cargo.toml`
- [x] Create `canaris/Cargo.toml` (mirrors galactic_defender)
- [x] Create `canaris/build.rs` (calls `blip_assets::canaris::generate()`)
- [x] Create stub `canaris/src/main.rs` (blank black window, state loop skeleton)
- [x] Add `pub mod canaris;` to `blip_assets/src/lib.rs`
- [x] Create `blip_assets/src/canaris.rs` with `generate()` returning all 9 images + 10 sounds

**Verify:** Compiles to WASM and native. Window opens.

---

### Pass 2 — Title & Sea Navigation
*Goal: Player ship scrolls across a tiling sea background. Camera follows. Enemies appear and move.*

- [x] Title screen: background, ship sprite, text, hi-score, any-key-to-start
- [x] `Game::start_game()` initialises all resources and state
- [x] Sea update: player movement (arrows/WASD + drag), vertical bob
- [x] Camera: `cam_x` tracking with world clamp
- [x] Background: tiled `sea_wave` with parallax offset, sky + horizon layers
- [x] Enemy ships: spawn off right edge, drift left, bob, respawn when off-screen
- [x] Food decay over time; hull damage when food = 0
- [x] State transitions: engagement trigger (enemy near) → Combat; port trigger → Port
- [x] Level timer: level-up every 60s+, harder enemies each level

**Verify:** Ship moves, camera follows, enemies scroll past, title→sea→(placeholder combat/port) transitions work.

---

### Pass 3 — Combat
*Goal: Fixed-screen cannon battle with cannonballs, hit detection, win/loss/retreat.*

- [x] Position player (left) and enemy (right) on enter, player Y snapped to COMBAT_BASE_Y
- [x] Player: UP/DOWN dodge, SPACE to fire cannonballs
- [x] Enemy AI: oscillating dodge pattern + aimed fire toward player Y
- [x] Cannonball physics: linear travel + gravity arc
- [x] Hit detection: `rects_overlap` → 2 hull damage, explosion, hit flash, sfx
- [x] Win: enemy hull ≤ 0 → gold + score → back to Sea
- [x] Retreat: timer expires → enemy displaced far in world space → back to Sea
- [x] Player death: hull ≤ 0 → State::Dead
- [x] KEY_BOARD when Y proximity < BOARD_Y_DIST → State::Boarding
- [x] Muzzle flash (player + enemy) for 80ms on fire
- [x] Bottom UI: colour-coded hull bars + retreat timer bar + contextual hint row
- [x] No-ammo warning blinks when cannons = 0

**Verify:** Ships fire at each other; hull decrements; sinking and retreat both return to Sea.

---

### Pass 4 — Boarding Minigame
*Goal: 6-slot crew combat screen. Both sides attack; player reinforces with SPACE.*

- [x] Render 6 horizontal slots: player crew left, enemy right
- [x] `crew_figure` tinted CYAN (player) / RED (enemy) per slot, 2× scale on ship-deck scene
- [x] Auto-tick: enemy attacks rightmost player slot (captures it on kill)
- [x] SPACE: attack leftmost enemy slot (destroy on kill → Empty)
- [x] Win: all enemy slots cleared → gold×2 + score → Sea
- [x] Loss: all player slots cleared → Dead
- [x] Timeout: draw → back to Sea, no loot
- [x] Hit flash: yellow highlight on attacked slot for 0.28s
- [x] Timeout bar with countdown, crew-count labels, [SPACE] ATTACK hint

**Verify:** Boarding resolves correctly in all three outcomes.

---

### Pass 5 — Port / Resource Management
*Goal: Harbour docking screen with menu-driven purchases.*

- [x] Render `port_bg` + menu panel
- [x] Player stats strip above menu: hull bar + crew/food/guns inline
- [x] Menu items: Repair / Hire Crew / Buy Cannons / Buy Food / Set Sail
- [x] Unaffordable items dimmed (DARKGRAY), cost shown dimmed too
- [x] UP/DOWN or W/S to move cursor; Enter or Space to confirm
- [x] Purchase: deduct gold if sufficient, apply effect, show message
- [x] Status message color-coded: green = success, red = insufficient gold
- [x] "Set Sail": transition back to Sea, play sea_music

Prices: Repair=30g, Crew=20g, Cannons=25g, Food=15g.

**Verify:** All 4 purchases work; gold cannot go negative; Set Sail returns to sea.

---

### Pass 6 — HUD, Score, Level Progression, Polish
*Goal: Full arcade loop with scoring, level scaling, death/respawn, and visual polish.*

- [x] HUD: `draw_hud_canaris` — score/hi/lives top bar + HULL/GOLD/FOOD/GUNS/LV second row
- [x] Score increments: sunk=200×level, boarded=150×level+loot×2
- [x] Level-up: `level_t` timer expires → level+1, spawn harder enemies
- [x] Dead state: red flash + fade-out overlay, respawn (lives>0) or GameOver
- [x] GameOver: final score, hi-score, press-any-key → Title + `web::spend_coin()`
- [x] Ship bob: `sin(time × BOB_FREQ) × BOB_AMP` in sea and title screens
- [x] Explosion scaling: grows over lifetime (`1.0 + (1-t)*0.7` multiplier)
- [x] Hit flash: `draw_texture_tinted(tint_white())` while `hit_flash_t > 0`
- [x] Music transitions: sea ↔ combat ↔ port on state changes
- [x] All sounds wired to events
- [x] Game card in `web/index.html`: Danish flag badge 🇩🇰 + keyboard badge ⌨
- [x] `web/canaris/index.html` shell page
- [x] `web/canaris/screenshot.png` placeholder

**Verify:** Full game loop playable. Die, respawn, hit hi-score, game over, restart. WASM build works.

---

## File Structure

```
crates/canaris/
├── PLAN.md              ← this file
├── Cargo.toml
├── build.rs
└── src/
    └── main.rs          ← entire game

crates/blip_assets/src/
├── lib.rs               ← add: pub mod canaris;
└── canaris.rs           ← new: all asset generation
```

---

## Structural Invariants

- All entity pools fixed-size arrays (`[T; N]`) — no hot-path heap allocation
- `cam_x` clamped so player sprite is always on screen
- `gold` and `food` clamped ≥ 0 in every update
- `play_music` called exactly once per state transition requiring a music change
- No `std::fs`, `std::thread`, `std::env` (WASM safety)
- All assets embedded via `include_bytes!` at compile time
