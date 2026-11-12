# The blip engine

`crates/blip` is a small layer over [macroquad](https://macroquad.rs) shared by every game. A game keeps its rules, art and
sound in its own crate and uses `blip` for the rest:

- a fixed virtual canvas, scaled to the window, with an optional CRT look;
- drawing, a 5×7 pixel font, the shared HUD and title/game-over screens;
- sound playback and generated music;
- score, lives and level (`Session`), timers, particles and popups;
- the bridge to the web page (coins, players, touch, best score);
- a playtest bot toolkit (`blip::bot`).

The same code runs natively and as WebAssembly; web-only calls do nothing natively.

## A game

```rust
use blip::{window_conf, Blip, Session, BLIP_BLACK};

fn conf() -> blip::macroquad::window::Conf { window_conf("SERPENT", 480, 540) }

#[blip::macroquad::main(conf)]
async fn main() {
    let mut blip = Blip::new(480, 540);
    let eat = blip::sound!("eat");  // built by build.rs
    let mut g = Game::new();
    loop {
        g.update(blip.delta_time, &eat);   // seconds since the last frame, at most 0.1
        blip.clear(BLIP_BLACK);
        g.draw(&blip);
        blip.next_frame(60).await;
    }
}
```

Games keep their screens in a `State` enum (`Title`, `Play`, `Over`, …) with an `update_*` and a `draw_*` function each.

## Assets

`build.rs` is `blip::build_script!();`. It runs the game's `src/gfx.rs` (`generate()`, pictures) and `src/tunes.rs`
(`sounds()` and `music()`) at build time:

- pictures and effects are embedded: `load_png(blip::png!("food"))`, `blip::sound!("eat")`;
- music is written to `web/<game>/music/<NAME>.wav` and streamed: `Jukebox::new("serpent", &["SLITHER", "FRENZY"])`.

`blip::image`, `blip::wav`, `blip::song`, `blip::cosy` and `blip::techno` are the generators the asset files build on.

## Drawing

Coordinates are canvas pixels from the top left.

```rust
blip.fill_rect(x, y, w, h, BLIP_RED);
blip.draw_line(x1, y1, x2, y2, BLIP_GRAY);
blip.fill_circle(cx, cy, r, BLIP_YELLOW);
blip.draw_texture(&tex, x, y, w, h);

blip.draw_text("GAME OVER", x, y, 3.0, BLIP_YELLOW);  // size: pixels per font pixel
blip.draw_centered("READY", y, 2.0, BLIP_WHITE);
blip.draw_hud(g.sess.score, g.sess.lives);            // SCORE, best, LIVES
blip.draw_start_prompt(y);                            // PRESS SPACE / FIRE / TAP TO START
blip.draw_game_over(score, web::high_score(), headline, best, prompt, ready);
```

Use whole-number text sizes, 2 or more where it must read on a phone.

## Input and sound

```rust
use blip::input::{key_held, key_pressed, BLIP_KEY_LEFT, BLIP_KEY_SPACE};
if key_held(BLIP_KEY_LEFT) { x -= SPEED * dt; }
if key_pressed(BLIP_KEY_SPACE) { fire(); }

blip::play_sfx(&eat);
blip::play_ambient(&wind);   // loops, under the music
```

On the web the deck, touch strip and gamepad arrive as the same key events.

## The web page

```rust
use blip::web;
web::spend_coin();           // a game started
web::set_mode(true);         // two players
web::report_score(score);    // game over: the page keeps the browser's best
web::high_score();           // that best, 0 before one exists
web::controls();             // keyboard, deck or touch, for prompts
```

## The control deck

`web/deck.js` and `web/deck.css` draw the cabinet's deck (two stations, each a ball-top stick or a pad and two
caps) and turn it into input. Blip's pages build it from `BLIP_GAMES`; another page can host it too (breadbin does):

```js
// <div id="topbar" class="kiosk-bar"><div class="deck-panel"></div></div>, and <html data-layout="upright"
// data-controls="stick"> (data-has-touch on touch screens)
BlipDeck.build(panel, { buttons: [{ label: 'FIRE' }], seats: 1 });
BlipController.init({ click: false, onInput: (name, down) => { /* 'up' … 'right', 'button1', 'button2' */ } });
BlipController.bindButtons(document.getElementById('fire-buttons'));
const stop = BlipDeck.bindStick({ stick: document.getElementById('stick-base'),
  dirs: { up: 'up', down: 'down', left: 'left', right: 'right' } });
BlipDeck.lean(stick, x, y);   // mirror held directions on the stick, from onInput
```

## Build and run

```sh
cargo run --release -p serpent   # native window
npm run build:web                # every game to web/<game>/index.wasm
node tools/serve_web.mjs         # serve web/
```
