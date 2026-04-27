//! Asset generation for blip games.
//!
//! Each game module exposes `generate(out_dir: &Path)` which writes its
//! PNG and WAV assets into `out_dir/{images,sounds}/`. Games drive this
//! from their `build.rs` and consume the bytes via `include_bytes!`.

//! Asset generation for blip games.
//!
//! Each game module exposes `generate()` returning a list of
//! `(relative_path, bytes)`. The build.rs of each game crate writes
//! these into `$OUT_DIR/assets/...` and the game embeds them with
//! `include_bytes!`.

use std::fs;
use std::path::Path;

pub mod image;
pub mod wav;

pub mod bouncer;
pub mod galactic_defender;
pub mod rally;
pub mod serpent;

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
