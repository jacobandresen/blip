# Blip Library Guide

A friendly walkthrough of the `blip` crate for anyone who wants to build a new game or understand how the existing ones work. You'll need basic Rust familiarity, but no macroquad experience.

---

## What blip is

`blip` is a thin arcade-game library built on top of [macroquad](https://macroquad.rs). It handles three things so your game code doesn't have to:

- **Rendering** — draws everything to a fixed-size virtual canvas (e.g. 480×540 pixels), then scales it up with letterboxing to fit any window. A CRT post-process (scanlines, occasional glitch effects) is applied automatically.
- **Frame pacing** — you call `blip.next_frame(60).await` once per tick; delta-time is available as `blip.delta_time`.
- **Platform shims** — the same code runs natively and as WebAssembly. Hi-score calls go to Supabase in the browser and are no-ops on the desktop.

Everything else — game logic, state, entities, physics — lives entirely in your game crate.

---

## The game loop

Every game in this repo follows this skeleton:

```rust
use blip::{window_conf, Blip, BLIP_BLACK};

fn conf() -> blip::macroquad::window::Conf {
    window_conf("MY GAME", 480, 540)
}

#[blip::macroquad::main(conf)]
async fn main() {
    let mut blip = Blip::new(480, 540);  // virtual canvas size
    let mut g    = Game::new();           // your game state

    loop {
        let dt = blip.delta_time;         // seconds since last frame

        // 1. Update game logic
        update(&mut g, dt);

        // 2. Draw everything
        blip.clear(BLIP_BLACK);
        draw(&blip, &g);

        // 3. Hand control back to macroquad, wait for next frame
        blip.next_frame(60).await;
    }
}
```

`Blip::new(w, h)` creates the render target. After `blip.next_frame().await` returns, `blip.delta_time` holds the time elapsed since the previous frame (capped at 0.1 s to survive focus loss).

---

## The state machine convention

All games use a simple state enum to separate their screens. The pattern is:

```rust
#[derive(Copy, Clone, PartialEq, Eq)]
enum State { Title, Play, Dead, Win, Over }

// In the main loop:
match g.state {
    State::Title => update_title(&mut g),
    State::Play  => update_play(&mut g, dt, &sfx),
    State::Dead  => update_dead(&mut g, dt),
    State::Win   => update_win(&mut g, dt),
    State::Over  => update_over(&mut g),
}

blip.clear(BLIP_BLACK);
match g.state {
    State::Title => draw_title(&blip),
    State::Play  => draw_play(&blip, &g),
    // ...
}
```

Each `update_*` function mutates the `Game` struct and transitions `g.state` when appropriate. Each `draw_*` function reads the `Game` struct and calls blip drawing methods. The separation keeps game logic and rendering easy to follow independently.

---

## Modules at a glance

### `blip::input` — reading the keyboard

```rust
use blip::input::{any_key_pressed, key_held, key_pressed,
                   BLIP_KEY_LEFT, BLIP_KEY_RIGHT, BLIP_KEY_SPACE};

// Held down this frame (good for movement):
if key_held(BLIP_KEY_LEFT) { player.x -= SPEED * dt; }

// Pressed this frame only (good for jumping, firing):
if key_pressed(BLIP_KEY_SPACE) { player.jump(); }

// Any key at all — handy for "press any key to continue":
if any_key_pressed() { g.state = State::Play; }
```

Available key constants: `BLIP_KEY_UP/DOWN/LEFT/RIGHT`, `BLIP_KEY_W/A/S/D`, `BLIP_KEY_SPACE`. Touch input from the web shell is automatically translated into these same key events.

### `blip::audio` — loading and playing sounds

Sounds must be loaded at startup (they're async), then stored and replayed synchronously during the game loop:

```rust
// At startup — load from embedded bytes:
const JUMP_WAV: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/assets/sounds/jump.wav"));
let jump_sfx = blip::audio::load_sound(JUMP_WAV).await;

// Or synthesize a beep on the fly (no WAV file needed):
let blip_sfx = blip::audio::beep(440.0, 80.0).await;  // 440 Hz, 80 ms

// During the game loop — free to call every frame:
blip::play_sfx(&jump_sfx);
blip::play_music(&music);    // loops at background volume
blip::play_ambient(&wind);   // loops quieter than music
```

### `blip::draw` — shapes and textures

All coordinates are in virtual-canvas pixels. The canvas origin (0, 0) is the top-left corner.

```rust
blip.fill_rect(x, y, w, h, BLIP_RED);          // filled rectangle
blip.draw_rect(x, y, w, h, BLIP_WHITE);         // outline only
blip.draw_line(x1, y1, x2, y2, BLIP_GRAY);
blip.fill_circle(cx, cy, radius, BLIP_YELLOW);
blip.draw_texture(&tex, x, y, w, h);            // stretch texture to fit w×h
blip.draw_texture_tinted(&tex, x, y, w, h, tint);
```

### `blip::font` — bitmap text

Text uses a built-in 5×7 pixel bitmap font. The `sz` parameter is a pixel scale multiplier (`sz=2.0` → 10×14 rendered pixels per glyph). Supported characters: `A-Z`, `0-9`, space, `!`, `:`, `-`, `.`.

```rust
blip.draw_text("GAME OVER", x, y, 3.0, BLIP_YELLOW);   // left-aligned
blip.draw_centered("PRESS ANY KEY", y, 2.0, BLIP_GRAY); // horizontally centred
blip.draw_number(score, x, y, 2.0, BLIP_WHITE);         // i32 shorthand
```

The standard HUD (SCORE / HI / LIVES across the top) is one call:

```rust
blip.draw_hud(g.score, g.hi_score, g.lives);
```

### `blip::color` — the palette

Ten named colours, matching the original C API:

| Constant | Appearance |
|----------|-----------|
| `BLIP_BLACK` | Background black |
| `BLIP_WHITE` | Pure white |
| `BLIP_RED` | Warm red |
| `BLIP_GREEN` | Arcade green |
| `BLIP_BLUE` | Royal blue |
| `BLIP_CYAN` | Bright cyan |
| `BLIP_MAGENTA` | Magenta/pink |
| `BLIP_YELLOW` | Score yellow |
| `BLIP_ORANGE` | HUD orange |
| `BLIP_GRAY` | Mid grey |
| `BLIP_DARKGRAY` | Dark separator grey |

You can define your own colours directly via `macroquad::color::Color { r, g, b, a }` (all values 0.0–1.0).

### `blip::math` — collision and utilities

```rust
use blip::{clamp, lerp, rand_int, rects_overlap};

// AABB collision — returns true if the two rectangles overlap:
if rects_overlap(ball.x, ball.y, BALL_W, BALL_H,
                 brick.x, brick.y, BRICK_W, BRICK_H) { ... }

rand_int(1, 6)   // inclusive random integer in [1, 6]
clamp(v, 0.0, 1.0)
lerp(a, b, 0.5)  // midpoint between a and b
```

### `blip::web` — hi-scores and kiosk

These calls talk to the browser's JavaScript bridge when running as WASM. On the desktop they do nothing, so your game logic works unchanged in both environments.

```rust
use blip::web;

// Most games use Session, which threads game_id for you:
let mut sess = Session::new(web::GAME_CANARIS, 3);  // 3 lives
sess.add_score(100);          // auto-saves hi if beaten
sess.refresh_hi();            // re-poll the global hi
sess.notify_game_over();      // tells kiosk the session ended

// Low-level web calls are still available if you need them:
let hi = web::load_hi_score(web::GAME_CANARIS);
if score > hi { web::save_hi_score(web::GAME_CANARIS, score); }
web::game_over(web::GAME_CANARIS, score);

// Charge a coin when starting a new game in kiosk mode:
web::spend_coin();
```

Available game ID constants: `GAME_BOUNCER`, `GAME_SERPENT`, `GAME_GALACTIC_DEFENDER`, `GAME_CANARIS`.

---

## Running a game natively

```bash
cargo run -p rally
cargo run -p serpent
# etc.
```

Native builds show a resizable window with the CRT effect. Hi-score calls are no-ops, so the game always starts from zero.

## Building for the web

```bash
rustup target add wasm32-unknown-unknown   # one-time setup
./build_web.sh                             # compiles all games → web/
python3 -m http.server -d web 8080         # serve locally
```

Each game is compiled to its own `.wasm` + `.js` pair under `web/<gamename>/`.
