//! The game's pictures (gfx.rs), effects and music (tunes.rs): see `blip::build_script`.
#[path = "src/gfx.rs"]
mod gfx;
#[path = "src/tunes.rs"]
mod tunes;

fn main() {
    for path in ["src/gfx.rs", "src/tunes.rs", "art"] {
        println!("cargo:rerun-if-changed={path}");
    }
    let mut assets = gfx::generate();
    assets.extend(tunes::sounds());
    blip::build_assets(assets, tunes::music());
}
