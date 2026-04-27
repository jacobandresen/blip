//! PNG image helpers.
//!
//! Sprites are authored as RGBA buffers (background = transparent black).
//! macroquad loads them via `Texture2D::from_file_with_format`.

use png::{BitDepth, ColorType, Encoder};

/// An RGBA8 image canvas. Origin is top-left; `(0,0,0,0)` background.
pub struct Image {
    pub w: u32,
    pub h: u32,
    pub px: Vec<u8>, // w*h*4
}

impl Image {
    pub fn new(w: u32, h: u32) -> Self {
        Self { w, h, px: vec![0u8; (w * h * 4) as usize] }
    }

    #[inline]
    pub fn set(&mut self, x: i32, y: i32, r: u8, g: u8, b: u8) {
        self.set_rgba(x, y, r, g, b, 255);
    }

    #[inline]
    pub fn set_rgba(&mut self, x: i32, y: i32, r: u8, g: u8, b: u8, a: u8) {
        if x < 0 || y < 0 || (x as u32) >= self.w || (y as u32) >= self.h {
            return;
        }
        let off = ((y as u32 * self.w + x as u32) * 4) as usize;
        self.px[off] = r;
        self.px[off + 1] = g;
        self.px[off + 2] = b;
        self.px[off + 3] = a;
    }

    /// Encode as a PNG file (RGBA8).
    pub fn encode_png(&self) -> Vec<u8> {
        let mut out = Vec::new();
        {
            let mut enc = Encoder::new(&mut out, self.w, self.h);
            enc.set_color(ColorType::Rgba);
            enc.set_depth(BitDepth::Eight);
            let mut writer = enc.write_header().expect("png header");
            writer.write_image_data(&self.px).expect("png data");
        }
        out
    }
}
