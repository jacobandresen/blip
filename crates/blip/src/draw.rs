//! Drawing primitives, as `Blip` methods over macroquad's shape and texture API.
//! All coordinates are in virtual-canvas pixels. The canvas origin (0, 0) is the top-left corner.

use macroquad::color::{Color, WHITE};
use macroquad::math::{vec2, Rect};
use macroquad::shapes::{draw_circle, draw_circle_lines, draw_line, draw_rectangle, draw_rectangle_lines, draw_triangle};
use macroquad::texture::{draw_texture_ex, DrawTextureParams, Texture2D};
use macroquad::window::clear_background;

use crate::Blip;

impl Blip {
    /// Fill the canvas with a solid colour. Call this at the start of your draw pass.
    pub fn clear(&self, c: Color) { clear_background(c); }

    /// Draw a solid filled rectangle.
    pub fn fill_rect(&self, x: f32, y: f32, w: f32, h: f32, c: Color) { draw_rectangle(x, y, w, h, c); }

    /// Fill a rectangle with `bands` flat stripes top to bottom, each the colour `at(k)` gives for its place `k`
    /// (0 at the top band, 1 at the bottom): a sky, a sea, a gradient that reads as pixel art.
    pub fn fill_bands(&self, x: f32, y: f32, w: f32, h: f32, bands: usize, at: impl Fn(f32) -> Color) {
        let bh = h / bands as f32;
        for i in 0..bands {
            let k = i as f32 / (bands - 1) as f32;
            draw_rectangle(x, y + i as f32 * bh, w, bh + 1.0, at(k));
        }
    }

    /// Draw a 1-pixel outline rectangle (no fill).
    pub fn draw_rect(&self, x: f32, y: f32, w: f32, h: f32, c: Color) { draw_rectangle_lines(x, y, w, h, 1.0, c); }

    /// Draw a circle outline with the given radius and line width.
    pub fn draw_circle(&self, cx: f32, cy: f32, r: f32, width: f32, c: Color) {
        draw_circle_lines(cx, cy, r.max(0.0), width.max(0.0), c);
    }

    /// Draw a 1-pixel line between two points.
    pub fn draw_line(&self, x1: f32, y1: f32, x2: f32, y2: f32, c: Color) { draw_line(x1, y1, x2, y2, 1.0, c); }

    /// Draw a line of arbitrary thickness between two points.
    pub fn draw_line_ex(&self, x1: f32, y1: f32, x2: f32, y2: f32, thickness: f32, c: Color) {
        draw_line(x1, y1, x2, y2, thickness, c);
    }

    /// Join points in order; a marked endpoint selects the alternate colour.
    pub fn draw_polyline(&self, points: impl IntoIterator<Item = (f32, f32, bool)>, thickness: f32, color: Color,
        alternate: Color) {
        let mut previous = None;
        for (x, y, marked) in points {
            if let Some((px, py, was_marked)) = previous {
                self.draw_line_ex(px, py, x, y, thickness, if marked || was_marked { alternate } else { color });
            }
            previous = Some((x, y, marked));
        }
    }

    /// Draw a glowing line: a soft wide halo under a bright hairline core. Use for vector silhouettes (ship
    /// outlines, asteroid edges) that should read as "lit up" against a black background.
    pub fn draw_glow_line(&self, x1: f32, y1: f32, x2: f32, y2: f32, c: Color) {
        let (halo, mid) = glow_colors(c, 0.16);
        draw_line(x1, y1, x2, y2, 6.0, halo);
        draw_line(x1, y1, x2, y2, 2.5, mid);
        draw_line(x1, y1, x2, y2, 1.0, c);
    }

    /// Draw a solid filled circle. (`cx`, `cy`) is the centre, `r` is the radius.
    pub fn fill_circle(&self, cx: f32, cy: f32, r: f32, c: Color) { draw_circle(cx, cy, r, c); }

    /// Draw a capsule from (`x1`, `y1`) to (`x2`, `y2`), tapering from `r1` to `r2`.
    /// Use opaque colours: a translucent one doubles where the round ends overlap the middle.
    #[allow(clippy::too_many_arguments)]
    pub fn fill_capsule(&self, x1: f32, y1: f32, x2: f32, y2: f32, r1: f32, r2: f32, c: Color) {
        let (dx, dy) = (x2 - x1, y2 - y1);
        let len = (dx * dx + dy * dy).sqrt();
        // One end inside the other: only the bigger circle shows.
        if len <= (r1 - r2).abs() + 0.01 {
            if r1 >= r2 { draw_circle(x1, y1, r1, c) } else { draw_circle(x2, y2, r2, c) }
            return;
        }
        // The two ends joined along their outer tangents: a tangent touches each circle where its normal n has
        // n·u = (r1 - r2) / len. Three primitives, where a row of circles a third of a radius apart took fifteen
        // for a limb (Brawler's crowd halved the frame rate on a slow CPU drawn that way).
        let (ux, uy) = (dx / len, dy / len);
        let s = (r1 - r2) / len;
        let co = (1.0 - s * s).max(0.0).sqrt();
        let (ax, ay) = (-uy * co + ux * s, ux * co + uy * s);
        let (bx, by) = (uy * co + ux * s, -ux * co + uy * s);
        let (p1a, p2a) = (vec2(x1 + ax * r1, y1 + ay * r1), vec2(x2 + ax * r2, y2 + ay * r2));
        let (p1b, p2b) = (vec2(x1 + bx * r1, y1 + by * r1), vec2(x2 + bx * r2, y2 + by * r2));
        draw_triangle(p1a, p2a, p2b, c);
        draw_triangle(p1a, p2b, p1b, c);
        draw_circle(x1, y1, r1, c);
        draw_circle(x2, y2, r2, c);
    }

