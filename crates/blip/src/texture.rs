//! Texture loading helpers shared by every blip game.

use macroquad::prelude::ImageFormat;
use macroquad::texture::{FilterMode, Texture2D};

/// Decode a PNG byte slice into a `Texture2D` with nearest-neighbour filtering
/// (preserves the pixel-art look). Call at startup for each `include_bytes!`'d
/// asset — see the [`blip_image!`](crate::blip_image) macro for the byte slices.
pub fn load_png(bytes: &'static [u8]) -> Texture2D {
    let tex = Texture2D::from_file_with_format(bytes, Some(ImageFormat::Png));
    tex.set_filter(FilterMode::Nearest);
    tex
}
