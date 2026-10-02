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
            // The crate's default is its fastest deflate, which left the
            // carrier at 254 KB; this is 12 times smaller, at build time only.
            enc.set_compression(png::Compression::Best);
            enc.set_adaptive_filter(png::AdaptiveFilterType::Adaptive);
            let mut writer = enc.write_header().expect("png header");
            writer.write_image_data(&self.px).expect("png data");
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_png_is_compressed_and_decodes_to_the_same_pixels() {
        // A gradient with a sprite's flat areas: 64 KB raw.
        let mut img = Image::new(128, 128);
        for y in 0..128i32 {
            for x in 0..128i32 {
                let v = if (x / 16 + y / 16) % 2 == 0 { 200 } else { (x + y) as u8 };
                img.set(x, y, v, 255 - v, 40);
            }
        }
        let png = img.encode_png();
        assert!(png.len() < img.px.len() / 8, "{} bytes for {} of pixels", png.len(), img.px.len());

        let mut reader = png::Decoder::new(&png[..]).read_info().expect("header");
        let mut out = vec![0u8; reader.output_buffer_size()];
        let info = reader.next_frame(&mut out).expect("frame");
        assert_eq!((info.width, info.height), (128, 128));
        assert_eq!(&out[..info.buffer_size()], &img.px[..]);
    }
}
