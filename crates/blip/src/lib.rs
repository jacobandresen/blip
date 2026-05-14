//! blip — shared arcade game library, Rust port atop macroquad.
//!
//! Mirrors the C `blip.h` API where possible. Differences:
//!   * The frame loop is owned by macroquad (`#[macroquad::main]`); games
//!     call `blip.next_frame().await` once per tick instead of the
//!     C-style `begin_frame` / `end_frame` pair.
//!   * Audio is preloaded as `BlipSound` values (async) and replayed
//!     synchronously, since macroquad's `load_sound_from_bytes` is async.

pub mod audio;
pub mod color;
pub mod ctx;
pub mod draw;
pub mod font;
pub mod input;
pub mod math;
pub mod pool;
pub mod session;
pub mod timer;
pub mod web;

pub use audio::{play_ambient, play_music, play_sfx, stop_ambient, stop_music, BlipSound};
pub use color::{
    BLIP_BLACK, BLIP_BLUE, BLIP_CYAN, BLIP_DARKGRAY, BLIP_GRAY, BLIP_GREEN, BLIP_MAGENTA,
    BLIP_ORANGE, BLIP_RED, BLIP_WHITE, BLIP_YELLOW,
};
pub use ctx::{window_conf, Blip};
pub use math::{clamp, lerp, rand_int, rects_overlap};
pub use pool::{pool_iter, pool_iter_mut, pool_spawn, Pooled};
pub use session::{LifeResult, Session};
pub use timer::Timer;

// Re-export macroquad's color::Color as BlipColor for game code.
pub use macroquad::color::Color as BlipColor;
// Re-export Texture2D as BlipTex for symmetry with the C API.
pub use macroquad::texture::Texture2D as BlipTex;

// Convenience re-export so games don't need to depend on macroquad directly
// for the few items they use most.
pub use macroquad;
