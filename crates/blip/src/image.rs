//! PNG helpers for RGBA sprites with transparent backgrounds.

use png::{BitDepth, ColorType, Encoder};

/// How many of a pixel's 3x3 samples `inside` covers (0 to 9): an antialiased edge.
pub fn coverage(x: i32, y: i32, inside: impl Fn(f32, f32) -> bool) -> u32 {
    let mut n = 0;
    for sy in 0..3 {
        for sx in 0..3 {
            n += inside(x as f32 + (sx as f32 + 0.5) / 3.0, y as f32 + (sy as f32 + 0.5) / 3.0) as u32;
        }
    }
    n
}

/// A whole RGB tune between two pixel colours (0..1), as `(u8,u8,u8)` canvas art works in.
pub fn mix_rgb(a: (u8, u8, u8), b: (u8, u8, u8), t: f32) -> (u8, u8, u8) {
    let f = |p: u8, q: u8| (p as f32 + (q as f32 - p as f32) * t) as u8;
    (f(a.0, b.0), f(a.1, b.1), f(a.2, b.2))
}

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

    /// Rings every solid shape one pixel round in `ink` (the blip look's outline, `crate::look::INK`), on the
    /// clear pixels beside it; leave the shape a pixel of room at the image's edge.
    pub fn outline(&mut self, ink: [u8; 3]) {
        let solid = |img: &Self, x: i32, y: i32| img.get(x, y)[3] > 127;
        let ring: Vec<(i32, i32)> = (0..self.h as i32)
            .flat_map(|y| (0..self.w as i32).map(move |x| (x, y)))
            .filter(|&(x, y)| !solid(self, x, y) && [(-1, 0), (1, 0), (0, -1), (0, 1)].iter().any(|(dx, dy)| solid(self, x + dx, y + dy)))
            .collect();
        for (x, y) in ring {
            self.set(x, y, ink[0], ink[1], ink[2]);
        }
    }

    /// The blip message box (`Blip::draw_panel` at runtime) at (x, y), `w` by `h`: ink with cut corners and a
    /// cream rim two pixels in, for build-time art.
    pub fn panel(&mut self, x: i32, y: i32, w: i32, h: i32) {
        let (ink, cream) = (crate::look::INK, crate::look::CREAM);
        for py in y..y + h {
            for px in x..x + w {
                let (dx, dy) = ((px - x).min(x + w - 1 - px), (py - y).min(y + h - 1 - py));
                if dx + dy < 2 { continue; }
                let rim = (dx == 2 || dx == 3 || dy == 2 || dy == 3) && dx >= 2 && dy >= 2 && dx + dy >= 5;
                let c = if rim { cream } else { ink };
                self.blend(px, py, [c[0], c[1], c[2], if rim { 255 } else { 240 }]);
            }
        }
    }

    /// The pixel at (x, y); clear outside the image.
    pub fn get(&self, x: i32, y: i32) -> [u8; 4] {
        if x < 0 || y < 0 || x as u32 >= self.w || y as u32 >= self.h { return [0; 4]; }
        let o = (y as usize * self.w as usize + x as usize) * 4;
        [self.px[o], self.px[o + 1], self.px[o + 2], self.px[o + 3]]
    }

    /// Composite an unpremultiplied color over the existing pixel.
    /// Paints every pixel from 3x3 samples of `colour` (RGB 0..1, `None` outside the shape): their mean, with the
    /// share inside as alpha, for antialiased edges.
    pub fn supersample(&mut self, colour: impl Fn(f32, f32) -> Option<(f32, f32, f32)>) {
        for py in 0..self.h as i32 {
            for px in 0..self.w as i32 {
                let (mut r, mut g, mut b, mut n) = (0.0f32, 0.0f32, 0.0f32, 0.0f32);
                for sy in 0..3 {
                    for sx in 0..3 {
                        if let Some(c) = colour(px as f32 + (sx as f32 + 0.5) / 3.0, py as f32 + (sy as f32 + 0.5) / 3.0) {
                            r += c.0;
                            g += c.1;
                            b += c.2;
                            n += 1.0;
                        }
                    }
                }
                if n > 0.0 {
                    let k = |v: f32| (v / n * 255.0) as u8;
                    self.set_rgba(px, py, k(r), k(g), k(b), (n / 9.0 * 255.0) as u8);
                }
            }
        }
    }

    pub fn blend(&mut self, x: i32, y: i32, color: [u8; 4]) {
        if x < 0 || y < 0 || x as u32 >= self.w || y as u32 >= self.h { return; }
        let offset = (y as usize * self.w as usize + x as usize) * 4;
        let src = color[3] as f32 / 255.0;
        let dst = self.px[offset + 3] as f32 / 255.0 * (1.0 - src);
        let alpha = src + dst;
        if alpha == 0.0 { return; }
        for (i, channel) in color[..3].iter().enumerate() {
            self.px[offset + i] = ((*channel as f32 * src + self.px[offset + i] as f32 * dst) / alpha).round() as u8;
        }
        self.px[offset + 3] = (alpha * 255.0).round() as u8;
    }

    /// Rasterize the runtime's pixel font into an asset without a graphics context.
    pub fn text(&mut self, text: &str, x: i32, y: i32, size: i32, color: [u8; 4]) {
        for (i, ch) in text.chars().enumerate() {
            let Some(glyph) = crate::glyphs::glyph_index(ch) else { continue; };
            for (row, bits) in crate::glyphs::FONT[glyph].iter().enumerate() {
                for column in 0..5 {
                    if bits & (1 << (4 - column)) == 0 { continue; }
                    for dy in 0..size {
                        for dx in 0..size {
                            self.blend(x + (i as i32 * 6 + column) * size + dx, y + row as i32 * size + dy, color);
                        }
                    }
                }
            }
        }
    }

    /// Antialiased line with flat ends, matching the runtime's line primitive.
    pub fn line(&mut self, from: (f32, f32), to: (f32, f32), width: f32, color: [u8; 4]) {
        let (dx, dy) = (to.0 - from.0, to.1 - from.1);
        let length = dx.hypot(dy);
        if length == 0.0 { return; }
        let (ux, uy) = (dx / length, dy / length);
        let radius = width / 2.0;
        let left = ((from.0.min(to.0) - radius).floor() as i32).max(0);
        let right = ((from.0.max(to.0) + radius).ceil() as i32).min(self.w as i32);
        let top = ((from.1.min(to.1) - radius).floor() as i32).max(0);
        let bottom = ((from.1.max(to.1) + radius).ceil() as i32).min(self.h as i32);
        for y in top..bottom {
            for x in left..right {
                let covered = coverage(x, y, |sx, sy| {
                    let (px, py) = (sx - from.0, sy - from.1);
                    let along = px * ux + py * uy;
                    (0.0..=length).contains(&along) && (px * uy - py * ux).abs() <= radius
                });
                if covered > 0 {
                    self.blend(x, y, [color[0], color[1], color[2], (color[3] as f32 * covered as f32 / 9.0).round() as u8]);
                }
            }
        }
    }

    pub fn circle(&mut self, cx: f32, cy: f32, radius: f32, color: [u8; 4]) {
        for y in ((cy - radius - 1.0).floor() as i32).max(0)..((cy + radius + 1.0).ceil() as i32).min(self.h as i32) {
            for x in ((cx - radius - 1.0).floor() as i32).max(0)..((cx + radius + 1.0).ceil() as i32).min(self.w as i32) {
                let coverage = (radius + 0.5 - (x as f32 + 0.5 - cx).hypot(y as f32 + 0.5 - cy)).clamp(0.0, 1.0);
                self.blend(x, y, [color[0], color[1], color[2], (color[3] as f32 * coverage) as u8]);
            }
        }
    }

    /// Decode a generated RGBA8 sprite for composition into another asset.
    pub fn from_png(bytes: &[u8]) -> Self {
        let mut reader = png::Decoder::new(bytes).read_info().expect("PNG header");
        let mut pixels = vec![0; reader.output_buffer_size()];
        let info = reader.next_frame(&mut pixels).expect("PNG pixels");
        assert_eq!(info.color_type, ColorType::Rgba);
        assert_eq!(info.bit_depth, BitDepth::Eight);
        pixels.truncate(info.buffer_size());
        Self { w: info.width, h: info.height, px: pixels }
    }

    /// Scale and composite with the same bilinear sampling as smooth runtime textures.
    pub fn blit(&mut self, source: &Self, x: f32, y: f32, width: f32, height: f32) {
        for py in (y.floor() as i32).max(0)..((y + height).ceil() as i32).min(self.h as i32) {
            for px in (x.floor() as i32).max(0)..((x + width).ceil() as i32).min(self.w as i32) {
                let sx = ((px as f32 + 0.5 - x) / width * source.w as f32 - 0.5).clamp(0.0, source.w as f32 - 1.0);
                let sy = ((py as f32 + 0.5 - y) / height * source.h as f32 - 0.5).clamp(0.0, source.h as f32 - 1.0);
                let (left, top) = (sx as u32, sy as u32);
                let (right, bottom) = ((left + 1).min(source.w - 1), (top + 1).min(source.h - 1));
                let (fx, fy) = (sx.fract(), sy.fract());
                let sample = |cx, cy, channel| source.px[((cy * source.w + cx) * 4) as usize + channel] as f32;
                let color = std::array::from_fn(|channel| {
                    let a = sample(left, top, channel) * (1.0 - fx) + sample(right, top, channel) * fx;
                    let b = sample(left, bottom, channel) * (1.0 - fx) + sample(right, bottom, channel) * fx;
                    (a * (1.0 - fy) + b * fy).round() as u8
                });
                self.blend(px, py, color);
            }
        }
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
    fn translucent_layers_preserve_color_and_coverage_on_transparent_art() {
        let mut image = Image::new(1, 1);
        image.blend(0, 0, [255, 0, 0, 128]);
        assert_eq!(image.px, [255, 0, 0, 128]);
        image.blend(0, 0, [0, 0, 255, 128]);
        assert_eq!(image.px, [85, 0, 170, 192]);
        image.blend(0, 0, [0, 255, 0, 255]);
        assert_eq!(image.px, [0, 255, 0, 255]);
    }

    #[test]
    fn scaled_sprites_blend_neighboring_pixels_without_losing_opacity() {
        let mut source = Image::new(2, 1);
        source.set(0, 0, 255, 0, 0);
        source.set(1, 0, 0, 0, 255);
        let mut image = Image::new(1, 1);
        image.blit(&source, 0.0, 0.0, 1.0, 1.0);
        assert_eq!(image.px, [128, 0, 128, 255]);
    }

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

        let back = Image::from_png(&png);
        assert_eq!((back.w, back.h), (128, 128));
        assert_eq!(back.px, img.px);
    }
}
