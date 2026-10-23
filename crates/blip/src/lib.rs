//! Shared game context, rendering, input, effects and audio for the BLIP games.
//! Macroquad drives the frame loop; sounds load asynchronously and play synchronously.

// Generating pictures and sounds: what each game's gfx.rs and tunes.rs build on.
pub mod cosy;
pub mod image;
pub mod song;
pub mod techno;
pub mod wav;
pub mod xm;

/// A picture its build script made (see `build_assets`), built into the game.
#[macro_export]
macro_rules! png {
    ($name:expr) => {
        (include_bytes!(concat!(env!("OUT_DIR"), "/assets/images/", $name, ".png")) as &[u8])
    };
}

/// An effect its build script made, built into the game.
#[macro_export]
macro_rules! wav {
    ($name:expr) => {
        (include_bytes!(concat!(env!("OUT_DIR"), "/assets/sounds/", $name, ".wav")) as &[u8])
    };
}

/// A generated file: its path under the asset folder, and its bytes.
pub type Asset = (&'static str, Vec<u8>);

/// A game's whole build script (`blip::build_script!();`): its pictures (src/gfx.rs) and its effects and music
/// (src/tunes.rs) become its assets, through `build_assets`.
#[macro_export]
macro_rules! build_script {
    () => {
        #[path = "src/gfx.rs"]
        mod gfx;
        #[path = "src/tunes.rs"]
        mod tunes;

        fn main() {
            println!("cargo:rerun-if-changed=src/gfx.rs");
            println!("cargo:rerun-if-changed=src/tunes.rs");
            let mut assets = gfx::generate();
            assets.extend(tunes::sounds());
            $crate::build_assets(assets, tunes::music());
        }
    };
}

/// A game's build script: the pictures and effects go to OUT_DIR/assets for the
/// game to embed, and every file, the music too, to web/TAPE/<GAME>/ASSETS, the
/// folder BLIP BASIC reads, with a manifest.json listing them.
pub fn build_assets(assets: Vec<Asset>, music: Vec<(String, Vec<u8>)>) {
    use std::{env, fs, path::PathBuf};
    let var = |name| env::var(name).expect(name);
    let out = PathBuf::from(var("OUT_DIR")).join("assets");
    let game = var("CARGO_PKG_NAME").to_ascii_uppercase();
    let tape = PathBuf::from(var("CARGO_MANIFEST_DIR")).join("../../web/TAPE").join(game).join("ASSETS");
    let write = |path: PathBuf, bytes: &[u8]| {
        fs::create_dir_all(path.parent().expect("a folder")).expect("make the folder");
        fs::write(path, bytes).expect("write the file");
    };
    // The folder holds exactly what the game makes now.
    let _ = fs::remove_dir_all(&tape);
    let files = assets.into_iter().map(|(path, bytes)| (path.to_string(), bytes)).chain(music);
    let mut names = Vec::new();
    for (path, bytes) in files {
        if !path.starts_with("music/") {
            write(out.join(&path), &bytes);
        }
        let name = path.rsplit('/').next().expect("a name").to_ascii_uppercase();
        assert!(!names.contains(&name), "two files called {name}");
        write(tape.join(&name), &bytes);
        names.push(name);
    }
    names.sort();
    let list = names.iter().map(|n| format!("\"{n}\"")).collect::<Vec<_>>().join(", ");
    write(tape.join("manifest.json"), format!("[{list}]\n").as_bytes());
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
    play_alert, play_ambient, play_music, play_music_volume, play_sfx, play_sfx_volume, stop_alert, stop_ambient,
    stop_music, BlipSound, Jukebox,
};
#[cfg(feature = "engine")]
pub use color::{
    blend, hsv, shade, BLIP_BLACK, BLIP_BLUE, BLIP_CYAN, BLIP_DARKGRAY, BLIP_GRAY, BLIP_GREEN, BLIP_MAGENTA,
    BLIP_ORANGE, BLIP_RED, BLIP_WHITE, BLIP_YELLOW, NEON_CYAN, NEON_GREEN, NEON_MAGENTA, NEON_ORANGE, NEON_PINK,
    NEON_PURPLE, NEON_YELLOW,
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
    angle_diff, clamp, distance, ease_out_cubic, lerp, ordered_ladder, rand_int, rand_range_f32, rand_seed,
    rects_overlap, scatter, segment_circle_overlap, segment_rect_overlap, smoothstep01, wrap,
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
