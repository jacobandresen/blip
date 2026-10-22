//! The game's pictures (gfx.rs), effects and music (tunes.rs): see `blip::build_assets`.

#[path = "src/gfx.rs"]
mod gfx;
#[path = "src/tunes.rs"]
mod tunes;

fn main() {
    println!("cargo:rerun-if-changed=src/gfx.rs");
    println!("cargo:rerun-if-changed=src/tunes.rs");
    let mut assets = gfx::generate();
    assets.extend(tunes::sounds());
    blip::build_assets(assets, tunes::music());
}
