# Blip Engine Arcade
A vibrant showcase of classic arcade games powered by the open-source Blip Engine.

> **Note:** This project is an experiment in AI-assisted game development. The games, engine, and supporting code were built collaboratively with AI tools.

## 👋 Welcome

The Blip Engine is an open-source framework designed to provide a standardized, high-performance layer for arcade game development in Rust. By compiling games into WebAssembly, we abstract away the complexities of low-level web graphics programming, allowing developers to focus purely on game logic, state management, and fun.

**Blip Arcade is built as a tribute to the golden age of arcade games.** This project is not intended for commercial use.

**[Play in the browser →](https://jacobandresen.github.io/blip/)**

---

## 🕹️ Game Modules

This collection features several classic titles, all running within the unified Blip Engine:

*   **Rally**: Keep the ball in play and keep your score high.
*   **Serpent**: Guide the snake through the maze, eat the pellet, and avoid self-collision.
*   **Bouncer**: The ultimate brick-breaking challenge.
*   **Galactic Defender**: Shoot down the endless swarm of invading aliens.
*   **Canaris**: A tribute to the Danish DOS game Kaptajn Kapers (1985) — never an arcade game, but lovingly reimagined as one here.

---

## ⚙️ Technology & Architecture

The Blip Engine ensures portability and high performance across all modules:

*   **Framework:** Blip Engine (Open-Source, Written in Rust)
*   **Output:** The primary compiled assets are delivered via WebAssembly (WASM), ensuring fast, consistent performance in modern web browsers.
*   **Architecture:** Game state management is separated from the core rendering loop, promoting modularity and scalability.
*   **Hi-scores:** Global hi-scores are persisted across sessions and shared between players via [Supabase](https://supabase.com) (Postgres). Scores are read on page load and saved automatically when a record is beaten.

---

## 🚀 Getting Started

### Development Setup
To explore development or contribute:

1.  **Prerequisites:** Ensure Rust and Cargo are installed.
2.  **Build:**
    ```bash
    cargo build --release
    ```
    *This compiles the core engine and individual game modules.*
3.  **Library guide:** See [docs/blip.md](docs/blip.md) for a walkthrough of the blip API — what each module does, the game loop pattern, and how to get started writing a new game.

### Web Distribution (Production)
To compile all games for the web:

```bash
# Step 1: Ensure the WASM target is installed (One-time setup)
rustup target add wasm32-unknown-unknown

# Step 2: Build all games into the web directory
./build_web.sh

# Step 3: Run a simple local server to view the compiled application
python3 -m http.server -d web 8080
```

---

## 🙏 Support & Contributing

### Contributing
The engine is open-source! We welcome contributions from the community. Please check the `CONTRIBUTING.md` file for detailed guidelines.

