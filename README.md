# BLIP ARCADE

Four classic arcade games running in the browser — written in Rust on
[macroquad](https://macroquad.rs) and compiled to WebAssembly.

**[Play in the browser →](https://jacobandresen.github.io/blip/)**

---

## The games

**Rally** — Keep the ball in play. Don't let it past your paddle.

**Serpent** — Guide the snake, eat the food, don't bite yourself.

**Bouncer** — Break every brick. Don't let the ball fall.

**Galactic Defender** — Shoot the aliens before they reach the ground.

---

## Controls

| | |
|---|---|
| Move | WASD or arrow keys |
| Shoot / launch | Space |

---

## Building

Native (each game is its own crate):

```
cargo run -p serpent
cargo run -p bouncer
cargo run -p rally
cargo run -p galactic_defender
```

Web (all four games at once):

```
rustup target add wasm32-unknown-unknown   # one-time
./build_web.sh
python3 -m http.server -d web 8080
```

The script writes `web/<game>/index.wasm` and `web/<game>/index.html`. The
macroquad JavaScript runtime is vendored at `web/mq_js_bundle.js`.

---

## About

Made by [Jacob Andresen](https://mastodon.gamedev.place/@jacobandresen) as an
experiment with [macroquad](https://macroquad.rs) and Rust on
`wasm32-unknown-unknown`.

Built in collaboration with [Claude](https://claude.ai) (Anthropic).
