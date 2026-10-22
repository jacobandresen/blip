//! The game's pictures (`gfx.rs`) and sounds (`tunes.rs`), written to OUT_DIR/assets.

use std::env;
use std::path::PathBuf;

#[path = "src/gfx.rs"]
mod gfx;
#[path = "src/tunes.rs"]
mod tunes;

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=src/gfx.rs");
    println!("cargo:rerun-if-changed=src/tunes.rs");
    let out_dir = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR"));
    let mut assets = gfx::generate();
    assets.extend(tunes::sounds());
    blip::write_assets(&out_dir.join("assets"), &assets);
}
