//! Shared game context, rendering, input, effects and audio for the BLIP games.
//! Macroquad drives the frame loop; sounds load asynchronously and play synchronously.

// Generating pictures and sounds: what each game's gfx.rs and tunes.rs build on.
pub mod cosy;
pub mod glyphs;
pub mod image;
pub mod look;
pub mod song;
pub mod techno;
pub mod wav;

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

/// An effect its build script made, loaded and ready to play: `blip::sound!("eat")`, in an async fn.
#[macro_export]
macro_rules! sound {
    ($name:expr) => {
        $crate::audio::load_sound($crate::wav!($name)).await
    };
}

/// A game's effects, named once: the struct and its async `load`. A field plays the sound of its own name, the one
/// after `=`, or holds what a `: Type = expression` makes (takes, say):
/// `blip::sounds!(Sounds { eat, kill = "defeated", hits: [BlipSound; 2] = [sound!("hit"), sound!("hit2")] });`
/// then `let sfx = Sounds::load().await;`. Doc comments and a visibility (`pub(crate) Sounds`) carry over.
#[macro_export]
macro_rules! sounds {
    ($(#[$doc:meta])* $vis:vis $name:ident { $($(#[$meta:meta])* $field:ident $(= $file:literal)? $(: $ty:ty = $init:expr)?),* $(,)? }) => {
        $(#[$doc])*
        $vis struct $name { $($(#[$meta])* $vis $field: $crate::sounds!(@type $($ty)?)),* }
        impl $name {
            $vis async fn load() -> Self {
                Self { $($field: $crate::sounds!(@load $field $(init $init)? $(file $file)?)),* }
            }
        }
    };
    (@type $ty:ty) => { $ty };
    (@type) => { $crate::BlipSound };
    (@load $field:ident init $init:expr) => { $init };
    (@load $field:ident file $file:literal) => { $crate::sound!($file) };
    (@load $field:ident) => { $crate::sound!(stringify!($field)) };
}

/// A generated file: its path under the asset folder, and its bytes.
pub type Asset = (&'static str, Vec<u8>);

/// A game's whole build script (`blip::build_script!();`): its pictures (src/gfx.rs) and its effects and music
/// (src/tunes.rs) become its assets, through `build_assets`. `blip::build_script!("art")` also rebuilds them when
/// the files under `art` change.
#[macro_export]
macro_rules! build_script {
    ($($watch:literal),*) => {
        #[path = "src/gfx.rs"]
        mod gfx;
        #[path = "src/tunes.rs"]
        mod tunes;

        fn main() {
            for path in ["src/gfx.rs", "src/tunes.rs" $(, $watch)*] {
                println!("cargo:rerun-if-changed={path}");
            }
            let mut assets = gfx::generate();
            assets.extend(tunes::sounds());
            $crate::build_assets(assets, tunes::music());
        }
    };
}

/// A game's build script: the pictures and effects go to OUT_DIR/assets for the
/// game to embed, and the music to `web/<game>/music`, beside the game's page,
/// for it to load while it plays.
pub fn build_assets(assets: Vec<Asset>, music: Vec<(String, Vec<u8>)>) {
    use std::{env, fs, path::PathBuf};
    let var = |name| env::var(name).expect(name);
    let out = PathBuf::from(var("OUT_DIR")).join("assets");
    let web = PathBuf::from(var("CARGO_MANIFEST_DIR")).join("../../web").join(var("CARGO_PKG_NAME"));
    let write = |path: PathBuf, bytes: &[u8]| {
        fs::create_dir_all(path.parent().expect("a folder")).expect("make the folder");
        fs::write(path, bytes).expect("write the file");
    };
    for (path, bytes) in assets {
        write(out.join(path), &bytes);
    }
    // The folder holds exactly the music the game makes now.
    let _ = fs::remove_dir_all(web.join("music"));
    for (path, bytes) in music {
        write(web.join(path), &bytes);
    }
}

/// Items that need the game engine (macroquad); a build script builds without them.
macro_rules! engine {
    ($($item:item)*) => { $(#[cfg(feature = "engine")] $item)* };
}

pub mod math;

engine! {
    pub mod audio;
    pub mod bot;
    pub mod color;
    pub mod ctx;
    mod draw;
    mod frame;
    pub mod font;
    pub mod fx;
    pub mod grid;
    pub mod input;
    pub mod pool;
    pub mod session;
    pub mod testing;
    pub mod timer;
    pub mod web;

    pub use audio::{
        play_alert, play_ambient, play_sfx, play_sfx_volume, stop_alert, stop_ambient,
        BlipSound, Jukebox,
    };
    pub use color::{
        blend, hsv, shade, BLIP_BLACK, BLIP_BLUE, BLIP_CREAM, BLIP_CYAN, BLIP_DARKGRAY, BLIP_DUSK, BLIP_GOLD, BLIP_GRAY,
        BLIP_GREEN, BLIP_HEART, BLIP_INK, BLIP_LEAF, BLIP_MAGENTA, BLIP_NIGHT, BLIP_ORANGE, BLIP_PLUM, BLIP_RED,
        BLIP_SEA, BLIP_SHADOW, BLIP_WHITE, BLIP_YELLOW,
    };
    pub use ctx::{window_conf, Blip};
    pub use draw::{load_png, load_png_smooth};
    pub use fx::{draw_fireworks, shake, EffectParticle, EffectParticles, Fx};
    pub use math::{
        angle_diff, clamp, distance, ease_out_cubic, frame_starts, lerp, rand_int, rand_range_f32, rand_seed,
        rects_overlap, scatter, segment_circle_overlap, segment_rect_overlap, smoothstep01, spool, sweep_hits,
        swept_from, wrap,
    };
    pub use pool::{pool_iter, pool_iter_mut, pool_spawn, Pooled};
    pub use session::{Continue, LifeResult, Session, Streak, GAME_OVER_MIN_WAIT};
    pub use timer::{blink, now, Timer};

    /// macroquad's colour, as the games name it.
    pub use macroquad::color::Color as BlipColor;
    /// macroquad itself, so a game needs no dependency of its own for the few items it uses directly.
    pub use macroquad;
}
