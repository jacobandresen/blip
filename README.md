# Blip Engine Arcade
Nine arcade games built with the Rust `blip` engine and compiled to WebAssembly.

**[Play in the browser →](https://jacobandresen.github.io/blip/)**

**[Explore the Blip API →](https://jacobandresen.github.io/blip/api.html)** — see how the games use the engine, then try making one with Rust or an AI coding agent such as Claude Code or Codex.

## Games

- **Rally** — table tennis for one or two players.
- **Serpent** — guide a snake through a maze.
- **Adder** — two vipers, and CPU ones to fill the pit: free movement, holes in the bodies to pass through.
- **Bouncer** — brick breaker with pickups and multi-ball.
- **Defender** — shoot the alien formation and its motherships.
- **Meteors** — steer and fire through drifting asteroids.
- **Raider** — vertical shooter with formations, flak and bosses.
- **Brawler** — one-on-one fighting with ten fighters and a nine-fight ladder.
- **Bubbler** — bubble-blowing platformer for one or two players.

---

## 🚀 Getting Started

### Native development

Install Rust and Cargo, then build or run a game:

```bash
cargo build --release
cargo run -p rally
```

See the [Blip API page](https://jacobandresen.github.io/blip/api.html) or [source guide](docs/blip.md).

### Web build

```bash
rustup target add wasm32-unknown-unknown
./build_web.sh
python3 -m http.server -d web 8080
```

The build places each game's `index.wasm` in `web/<game>/`; the shared shell and macroquad runtime are served from `web/`.

### Deployment and previews

GitHub Pages serves the `gh-pages` branch. A push to `main` applies new database migrations, then publishes `web/` to the branch's root ([pages.yml](.github/workflows/pages.yml)); the site is at <https://jacobandresen.github.io/blip/>.

Each pull request from a branch of this repository gets a preview at `https://jacobandresen.github.io/blip/pr-preview/pr-<number>/`, linked in a comment on the pull request and removed when it closes ([preview.yml](.github/workflows/preview.yml)). Previews have no high scores and no offline cache, and share saved settings (coins, sound, home-screen order) with the live site. Pull requests from forks get no preview.

### Kiosk controls

Use the stick or arrows to select a game and fire to start. Fire, Enter or `5` inserts a coin; Backspace returns to the cabinet; `M` toggles sound. The fullscreen choice persists across pages, and idle games return to the cabinet.

See [CONTRIBUTING.md](CONTRIBUTING.md) before submitting changes.
