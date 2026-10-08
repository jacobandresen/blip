//! Build-time asset generators for the BLIP games.
//! Each `generate()` returns `(relative_path, bytes)` pairs for the crate's `build.rs`.

use std::fs;
use std::path::Path;

pub mod image;
pub mod cosy;
pub mod song;
pub mod techno;
pub mod wav;

pub mod adder;
pub mod bouncer;
pub mod brawler;
pub mod bubbler;
pub mod galactic_defender;
pub mod meteors;
pub mod rally;
pub mod serpent;
pub mod sky_raider;

pub type Asset = (&'static str, Vec<u8>);

/// Write a list of `(relative_path, bytes)` rooted at `out_dir`.
/// Creates parent directories as needed. Intended for `build.rs`.
pub fn write_assets(out_dir: &Path, assets: &[Asset]) {
    for (rel, bytes) in assets {
        let dest = out_dir.join(rel);
        if let Some(parent) = dest.parent() {
            fs::create_dir_all(parent).expect("create asset dir");
        }
        fs::write(&dest, bytes).expect("write asset");
    }
}
