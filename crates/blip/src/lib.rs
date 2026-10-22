//! Shared game context, rendering, input, effects and audio for the BLIP games.
//! Macroquad drives the frame loop; sounds load asynchronously and play synchronously.

// Generating pictures and sounds: what each game's gfx.rs and tunes.rs build on.
pub mod cosy;
pub mod image;
pub mod song;
pub mod techno;
pub mod wav;

/// A generated file: its path under the asset folder, and its bytes.
pub type Asset = (&'static str, Vec<u8>);

/// Write generated files under `dir`, making folders as needed.
pub fn write_assets(dir: &std::path::Path, assets: &[Asset]) {
    for (path, bytes) in assets {
        let dest = dir.join(path);
        if let Some(parent) = dest.parent() {
            std::fs::create_dir_all(parent).expect("create asset folder");
        }
        std::fs::write(&dest, bytes).expect("write asset");
    }
}

#[cfg(feature = "engine")]
pub mod audio;
#[cfg(feature = "engine")]
pub mod bot;
#[cfg(feature = "engine")]
pub mod color;
#[cfg(feature = "engine")]
pub mod ctx;
#[cfg(feature = "engine")]
pub mod draw;
#[cfg(feature = "engine")]
pub mod font;
#[cfg(feature = "engine")]
pub mod fx;
#[cfg(feature = "engine")]
pub mod input;
#[cfg(feature = "engine")]
pub mod math;
#[cfg(feature = "engine")]
pub mod pool;
#[cfg(feature = "engine")]
pub mod session;
#[cfg(feature = "engine")]
pub mod timer;
#[cfg(feature = "engine")]
pub mod web;

#[cfg(feature = "engine")]
pub use audio::{
    play_alert, play_ambient, play_music, play_music_volume, play_sfx, play_sfx_volume, stop_alert,
    stop_ambient, stop_music, BlipSound, Jukebox,
};
#[cfg(feature = "engine")]
pub use color::{
    blend, hsv, shade, BLIP_BLACK, BLIP_BLUE, BLIP_CYAN, BLIP_DARKGRAY, BLIP_GRAY, BLIP_GREEN,
    BLIP_MAGENTA, BLIP_ORANGE, BLIP_RED, BLIP_WHITE, BLIP_YELLOW, NEON_CYAN, NEON_GREEN,
    NEON_MAGENTA, NEON_ORANGE, NEON_PINK, NEON_PURPLE, NEON_YELLOW,
};
#[cfg(feature = "engine")]
pub use ctx::{window_conf, Blip};
#[cfg(feature = "engine")]
pub use draw::{load_png, load_png_smooth};
#[cfg(feature = "engine")]
pub use fx::Fx;
#[cfg(feature = "engine")]
pub use fx::{EffectParticle, EffectParticles};
#[cfg(feature = "engine")]
pub use math::{
    angle_diff, clamp, distance, ease_out_cubic, lerp, ordered_ladder, rand_int, rand_range_f32,
    rand_seed, rects_overlap, scatter, segment_circle_overlap, segment_rect_overlap, smoothstep01,
    wrap,
};
#[cfg(feature = "engine")]
pub use pool::{pool_iter, pool_iter_mut, pool_spawn, Pooled};
#[cfg(feature = "engine")]
pub use session::{LifeResult, Session, GAME_OVER_MIN_WAIT};
#[cfg(feature = "engine")]
pub use timer::Timer;

// Re-export macroquad's color::Color as BlipColor for game code.
#[cfg(feature = "engine")]
pub use macroquad::color::Color as BlipColor;
// Re-export Texture2D as BlipTex for symmetry with the C API.
#[cfg(feature = "engine")]
pub use macroquad::texture::Texture2D as BlipTex;

// Convenience re-export so games don't need to depend on macroquad directly
// for the few items they use most.
#[cfg(feature = "engine")]
pub use macroquad;