    /// Fill the triangle `a`, `b`, `c`.
    pub fn fill_triangle(&self, a: (f32, f32), b: (f32, f32), c: (f32, f32), col: Color) {
        draw_triangle(a.into(), b.into(), c.into(), col);
    }

    /// Fill the quad `a`, `b`, `c`, `d` (convex, in order round it) as two triangles.
    pub fn fill_quad(&self, a: (f32, f32), b: (f32, f32), c: (f32, f32), d: (f32, f32), col: Color) {
        self.fill_triangle(a, b, c, col);
        self.fill_triangle(a, c, d, col);
    }

    /// Fill the polygon `pts` (in order round it, star-shaped about `centre`) as a fan from `centre`.
    pub fn fill_fan(&self, centre: (f32, f32), pts: &[(f32, f32)], col: Color) {
        for k in 0..pts.len() {
            self.fill_triangle(centre, pts[k], pts[(k + 1) % pts.len()], col);
        }
    }

    /// Draw `rays` alternating wedges from (`cx`, `cy`) to `reach`, starting at `turn`.
    /// Even ray counts cover the full circle.
    pub fn fill_sunburst(&self, cx: f32, cy: f32, rays: usize, turn: f32, reach: f32, colors: [Color; 2]) {
        // Half the angle a ray is given, as the slope its edge leaves the axis at.
        let spread = (std::f32::consts::PI / rays as f32).tan();
        for j in 0..rays {
            let ang = turn + j as f32 * std::f32::consts::TAU / rays as f32;
            let (dx, dy) = (ang.cos(), ang.sin());
            // Circles spaced by their own size: a wedge in forty, not four hundred.
            let mut d = 16.0;
            while d < reach {
                let r = d * spread;
                draw_circle(cx + dx * d, cy + dy * d, r, colors[j % 2]);
                d += r * 0.7;
            }
        }
    }

    /// Draw a soft glowing circle: a faint wide halo under a bright core. Good for thruster flames, muzzle
    /// flashes, and explosion particles.
    pub fn fill_glow_circle(&self, cx: f32, cy: f32, r: f32, c: Color) {
        let (halo, mid) = glow_colors(c, 0.15);
        draw_circle(cx, cy, r * 2.6, halo);
        draw_circle(cx, cy, r * 1.5, mid);
        draw_circle(cx, cy, r, c);
    }

    /// Draw a texture stretched to fill the given rectangle.
    pub fn draw_texture(&self, tex: &Texture2D, x: f32, y: f32, w: f32, h: f32) {
        self.draw_texture_tinted(tex, x, y, w, h, WHITE);
    }

    /// Draw a texture stretched to fill the given rectangle, multiplied by a tint colour: hit flashes,
    /// transparency, palette swaps.
    pub fn draw_texture_tinted(&self, tex: &Texture2D, x: f32, y: f32, w: f32, h: f32, tint: Color) {
        draw_texture_ex(tex, x, y, tint, DrawTextureParams { dest_size: Some(vec2(w, h)), ..Default::default() });
    }

    /// The same, mirrored when `face` is negative: a figure drawn facing right turns to the way it is going.
    #[allow(clippy::too_many_arguments)]
    pub fn draw_texture_facing(&self, tex: &Texture2D, x: f32, y: f32, w: f32, h: f32, tint: Color, face: f32) {
        let params = DrawTextureParams { dest_size: Some(vec2(w, h)), flip_x: face < 0.0, ..Default::default() };
        draw_texture_ex(tex, x, y, tint, params);
    }

    /// Draw cell (`col`, `row`) of a `cols` x `rows` sprite sheet, rotated by `rotation` radians (clockwise)
    /// about its centre.
    #[allow(clippy::too_many_arguments)]
    pub fn draw_texture_cell(&self, tex: &Texture2D, col: u32, cols: u32, row: u32, rows: u32,
        x: f32, y: f32, w: f32, h: f32, rotation: f32) {
        let (cw, ch) = (tex.width() / cols as f32, tex.height() / rows as f32);
        draw_texture_ex(tex, x, y, WHITE, DrawTextureParams {
            dest_size: Some(vec2(w, h)),
            source: Some(Rect::new(cw * col as f32, ch * row as f32, cw, ch)),
            rotation,
            ..Default::default()
        });
    }

    /// Draw the `(sx, sy, sw, sh)` pixel region of a texture into the given rectangle; a negative `w` mirrors it.
    #[allow(clippy::too_many_arguments)]
    pub fn draw_texture_region(&self, tex: &Texture2D, sx: f32, sy: f32, sw: f32, sh: f32,
        x: f32, y: f32, w: f32, h: f32) {
        draw_texture_ex(tex, x, y, WHITE, DrawTextureParams {
            dest_size: Some(vec2(w.abs(), h)),
            source: Some(Rect::new(sx, sy, sw, sh)),
            flip_x: w < 0.0,
            ..Default::default()
        });
    }
}

fn glow_colors(c: Color, halo_alpha: f32) -> (Color, Color) {
    (Color { a: c.a * halo_alpha, ..c }, Color { a: c.a * 0.35, ..c })
}

/// Decode a PNG (usually `include_bytes!`) into a texture that keeps crisp
/// pixels when scaled: for pixel art.
pub fn load_png(bytes: &[u8]) -> Texture2D {
    let tex = Texture2D::from_file_with_format(bytes, Some(macroquad::prelude::ImageFormat::Png));
    tex.set_filter(macroquad::texture::FilterMode::Nearest);
    tex
}

/// The same, smoothed when scaled: for shaded sprites and soft edges.
pub fn load_png_smooth(bytes: &[u8]) -> Texture2D {
    let tex = load_png(bytes);
    tex.set_filter(macroquad::texture::FilterMode::Linear);
    tex
}
