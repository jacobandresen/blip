use std::env;
use std::path::PathBuf;

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    let out_dir = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR"));
    let assets = blip_assets::galactic_defender::generate();
    blip_assets::write_assets(&out_dir.join("assets"), &assets);
}
