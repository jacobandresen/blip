use std::env;
use std::fs;
use std::path::PathBuf;

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    let out_dir = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR"));
    let assets = blip_assets::rivet::generate();
    blip_assets::write_assets(&out_dir.join("assets"), &assets);

    // Write placeholder screenshot if not already present.
    let manifest = PathBuf::from(env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR"));
    let shot_path = manifest.join("../../web/rivet/screenshot.png");
    if !shot_path.exists() {
        if let Some(p) = shot_path.parent() {
            fs::create_dir_all(p).ok();
        }
        let png = blip_assets::rivet::screenshot();
        fs::write(&shot_path, png).ok();
    }
}
