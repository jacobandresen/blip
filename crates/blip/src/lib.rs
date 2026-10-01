//! blip — the shared arcade game library, on macroquad.
//!
//! macroquad owns the frame loop (`#[macroquad::main]`): games call
//! `blip.next_frame().await` once per tick. Audio is preloaded as
//! `BlipSound` values at startup (loading is async) and replayed
//! synchronously in the loop.

pub mod audio;
pub mod bot;
pub mod color;
pub mod ctx;
pub mod draw;
pub mod font;
pub mod fx;
pub mod input;
pub mod math;
pub mod pool;
pub mod session;
pub mod timer;
pub mod web;

pub use audio::{
    play_alert, play_ambient, play_music, play_sfx, play_sfx_volume, stop_alert, stop_ambient, stop_music,
    BlipSound, Jukebox,
};
pub use color::{
    hsv,
    BLIP_BLACK, BLIP_BLUE, BLIP_CYAN, BLIP_DARKGRAY, BLIP_GRAY, BLIP_GREEN, BLIP_MAGENTA,
    BLIP_ORANGE, BLIP_RED, BLIP_WHITE, BLIP_YELLOW, NEON_CYAN, NEON_GREEN, NEON_MAGENTA,
    NEON_ORANGE, NEON_PINK, NEON_PURPLE, NEON_YELLOW,
};
pub use ctx::{window_conf, Blip};
pub use draw::{load_png, load_png_smooth};
pub use fx::Fx;
pub use math::{clamp, lerp, rand_int, rects_overlap, rand_seed};
pub use pool::{pool_iter, pool_iter_mut, pool_spawn, Pooled};
pub use session::{LifeResult, Session, GAME_OVER_MIN_WAIT};
pub use timer::Timer;

// Re-export macroquad's color::Color as BlipColor for game code.
pub use macroquad::color::Color as BlipColor;
// Re-export Texture2D as BlipTex for symmetry with the C API.
pub use macroquad::texture::Texture2D as BlipTex;

// Convenience re-export so games don't need to depend on macroquad directly
// for the few items they use most.
pub use macroquad;
