// Compiled by build.rs, and by the game only for its tests: much goes unused in each.
#![allow(dead_code)]

use std::f32::consts::PI;

use blip::image::Image;
use blip::Asset;

// Must match crates/raider/src/main.rs's PLAYER_W / PLAYER_H.
const PLAYER_W: i32 = 66;
const PLAYER_H: i32 = 58;
// Must match crates/raider/src/main.rs's ENEMY_W / ENEMY_H.
const ENEMY_W: i32 = 62;
const ENEMY_H: i32 = 52;
include!("boss_shape.rs");
// Must match crates/raider/src/main.rs's POW_W / POW_H.
const POW_W: i32 = 14;
const POW_H: i32 = 14;
// Must match crates/raider/src/main.rs's HEALTH_W / HEALTH_H.
const HEALTH_W: i32 = 14;
const HEALTH_H: i32 = 14;
// Must match crates/raider/src/main.rs's CARRIER_W / CARRIER_H.
const CARRIER_W: i32 = 186;
const CARRIER_H: i32 = 760;
// Must match crates/raider/src/main.rs's BOAT_W / BOAT_H.
const BOAT_W: i32 = 34;
const BOAT_H: i32 = 16;
// Must match crates/raider/src/main.rs's ISLAND_SIZES.
const ISLAND_SIZES: [(i32, i32); 3] = [(64, 44), (98, 68), (140, 96)];

// Four-times supersampled aircraft planforms; sprite origin is the nose, pointing up.

const SS: usize = 4;

/// `k` is design units to pixels (a design drawn at 1 can be rendered at 2
/// for twice the size and twice the detail); `clip`, in design units, is
/// the only area a fill scans, which keeps a big sprite quick to build.
struct Canvas {
    w: usize,
    h: usize,
    k: f32,
    clip: (f32, f32, f32, f32),
    buf: Vec<[f32; 4]>,
}

impl Canvas {
    fn new(w: i32, h: i32) -> Self {
        Self::scaled(w, h, 1.0)
    }
    fn scaled(w: i32, h: i32, k: f32) -> Self {
        let (w, h) = (w as usize, h as usize);
        Self { w, h, k, clip: (0.0, 0.0, 1e9, 1e9), buf: vec![[0.0; 4]; w * h * SS * SS] }
    }
    fn clip(&mut self, x0: f32, y0: f32, x1: f32, y1: f32) {
        self.clip = (x0, y0, x1, y1);
    }
    fn unclip(&mut self) {
        self.clip = (0.0, 0.0, 1e9, 1e9);
    }
    /// Paint `col` (straight RGBA, 0..1) wherever `inside(x, y)` holds,
    /// with `shade(x, y)` scaling its RGB.
    fn fill(&mut self, inside: impl Fn(f32, f32) -> bool, shade: impl Fn(f32, f32) -> f32, col: [f32; 4]) {
        let sw = self.w * SS;
        let unit = self.k * SS as f32;
        let (cx0, cy0, cx1, cy1) = self.clip;
        let sx0 = ((cx0 * unit).floor().max(0.0) as usize).min(sw);
        let sx1 = ((cx1 * unit).ceil().max(0.0) as usize).min(sw);
        let sy0 = ((cy0 * unit).floor().max(0.0) as usize).min(self.h * SS);
        let sy1 = ((cy1 * unit).ceil().max(0.0) as usize).min(self.h * SS);
        for sy in sy0..sy1 {
            for sx in sx0..sx1 {
                let (x, y) = ((sx as f32 + 0.5) / unit, (sy as f32 + 0.5) / unit);
                if !inside(x, y) {
                    continue;
                }
                let k = shade(x, y);
                let d = &mut self.buf[sy * sw + sx];
                let a = col[3];
                for c in 0..3 {
                    d[c] = d[c] * (1.0 - a) + (col[c] * k).min(1.0) * a;
                }
                d[3] = d[3] + a * (1.0 - d[3]);
            }
        }
    }
    fn solid(&mut self, inside: impl Fn(f32, f32) -> bool, col: [f32; 4]) {
        self.fill(inside, |_, _| 1.0, col);
    }
    /// Box-filter down to the sprite, add a soft dark outline so the
    /// silhouette holds against the sea, optionally flipped nose-down.
    fn finish(&self, flip: bool) -> Vec<u8> {
        let (w, h) = (self.w, self.h);
        let sw = w * SS;
        let mut px = vec![[0.0f32; 4]; w * h];
        for y in 0..h {
            for x in 0..w {
                let mut acc = [0.0f32; 4];
                for j in 0..SS {
                    for i in 0..SS {
                        let d = self.buf[(y * SS + j) * sw + x * SS + i];
                        for c in 0..3 {
                            acc[c] += d[c] * d[3];
                        }
                        acc[3] += d[3];
                    }
                }
                let a = acc[3] / (SS * SS) as f32;
                px[y * w + x] =
                    if acc[3] > 0.0 { [acc[0] / acc[3], acc[1] / acc[3], acc[2] / acc[3], a] } else { [0.0; 4] };
            }
        }
        let mut img = Image::new(w as u32, h as u32);
        for y in 0..h {
            for x in 0..w {
                let p = px[y * w + x];
                let mut near = 0.0f32;
                for (dx, dy) in [(-1i32, 0i32), (1, 0), (0, -1), (0, 1)] {
                    let (nx, ny) = (x as i32 + dx, y as i32 + dy);
                    if nx >= 0 && ny >= 0 && (nx as usize) < w && (ny as usize) < h {
                        near = near.max(px[ny as usize * w + nx as usize][3]);
                    }
                }
                let outline = (near - p[3]).max(0.0) * 0.55;
                let a = p[3] + outline * (1.0 - p[3]);
                let rgb = if a > 0.0 { [p[0] * p[3] / a, p[1] * p[3] / a, p[2] * p[3] / a] } else { [0.0; 3] };
                let oy = if flip { h - 1 - y } else { y } as i32;
                img.set_rgba(
                    x as i32,
                    oy,
                    (rgb[0] * 255.0) as u8,
                    (rgb[1] * 255.0) as u8,
                    (rgb[2] * 255.0) as u8,
                    (a * 255.0).round() as u8,
                );
            }
        }
        img.encode_png()
    }
}

fn rgb(c: (u8, u8, u8)) -> [f32; 4] {
    [c.0 as f32 / 255.0, c.1 as f32 / 255.0, c.2 as f32 / 255.0, 1.0]
}
fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

/// A lifting surface either side of the centreline `cx`: leading / trailing
/// edge at the root and at the tip, and how much of the outer span is
/// rounded off (0 = square tip, ~0.35 = the Japanese elliptical tips).
#[derive(Clone, Copy)]
struct Surface {
    half_span: f32,
    root: (f32, f32),
    tip: (f32, f32),
    round: f32,
}

impl Surface {
    fn contains(&self, cx: f32, x: f32, y: f32) -> bool {
        let t = (x - cx).abs() / self.half_span;
        if t > 1.0 {
            return false;
        }
        let (mut le, mut te) = (lerp(self.root.0, self.tip.0, t), lerp(self.root.1, self.tip.1, t));
        if self.round > 0.0 && t > 1.0 - self.round {
            let u = (t - (1.0 - self.round)) / self.round;
            let (mid, half) = ((le + te) / 2.0, (te - le) / 2.0 * (1.0 - u * u).max(0.0).sqrt());
            le = mid - half;
            te = mid + half;
        }
        y >= le && y <= te
    }
    /// Lit leading edge, darker toward the trailing edge and the tip.
    fn shade(&self, cx: f32, x: f32, y: f32) -> f32 {
        let t = (x - cx).abs() / self.half_span;
        let (le, te) = (lerp(self.root.0, self.tip.0, t), lerp(self.root.1, self.tip.1, t));
        let c = ((y - le) / (te - le).max(0.1)).clamp(0.0, 1.0);
        1.12 - 0.22 * c - 0.08 * t
    }
}

/// One fighter seen from above, nose up.
struct Fighter {
    w: i32,
    /// fuselage: half width at its widest, where that is, and the tail's
    body_half: f32,
    widest: f32,
    tail_half: f32,
    tail_y: f32,
    /// radial cowl (round, blunt) or inline nose (pointed), half width, length
    radial: bool,
    nose_half: f32,
    nose_len: f32,
    wing: Surface,
    tailplane: Surface,
    /// canopy centre y, half length, half width
    canopy: (f32, f32, f32),
    body: (u8, u8, u8),
    wing_col: (u8, u8, u8),
    nose_col: (u8, u8, u8),
    spinner: (u8, u8, u8),
    /// gun ports along the leading edge, as fractions of the half span
    guns: &'static [f32],
}

impl Fighter {
    fn fuselage_half(&self, y: f32) -> f32 {
        if y < 0.0 || y > self.tail_y {
            return -1.0;
        }
        if y < self.widest {
            let s = y / self.widest;
            if self.radial {
                // a blunt round cowl that is nearly full width at once
                lerp(self.nose_half, self.body_half, s) * (1.0 - (1.0 - (s * 4.0).min(1.0)).powi(2) * 0.55)
            } else {
                lerp(0.9, self.body_half, s.sqrt())
            }
        } else {
            lerp(self.body_half, self.tail_half, (y - self.widest) / (self.tail_y - self.widest))
        }
    }

    fn paint(&self, c: &mut Canvas) {
        let cx = self.w as f32 / 2.0;
        // propeller disc: a faint blur ahead of the nose
        c.solid(
            |x, y| ((x - cx) / (self.wing.half_span * 0.36)).powi(2) + ((y - 1.0) / 0.9).powi(2) <= 1.0,
            [0.75, 0.75, 0.72, 0.28],
        );
        let (wing, tp) = (self.wing, self.tailplane);
        c.fill(|x, y| tp.contains(cx, x, y), |x, y| tp.shade(cx, x, y), rgb(self.wing_col));
        c.fill(|x, y| wing.contains(cx, x, y), |x, y| wing.shade(cx, x, y), rgb(self.wing_col));
        // control-surface hinge line along the trailing edge
        c.solid(
            |x, y| {
                let t = (x - cx).abs() / wing.half_span;
                let te = lerp(wing.root.1, wing.tip.1, t)
                    - (lerp(wing.root.1, wing.tip.1, t) - lerp(wing.root.0, wing.tip.0, t)) * 0.22;
                t > 0.18 && t < 0.9 && (y - te).abs() < 0.28 && wing.contains(cx, x, y)
            },
            [0.0, 0.0, 0.0, 0.22],
        );
        // fuselage, shaded as a cylinder lit from above
        c.fill(
            |x, y| (x - cx).abs() <= self.fuselage_half(y),
            |x, y| {
                let hw = self.fuselage_half(y).max(0.3);
                let u = ((x - cx) / hw).clamp(-1.0, 1.0);
                0.72 + 0.42 * (1.0 - u * u).sqrt()
            },
            rgb(self.body),
        );
        // nose / cowl colour over the front of the fuselage
        c.fill(
            |x, y| y < self.nose_len && (x - cx).abs() <= self.fuselage_half(y),
            |x, y| {
                let hw = self.fuselage_half(y).max(0.3);
                let u = ((x - cx) / hw).clamp(-1.0, 1.0);
                0.7 + 0.45 * (1.0 - u * u).sqrt()
            },
            rgb(self.nose_col),
        );
        // spinner
        // spinner: small in the middle of a radial's cowl ring, long on an inline nose
        let (sw, sl) = if self.radial { (0.75, 0.9) } else { (1.1, 1.5) };
        c.solid(|x, y| ((x - cx) / sw).powi(2) + ((y - sl * 0.7) / sl).powi(2) <= 1.0, rgb(self.spinner));
        // canopy glass with a highlight
        let (cy, cl, cw) = self.canopy;
        c.solid(|x, y| ((x - cx) / cw).powi(2) + ((y - cy) / cl).powi(2) <= 1.0, [0.16, 0.24, 0.32, 1.0]);
        c.solid(
            |x, y| ((x - cx + cw * 0.3) / (cw * 0.35)).powi(2) + ((y - cy + cl * 0.25) / (cl * 0.45)).powi(2) <= 1.0,
            [0.75, 0.88, 1.0, 0.85],
        );
        self.details(c);
    }

    /// The small things that make it read as a machine: panel lines,
    /// ailerons and elevators, gun ports, exhausts and their soot, the
    /// canopy frame, the fin seen edge-on.
    fn details(&self, c: &mut Canvas) {
        let cx = self.w as f32 / 2.0;
        let (wing, tp) = (self.wing, self.tailplane);
        let line = [0.0, 0.0, 0.0, 0.2];
        // chordwise panel lines and the main spar
        for t in [0.34f32, 0.62, 0.82] {
            c.solid(
                |x, y| ((x - cx).abs() / wing.half_span - t).abs() * wing.half_span < 0.13 && wing.contains(cx, x, y),
                line,
            );
        }
        c.solid(
            |x, y| {
                let t = (x - cx).abs() / wing.half_span;
                let (le, te) = (lerp(wing.root.0, wing.tip.0, t), lerp(wing.root.1, wing.tip.1, t));
                (y - (le + (te - le) * 0.3)).abs() < 0.12 && t > 0.12 && wing.contains(cx, x, y)
            },
            [0.0, 0.0, 0.0, 0.12],
        );
        // elevator hinge
        c.solid(
            |x, y| {
                let t = (x - cx).abs() / tp.half_span;
                let (le, te) = (lerp(tp.root.0, tp.tip.0, t), lerp(tp.root.1, tp.tip.1, t));
                (y - (te - (te - le) * 0.35)).abs() < 0.14 && t > 0.1 && tp.contains(cx, x, y)
            },
            [0.0, 0.0, 0.0, 0.25],
        );
        // the fin, edge-on down the tail
        let (fy0, fy1) = (tp.root.0 - 1.0, self.tail_y);
        c.solid(|x, y| y > fy0 && y < fy1 && (x - cx).abs() < 0.35, [0.1, 0.1, 0.1, 0.45]);
        // fuselage panel bands
        for yb in [self.widest + 0.5, self.widest + (self.tail_y - self.widest) * 0.5] {
            c.solid(|x, y| (y - yb).abs() < 0.12 && (x - cx).abs() <= self.fuselage_half(y), line);
        }
        // gun ports: dark muzzles just behind the leading edge
        for &t in self.guns {
            for sgn in [-1.0f32, 1.0] {
                let gx = cx + sgn * t * wing.half_span;
                let le = lerp(wing.root.0, wing.tip.0, t);
                c.solid(
                    |x, y| ((x - gx) / 0.3).powi(2) + ((y - le - 0.3) / 0.45).powi(2) <= 1.0,
                    [0.05, 0.05, 0.05, 0.9],
                );
            }
        }
        // exhausts: stubs down the nose of an inline, a soot streak behind a radial's cowl
        if self.radial {
            for sgn in [-1.0f32, 1.0] {
                c.solid(
                    |x, y| {
                        y > self.nose_len
                            && y < self.nose_len + 3.5
                            && ((x - cx) * sgn - self.fuselage_half(y) * 0.72).abs() < 0.35
                    },
                    [0.08, 0.07, 0.06, 0.5],
                );
            }
        } else {
            for k in 0..4 {
                let ey = 3.2 + k as f32 * 0.9;
                for sgn in [-1.0f32, 1.0] {
                    let ex = cx + sgn * (self.fuselage_half(ey) + 0.1);
                    c.solid(|x, y| ((x - ex) / 0.35).powi(2) + ((y - ey) / 0.3).powi(2) <= 1.0, [0.12, 0.1, 0.08, 0.9]);
                }
            }
            for sgn in [-1.0f32, 1.0] {
                c.solid(
                    |x, y| y > 6.5 && y < 10.0 && ((x - cx) * sgn - self.fuselage_half(y) * 0.85).abs() < 0.3,
                    [0.1, 0.09, 0.08, 0.35],
                );
            }
        }
        // canopy frame
        let (cy, cl, cw) = self.canopy;
        for fy in [cy - cl * 0.35, cy + cl * 0.25] {
            c.solid(
                |x, y| (y - fy).abs() < 0.14 && ((x - cx) / cw).powi(2) + ((y - cy) / cl).powi(2) <= 1.0,
                [0.1, 0.1, 0.12, 0.8],
            );
        }
        // the propeller: two faint blade arcs in the disc
        let r = wing.half_span * 0.36;
        c.solid(
            |x, y| {
                let (dx, dy) = ((x - cx) / r, (y - 1.0) / 0.9);
                let d = dx * dx + dy * dy;
                d <= 1.0 && d > 0.8
            },
            [0.9, 0.9, 0.88, 0.25],
        );
    }
}

/// A US star-and-bar on the port wing, as the P-51 carried it.
fn us_star(c: &mut Canvas, x0: f32, y0: f32) {
    let r = 2.1;
    let blue = [0.10, 0.16, 0.42, 1.0];
    c.solid(|x, y| (x - x0).abs() <= r + 1.8 && (y - y0).abs() <= 0.75, blue);
    c.solid(|x, y| (x - x0).abs() <= r + 1.4 && (y - y0).abs() <= 0.42, [1.0, 1.0, 1.0, 1.0]);
    c.solid(|x, y| (x - x0).powi(2) + (y - y0).powi(2) <= r * r, blue);
    c.solid(|x, y| star(x - x0, y - y0, r * 0.95), [1.0, 1.0, 1.0, 1.0]);
}

/// Inside a five-pointed star of outer radius `r`, point up.
fn star(x: f32, y: f32, r: f32) -> bool {
    let pts: Vec<(f32, f32)> = (0..10)
        .map(|i| {
            let a = -std::f32::consts::FRAC_PI_2 + i as f32 * PI / 5.0;
            let rr = if i % 2 == 0 { r } else { r * 0.4 };
            (a.cos() * rr, a.sin() * rr)
        })
        .collect();
    let mut inside = false;
    let mut j = pts.len() - 1;
    for i in 0..pts.len() {
        let ((xi, yi), (xj, yj)) = (pts[i], pts[j]);
        if (yi > y) != (yj > y) && x < (xj - xi) * (y - yi) / (yj - yi) + xi {
            inside = !inside;
        }
        j = i;
    }
    inside
}

/// A hinomaru with a thin white surround, on each wing.
fn hinomaru(c: &mut Canvas, cx: f32, dx: f32, y0: f32, r: f32) {
    for x0 in [cx - dx, cx + dx] {
        c.solid(|x, y| (x - x0).powi(2) + (y - y0).powi(2) <= (r + 0.55).powi(2), [0.97, 0.97, 0.95, 1.0]);
        c.solid(|x, y| (x - x0).powi(2) + (y - y0).powi(2) <= r * r, [0.80, 0.08, 0.10, 1.0]);
    }
}

/// Player: a P-51D Mustang in natural metal — straight tapered wings with
/// squarish tips, a long inline nose with an olive anti-glare panel, a
/// bubble canopy, red spinner and yellow nose band, star on the port wing.
fn player_plane() -> Vec<u8> {
    // designed at 36x32, rendered at 1.5x
    let (w, h) = (36, 32);
    let mut c = Canvas::scaled(PLAYER_W, PLAYER_H, PLAYER_W as f32 / 36.0);
    let _ = h;
    let cx = w as f32 / 2.0;
    let metal = (196, 202, 212);
    let f = Fighter {
        w,
        body_half: 2.5,
        widest: 11.0,
        tail_half: 0.7,
        tail_y: 31.5,
        radial: false,
        nose_half: 1.2,
        nose_len: 4.2,
        wing: Surface { half_span: 17.6, root: (10.0, 18.6), tip: (11.4, 15.0), round: 0.06 },
        tailplane: Surface { half_span: 6.6, root: (25.6, 30.4), tip: (26.9, 29.4), round: 0.12 },
        canopy: (13.2, 2.6, 1.45),
        body: metal,
        wing_col: (206, 212, 222),
        nose_col: (228, 196, 40),
        spinner: (200, 40, 36),
        guns: &[0.38, 0.43, 0.48], // six .50s, three a side
    };
    f.paint(&mut c);
    // olive-drab anti-glare panel from the nose band to the windscreen
    c.solid(|x, y| y > 4.2 && y < 10.8 && (x - cx).abs() <= 0.8, [0.34, 0.36, 0.22, 1.0]);
    us_star(&mut c, cx - 11.5, 13.6);
    c.finish(false)
}

/// Draw an enemy fighter: kind 0 Zero, 1 Ki-43, 2 Ki-84 ace (drops a power-up).
fn enemy_plane(kind: usize) -> Vec<u8> {
    // designed at 26x22, rendered at 2x
    let w = 26;
    let mut c = Canvas::scaled(ENEMY_W, ENEMY_H, ENEMY_W as f32 / 26.0);
    let cx = w as f32 / 2.0;
    let (body, wing) = enemy_colors(kind);
    let f = match kind {
        0 => Fighter {
            w,
            body_half: 1.9,
            widest: 4.5,
            tail_half: 0.5,
            tail_y: 20.5,
            radial: true,
            nose_half: 1.8,
            nose_len: 3.4,
            wing: Surface { half_span: 12.8, root: (5.6, 11.4), tip: (7.4, 10.4), round: 0.38 },
            tailplane: Surface { half_span: 5.0, root: (16.6, 20.0), tip: (17.6, 19.4), round: 0.45 },
            canopy: (8.6, 2.3, 1.1),
            body,
            wing_col: wing,
            nose_col: (38, 38, 42),
            spinner: (150, 110, 70),
            guns: &[0.42], // a 20 mm cannon in each wing
        },
        1 => Fighter {
            w,
            body_half: 1.6,
            widest: 4.2,
            tail_half: 0.45,
            tail_y: 19.8,
            radial: true,
            nose_half: 1.5,
            nose_len: 2.8,
            wing: Surface { half_span: 11.8, root: (5.4, 10.8), tip: (7.8, 9.8), round: 0.3 },
            tailplane: Surface { half_span: 4.4, root: (16.2, 19.2), tip: (17.0, 18.8), round: 0.4 },
            canopy: (8.4, 2.4, 0.95),
            body,
            wing_col: wing,
            nose_col: (60, 60, 58),
            spinner: (150, 110, 70),
            guns: &[],
        },
        _ => Fighter {
            w,
            body_half: 2.1,
            widest: 5.0,
            tail_half: 0.55,
            tail_y: 21.0,
            radial: true,
            nose_half: 1.6,
            nose_len: 3.2,
            wing: Surface { half_span: 12.0, root: (6.0, 11.6), tip: (7.6, 10.4), round: 0.32 },
            tailplane: Surface { half_span: 4.8, root: (16.8, 20.4), tip: (17.8, 19.8), round: 0.4 },
            canopy: (9.0, 2.2, 1.15),
            body,
            wing_col: wing,
            nose_col: (52, 56, 44),
            spinner: (160, 120, 70),
            guns: &[0.36, 0.44],
        },
    };
    f.paint(&mut c);
    // IJN/IJA yellow ID stripe on the inboard leading edges
    let wg = f.wing;
    c.solid(
        |x, y| {
            let t = (x - cx).abs() / wg.half_span;
            t > 0.14 && t < 0.5 && wg.contains(cx, x, y) && y < lerp(wg.root.0, wg.tip.0, t) + 0.9
        },
        [0.95, 0.72, 0.12, if kind == 2 { 1.0 } else { 0.8 }],
    );
    hinomaru(&mut c, cx, wg.half_span * 0.62, lerp(wg.root.0, wg.root.1, 0.5) + 0.4, 1.7);
    c.finish(true)
}

/// Enemy body and wing colours maintain at least 3:1 contrast against both sky colours.
/// `enemy_planes_are_visible_against_the_sky` checks the contrast floor.
pub(super) fn enemy_colors(kind: usize) -> ((u8, u8, u8), (u8, u8, u8)) {
    let body: (u8, u8, u8) = match kind {
        0 => (125, 175, 115),
        1 => (205, 158, 68),
        _ => (214, 218, 224), // natural metal: the ace's Ki-84 is left unpainted
    };
    let wing = (body.0.saturating_add(38), body.1.saturating_add(38), body.2.saturating_add(38));
    (body, wing)
}

/// A heavy aircraft seen from above, nose up, in design px (= sprite px).
/// Every length is laid out from real planforms of the type.
struct Bomber {
    w: i32,
    fus_half: f32,
    fus_widest: f32,
    tail_y: f32,
    nose_glaze: f32,
    hull: bool,
    floats: bool,
    wing: Surface,
    engines: &'static [f32], // nacelle positions, fractions of the half span
    cowl: f32,
    nacelle_len: f32,
    tailplane: Surface,
    fins: &'static [f32], // fin positions, fractions of the tailplane half span
    turrets: &'static [(f32, f32)],
}

impl Bomber {
    fn fuselage_half(&self, y: f32) -> f32 {
        if y < 0.0 || y > self.tail_y {
            return -1.0;
        }
        let hw = if self.hull { self.fus_half * 1.25 } else { self.fus_half };
        if y < self.fus_widest {
            let s = y / self.fus_widest;
            hw * (1.0 - (1.0 - s).powi(2)).sqrt().max(0.25)
        } else {
            lerp(hw, hw * 0.3, ((y - self.fus_widest) / (self.tail_y - self.fus_widest)).powf(1.3))
        }
    }

    fn paint(&self, c: &mut Canvas) {
        let cx = self.w as f32 / 2.0;
        let hs = self.wing.half_span;
        let (wing, tp) = (self.wing, self.tailplane);
        let green = (104, 132, 92);
        let green_w = (116, 146, 102);
        let le = |t: f32| lerp(wing.root.0, wing.tip.0, t);
        let te = |t: f32| lerp(wing.root.1, wing.tip.1, t);
        // propeller discs ahead of each engine
        for &t in self.engines {
            for sgn in [-1.0f32, 1.0] {
                let (ex, ey) = (cx + sgn * t * hs, le(t) - self.cowl * 1.4);
                let r = self.cowl * 2.3;
                c.solid(|x, y| ((x - ex) / r).powi(2) + ((y - ey) / (r * 0.18)).powi(2) <= 1.0, [0.8, 0.8, 0.78, 0.22]);
            }
        }
        // tail surfaces and fins
        c.fill(|x, y| tp.contains(cx, x, y), |x, y| tp.shade(cx, x, y), rgb(green_w));
        c.solid(
            |x, y| {
                let t = (x - cx).abs() / tp.half_span;
                let (l, e) = (lerp(tp.root.0, tp.tip.0, t), lerp(tp.root.1, tp.tip.1, t));
                (y - (e - (e - l) * 0.35)).abs() < 0.4 && t > 0.1 && tp.contains(cx, x, y)
            },
            [0.0, 0.0, 0.0, 0.25],
        );
        // wings, with floats under the tips of a flying boat
        c.fill(|x, y| wing.contains(cx, x, y), |x, y| wing.shade(cx, x, y), rgb(green_w));
        if self.floats {
            for sgn in [-1.0f32, 1.0] {
                let (fx, fy) = (cx + sgn * 0.84 * hs, (le(0.84) + te(0.84)) / 2.0);
                c.solid(
                    |x, y| ((x - fx) / (self.cowl * 0.8)).powi(2) + ((y - fy) / (self.cowl * 2.4)).powi(2) <= 1.0,
                    [0.55, 0.6, 0.52, 1.0],
                );
            }
        }
        // panel lines, spar, flaps and ailerons
        let line = [0.0, 0.0, 0.0, 0.18];
        for k in 1..8 {
            let t = k as f32 / 8.0;
            c.solid(|x, y| ((x - cx).abs() / hs - t).abs() * hs < 0.35 && wing.contains(cx, x, y), line);
        }
        c.solid(
            |x, y| {
                let t = (x - cx).abs() / hs;
                (y - (le(t) + (te(t) - le(t)) * 0.3)).abs() < 0.3 && wing.contains(cx, x, y)
            },
            [0.0, 0.0, 0.0, 0.1],
        );
        c.solid(
            |x, y| {
                let t = (x - cx).abs() / hs;
                (y - (te(t) - (te(t) - le(t)) * 0.25)).abs() < 0.4 && t > 0.12 && wing.contains(cx, x, y)
            },
            [0.0, 0.0, 0.0, 0.28],
        );
        // yellow ID stripe on the inboard leading edge
        c.solid(
            |x, y| {
                let t = (x - cx).abs() / hs;
                t > 0.08 && t < 0.3 && wing.contains(cx, x, y) && y < le(t) + 1.6
            },
            [0.95, 0.72, 0.12, 0.85],
        );
        // engine nacelles: cowl ring forward, nacelle over the wing, soot aft
        for &t in self.engines {
            for sgn in [-1.0f32, 1.0] {
                let ex = cx + sgn * t * hs;
                let (y0, y1) = (le(t) - self.cowl * 1.3, le(t) + self.nacelle_len);
                c.fill(
                    |x, y| {
                        y > y0 && y < y1 && ((x - ex).abs() / self.cowl) <= (1.0 - ((y - y0) / (y1 - y0)).powi(3) * 0.7)
                    },
                    |x, _| 0.7 + 0.4 * (1.0 - ((x - ex) / self.cowl).powi(2)).max(0.0).sqrt(),
                    rgb(green),
                );
                c.solid(
                    |x, y| {
                        ((x - ex) / self.cowl).powi(2) + ((y - y0 - self.cowl * 0.3) / (self.cowl * 0.45)).powi(2)
                            <= 1.0
                    },
                    [0.12, 0.12, 0.13, 1.0],
                );
                c.solid(
                    |x, y| {
                        ((x - ex) / (self.cowl * 0.3)).powi(2)
                            + ((y - y0 - self.cowl * 0.1) / (self.cowl * 0.3)).powi(2)
                            <= 1.0
                    },
                    [0.5, 0.45, 0.35, 1.0],
                );
                c.solid(
                    |x, y| y > y1 - 1.0 && y < y1 + self.nacelle_len * 0.18 && (x - ex).abs() < self.cowl * 0.3,
                    [0.08, 0.07, 0.06, 0.16],
                );
            }
        }
        // fins, edge-on
        for &f in self.fins {
            for sgn in [-1.0f32, 1.0] {
                let fx = cx + sgn * f * tp.half_span;
                if f == 0.0 && sgn > 0.0 {
                    continue;
                }
                c.solid(
                    |x, y| y > tp.root.0 - 3.0 && y < self.tail_y + 1.0 && (x - fx).abs() < 0.8,
                    [0.2, 0.26, 0.18, 0.95],
                );
            }
        }
        // fuselage (a boat hull is wider, with a keel line)
        c.fill(
            |x, y| (x - cx).abs() <= self.fuselage_half(y),
            |x, y| {
                let hw = self.fuselage_half(y).max(0.3);
                let u = ((x - cx) / hw).clamp(-1.0, 1.0);
                0.7 + 0.42 * (1.0 - u * u).sqrt()
            },
            rgb(green),
        );
        if self.hull {
            c.solid(|x, y| (x - cx).abs() < 0.5 && y > self.fus_widest && y < self.tail_y * 0.8, [0.0, 0.0, 0.0, 0.2]);
        }
        for k in 1..6 {
            let yb = self.tail_y * k as f32 / 6.0;
            c.solid(|x, y| (y - yb).abs() < 0.3 && (x - cx).abs() <= self.fuselage_half(y), line);
        }
        // glazed nose with its frames, then the cockpit
        c.solid(|x, y| y < self.nose_glaze && (x - cx).abs() <= self.fuselage_half(y) * 0.92, [0.3, 0.42, 0.5, 1.0]);
        for k in 1..4 {
            let fy = self.nose_glaze * k as f32 / 4.0;
            c.solid(
                |x, y| (y - fy).abs() < 0.3 && (x - cx).abs() <= self.fuselage_half(y) * 0.92,
                [0.15, 0.18, 0.2, 0.9],
            );
        }
        c.solid(|x, y| y < self.nose_glaze && (x - cx).abs() < 0.3, [0.15, 0.18, 0.2, 0.9]);
        let (cy, cl) = (self.nose_glaze + self.fus_half * 1.6, self.fus_half * 1.1);
        c.solid(
            |x, y| ((x - cx) / (self.fus_half * 0.55)).powi(2) + ((y - cy) / cl).powi(2) <= 1.0,
            [0.2, 0.28, 0.36, 1.0],
        );
        c.solid(
            |x, y| {
                ((x - cx + self.fus_half * 0.18) / (self.fus_half * 0.2)).powi(2)
                    + ((y - cy + cl * 0.3) / (cl * 0.4)).powi(2)
                    <= 1.0
            },
            [0.8, 0.9, 1.0, 0.7],
        );
        // gun turrets: glazed domes with twin barrels
        let h = self.tail_y / 0.99;
        for &(tx, ty) in self.turrets {
            let (gx, gy) = (cx + tx * hs, ty * h);
            let r = (self.fus_half * 0.55).max(2.2);
            let back = ty > 0.9;
            let dir = if back { 1.0 } else { -1.0 };
            for off in [-0.9f32, 0.9] {
                c.solid(
                    |x, y| (x - gx - off).abs() < 0.45 && (y - gy) * dir > 0.0 && (y - gy).abs() < r * 2.4,
                    [0.08, 0.08, 0.08, 1.0],
                );
            }
            c.solid(|x, y| ((x - gx) / r).powi(2) + ((y - gy) / r).powi(2) <= 1.0, [0.35, 0.48, 0.56, 1.0]);
            c.solid(
                |x, y| ((x - gx + r * 0.3) / (r * 0.35)).powi(2) + ((y - gy - r * 0.3) / (r * 0.35)).powi(2) <= 1.0,
                [0.85, 0.95, 1.0, 0.7],
            );
        }
        // hinomaru on each wing
        let t = 0.68;
        let chord = te(t) - le(t);
        hinomaru(c, cx, t * hs, le(t) + chord * 0.5, chord * 0.28);
    }
}

/// The seven bosses, smallest to largest: Ki-49 Donryu, Ki-67 Hiryu,
/// G4M "Betty", G8N Renzan, H8K flying boat, G5N Shinzan and the six-engine
/// Fugaku. Drawn nose-up and flipped so they fly at the player.
fn boss_plane(tier: usize) -> Vec<u8> {
    let (w, h) = BOSS_SIZES[tier];
    let (hs, l) = (w as f32 / 2.0, h as f32);
    let wing = |rle: f32, rte: f32, tle: f32, tte: f32, round: f32| Surface {
        half_span: hs * 0.98,
        root: (l * rle, l * rte),
        tip: (l * tle, l * tte),
        round,
    };
    let tailp = |span: f32, y0: f32, y1: f32| Surface {
        half_span: hs * span,
        root: (l * y0, l * y1),
        tip: (l * (y0 + 0.03), l * (y1 - 0.02)),
        round: 0.3,
    };
    let b = match tier {
        0 => Bomber {
            w,
            fus_half: 5.5,
            fus_widest: l * 0.3,
            tail_y: l * 0.98,
            nose_glaze: l * 0.1,
            hull: false,
            floats: false,
            wing: wing(0.30, 0.52, 0.37, 0.46, 0.2),
            engines: &[0.3],
            cowl: 4.2,
            nacelle_len: l * 0.2,
            tailplane: tailp(0.34, 0.84, 0.95),
            fins: &[0.0],
            turrets: BOSS_TURRETS[0],
        },
        1 => Bomber {
            w,
            fus_half: 5.2,
            fus_widest: l * 0.3,
            tail_y: l * 0.98,
            nose_glaze: l * 0.1,
            hull: false,
            floats: false,
            wing: wing(0.30, 0.50, 0.37, 0.45, 0.25),
            engines: &[0.3],
            cowl: 4.4,
            nacelle_len: l * 0.22,
            tailplane: tailp(0.34, 0.84, 0.95),
            fins: &[0.0],
            turrets: BOSS_TURRETS[1],
        },
        2 => Bomber {
            w,
            fus_half: 7.5,
            fus_widest: l * 0.35,
            tail_y: l * 0.99,
            nose_glaze: l * 0.12,
            hull: false,
            floats: false,
            wing: wing(0.32, 0.54, 0.40, 0.48, 0.35),
            engines: &[0.28],
            cowl: 5.0,
            nacelle_len: l * 0.2,
            tailplane: tailp(0.3, 0.84, 0.95),
            fins: &[0.0],
            turrets: BOSS_TURRETS[2],
        },
        3 => Bomber {
            w,
            fus_half: 6.5,
            fus_widest: l * 0.3,
            tail_y: l * 0.98,
            nose_glaze: l * 0.1,
            hull: false,
            floats: false,
            wing: wing(0.30, 0.52, 0.38, 0.46, 0.15),
            engines: &[0.22, 0.5],
            cowl: 4.8,
            nacelle_len: l * 0.18,
            tailplane: tailp(0.3, 0.84, 0.95),
            fins: &[0.0],
            turrets: BOSS_TURRETS[3],
        },
        4 => Bomber {
            w,
            fus_half: 7.5,
            fus_widest: l * 0.28,
            tail_y: l * 0.98,
            nose_glaze: l * 0.07,
            hull: true,
            floats: true,
            wing: wing(0.30, 0.46, 0.34, 0.41, 0.2),
            engines: &[0.2, 0.42],
            cowl: 5.0,
            nacelle_len: l * 0.14,
            tailplane: tailp(0.26, 0.86, 0.96),
            fins: &[0.0],
            turrets: BOSS_TURRETS[4],
        },
        5 => Bomber {
            w,
            fus_half: 7.0,
            fus_widest: l * 0.3,
            tail_y: l * 0.97,
            nose_glaze: l * 0.09,
            hull: false,
            floats: false,
            wing: wing(0.30, 0.48, 0.36, 0.43, 0.15),
            engines: &[0.2, 0.45],
            cowl: 5.2,
            nacelle_len: l * 0.16,
            tailplane: tailp(0.3, 0.85, 0.95),
            fins: &[0.85],
            turrets: BOSS_TURRETS[5],
        },
        _ => Bomber {
            w,
            fus_half: 9.0,
            fus_widest: l * 0.28,
            tail_y: l * 0.98,
            nose_glaze: l * 0.07,
            hull: false,
            floats: false,
            wing: wing(0.30, 0.46, 0.36, 0.41, 0.15),
            engines: &[0.14, 0.3, 0.46],
            cowl: 5.6,
            nacelle_len: l * 0.14,
            tailplane: tailp(0.24, 0.86, 0.96),
            fins: &[0.0],
            turrets: BOSS_TURRETS[6],
        },
    };
    let mut c = Canvas::new(w, h);
    b.paint(&mut c);
    c.finish(true)
}

/// Power-up capsule dropped by the ace: a glowing diamond with a bright core.
fn powerup_capsule() -> Vec<u8> {
    let (w, h) = (POW_W, POW_H);
    let mut img = Image::new(w as u32, h as u32);
    let cx = w as f32 / 2.0;
    let cy = h as f32 / 2.0;
    for y in 0..h {
        for x in 0..w {
            let dx = (x as f32 - cx).abs();
            let dy = (y as f32 - cy).abs();
            let d = dx + dy; // diamond metric
            if d <= cx.min(cy) {
                img.set(x, y, 40, 230, 120);
            }
            if d <= cx.min(cy) * 0.45 {
                img.set(x, y, 255, 255, 255);
            }
        }
    }
    img.encode_png()
}

/// Health pickup, from regular fighters: a white roundel with a red cross, a
/// different shape and fixed colour from the weapon capsule's diamond.
fn health_pack() -> Vec<u8> {
    let (w, h) = (HEALTH_W, HEALTH_H);
    let mut img = Image::new(w as u32, h as u32);
    let cx = w as f32 / 2.0;
    let cy = h as f32 / 2.0;
    let r = cx.min(cy) - 0.5;
    let bar_half = r * 0.34;
    for y in 0..h {
        for x in 0..w {
            let dx = x as f32 - cx;
            let dy = y as f32 - cy;
            if dx * dx + dy * dy <= r * r {
                img.set(x, y, 240, 240, 236); // white roundel
            }
            if dx.abs() <= bar_half && dy.abs() <= r * 0.72 {
                img.set(x, y, 220, 50, 50); // the cross's vertical bar
            }
            if dy.abs() <= bar_half && dx.abs() <= r * 0.72 {
                img.set(x, y, 220, 50, 50); // the cross's horizontal bar
            }
        }
    }
    img.encode_png()
}

/// Draw a 1944 Essex-class carrier, scaled to fit the game canvas.
fn carrier_ship() -> Vec<u8> {
    let (w, h) = (CARRIER_W, CARRIER_H);
    let mut c = Canvas::new(w, h);
    let (wf, hf) = (w as f32, h as f32);
    let cx = 88.0; // the deck centreline (the island is to starboard)
    let (dl, dr) = (cx - 75.0, cx + 75.0);
    let (bow, stern) = (42.0, hf - 18.0);
    let hull = [0.24, 0.27, 0.31, 1.0];
    let grey = [0.46, 0.49, 0.53, 1.0];
    let white = [0.92, 0.92, 0.88, 1.0];
    // the hull below the deck: a sharp bow and a rounded stern
    c.clip(0.0, 0.0, wf, 70.0);
    c.solid(|x, y| y > 2.0 && y < 70.0 && (x - cx).abs() < (y - 2.0) * 1.3, hull);
    c.clip(0.0, stern - 20.0, wf, hf);
    c.solid(|x, y| y > stern - 20.0 && ((x - cx) / 72.0).powi(2) + ((y - (stern - 20.0)) / 36.0).powi(2) <= 1.0, hull);
    // sponsons and catwalks along both sides
    c.clip(0.0, 0.0, wf, hf);
    c.solid(
        |x, y| y > bow + 30.0 && y < stern - 10.0 && ((x > dl - 9.0 && x < dl) || (x > dr && x < dr + 9.0)),
        [0.3, 0.33, 0.37, 1.0],
    );
    for k in 0..90 {
        let y0 = bow + 34.0 + k as f32 * 7.4;
        if y0 > stern - 14.0 {
            break;
        }
        c.clip(0.0, y0 - 1.0, wf, y0 + 1.0);
        c.solid(
            |x, y| (y - y0).abs() < 0.5 && ((x - (dl - 8.0)).abs() < 0.6 || (x - (dr + 8.0)).abs() < 0.6),
            [0.62, 0.64, 0.66, 1.0],
        );
    }
    // the deck: squared at the ends, narrower at the bow
    c.clip(0.0, bow - 2.0, wf, stern + 2.0);
    let deck_half = |y: f32| {
        if y < bow + 60.0 {
            lerp(52.0, 75.0, ((y - bow) / 60.0).clamp(0.0, 1.0))
        } else {
            75.0
        }
    };
    let deck = |x: f32, y: f32| y > bow && y < stern && (x - cx).abs() <= deck_half(y);
    c.fill(
        deck,
        |x, _y| {
            // planking: a slightly different tone every strip, seams between
            let strip = ((x - cx + 80.0) / 2.6).floor();
            let tone = 0.94 + 0.06 * ((strip * 12.9898).sin() * 43758.5).fract();
            let seam = if ((x - cx + 80.0) % 2.6) < 0.35 { 0.82 } else { 1.0 };
            tone * seam
        },
        [0.29, 0.34, 0.42, 1.0],
    );
    // tie-down strips across the deck
    for k in 0..40 {
        let y0 = bow + 20.0 + k as f32 * 18.0;
        if y0 > stern - 4.0 {
            break;
        }
        c.clip(0.0, y0 - 1.0, wf, y0 + 1.0);
        c.solid(|x, y| (y - y0).abs() < 0.35 && deck(x, y), [0.55, 0.58, 0.62, 0.28]);
    }
    c.clip(0.0, bow - 2.0, wf, stern + 2.0);
    // deck edge lines
    c.solid(|x, y| deck(x, y) && (deck_half(y) - (x - cx).abs()) < 1.2, [0.8, 0.8, 0.78, 0.8]);
    // elevators: centreline forward and aft, deck-edge to port
    for (y0, y1) in [(bow + 110.0, bow + 158.0), (stern - 196.0, stern - 148.0)] {
        c.clip(cx - 30.0, y0 - 2.0, cx + 30.0, y1 + 2.0);
        c.solid(
            |x, y| {
                y > y0
                    && y < y1
                    && (x - cx).abs() < 25.0
                    && ((y - y0).abs() < 0.8
                        || (y - y1).abs() < 0.8
                        || ((x - cx).abs() - 25.0).abs() < 0.8
                        || (x - cx).abs() > 24.2)
            },
            [0.1, 0.12, 0.15, 0.9],
        );
        c.solid(|x, y| y > y0 + 1.0 && y < y1 - 1.0 && (x - cx).abs() < 24.0, [0.0, 0.0, 0.0, 0.08]);
    }
    let (ey0, ey1) = (bow + 250.0, bow + 292.0);
    c.clip(0.0, ey0 - 2.0, dl + 2.0, ey1 + 2.0);
    c.solid(|x, y| y > ey0 && y < ey1 && x > dl - 20.0 && x < dl + 1.0, [0.29, 0.34, 0.42, 1.0]);
    c.solid(
        |x, y| {
            y > ey0
                && y < ey1
                && x > dl - 20.0
                && x < dl + 1.0
                && ((y - ey0).abs() < 0.8 || (y - ey1).abs() < 0.8 || (x - (dl - 20.0)).abs() < 0.8)
        },
        [0.1, 0.12, 0.15, 0.9],
    );
    // centreline: dashed white
    c.clip(cx - 2.0, bow, cx + 2.0, stern);
    c.solid(|x, y| (x - cx).abs() < 0.9 && y > bow + 50.0 && y < stern - 40.0 && ((y - bow) % 22.0) < 13.0, white);
    // catapult tracks at the bow
    for off in [-34.0f32, 34.0] {
        c.clip(cx + off - 2.0, bow, cx + off + 2.0, bow + 90.0);
        c.solid(|x, y| (x - cx - off).abs() < 0.6 && y > bow + 10.0 && y < bow + 84.0, [0.08, 0.08, 0.1, 0.8]);
    }
    // arresting wires aft, with their sheaves at the deck edge
    for k in 0..9 {
        let y0 = stern - 130.0 + k as f32 * 10.0;
        c.clip(0.0, y0 - 2.0, wf, y0 + 2.0);
        c.solid(|x, y| (y - y0).abs() < 0.45 && deck(x, y) && (x - cx).abs() < 70.0, [0.75, 0.76, 0.74, 0.8]);
        for sgn in [-1.0f32, 1.0] {
            c.solid(|x, y| ((x - cx - sgn * 70.0).powi(2) + (y - y0).powi(2)) < 2.2, [0.12, 0.12, 0.12, 1.0]);
        }
    }
    // barriers forward of the wires
    for y0 in [stern - 150.0, stern - 143.0] {
        c.clip(0.0, y0 - 2.0, wf, y0 + 2.0);
        c.solid(|x, y| (y - y0).abs() < 0.6 && deck(x, y) && (x - cx).abs() < 66.0, [0.85, 0.75, 0.2, 0.8]);
    }
    // the hull number, "9", fore and aft
    let nine = |x: f32, y: f32, x0: f32, y0: f32| -> bool {
        let (u, v) = ((x - x0) / 14.0, (y - y0) / 24.0);
        if !(0.0..=1.0).contains(&u) || !(0.0..=1.0).contains(&v) {
            return false;
        }
        let bar = 0.2;
        let top = v < bar;
        let mid = (v - 0.45).abs() < bar / 2.0;
        let bot = v > 1.0 - bar;
        let left = u < bar * 1.4 && v < 0.5;
        let right = u > 1.0 - bar * 1.4;
        top || mid || (bot && u > 0.2) || left || right
    };
    c.clip(cx - 10.0, bow + 20.0, cx + 10.0, bow + 50.0);
    c.solid(|x, y| nine(x, y, cx - 7.0, bow + 22.0), white);
    c.clip(cx - 10.0, stern - 34.0, cx + 10.0, stern - 4.0);
    // aft, turned round to read from astern
    c.solid(|x, y| nine(cx * 2.0 - x, (stern - 20.0) * 2.0 - y, cx - 7.0, stern - 32.0), white);
    // the island, to starboard, its shadow on the deck
    let (ix0, ix1, iy0, iy1) = (dr - 12.0, dr + 22.0, bow + 200.0, bow + 330.0);
    c.clip(ix0 - 8.0, iy0 - 4.0, ix1 + 2.0, iy1 + 8.0);
    c.solid(|x, y| x > ix0 - 6.0 && x < ix0 && y > iy0 + 6.0 && y < iy1 + 6.0, [0.0, 0.0, 0.0, 0.25]);
    c.fill(
        |x, y| x > ix0 && x < ix1 && y > iy0 && y < iy1 && !(y < iy0 + 20.0 && x < ix0 + (iy0 + 20.0 - y) * 0.5),
        |x, _| 0.85 + 0.25 * ((x - ix0) / (ix1 - ix0)),
        grey,
    );
    // bridge windows, the funnel with its soot, radar and mast
    c.solid(
        |x, y| y > iy0 + 22.0 && y < iy0 + 25.0 && x > ix0 + 3.0 && x < ix1 - 3.0 && ((x - ix0) % 3.5) < 2.0,
        [0.1, 0.14, 0.2, 1.0],
    );
    let (fx, fy) = ((ix0 + ix1) / 2.0 + 2.0, iy0 + 78.0);
    c.solid(|x, y| ((x - fx) / 10.0).powi(2) + ((y - fy) / 20.0).powi(2) <= 1.0, [0.36, 0.38, 0.42, 1.0]);
    c.solid(|x, y| ((x - fx) / 7.0).powi(2) + ((y - fy) / 16.0).powi(2) <= 1.0, [0.06, 0.06, 0.07, 1.0]);
    let (ry, rx) = (iy0 + 44.0, (ix0 + ix1) / 2.0);
    c.solid(
        |x, y| (x - rx).abs() < 9.0 && (y - ry).abs() < 3.0 && (((x - rx + 9.0) % 2.0) < 0.6 || (y - ry).abs() > 2.4),
        [0.7, 0.72, 0.75, 1.0],
    );
    c.solid(|x, y| (x - rx).abs() < 0.6 && y > ry - 10.0 && y < ry + 30.0, [0.2, 0.2, 0.22, 1.0]);
    c.solid(|x, y| (y - (ry - 8.0)).abs() < 0.5 && (x - rx).abs() < 6.0, [0.2, 0.2, 0.22, 1.0]);
    // twin 5-inch mounts, two forward of the island, two aft
    for (my, fwd) in [(iy0 - 36.0, true), (iy0 - 16.0, true), (iy1 + 12.0, false), (iy1 + 32.0, false)] {
        let mx = dr + 4.0;
        c.clip(mx - 12.0, my - 20.0, mx + 12.0, my + 20.0);
        let dir = if fwd { -1.0 } else { 1.0 };
        for off in [-2.2f32, 2.2] {
            c.solid(
                |x, y| (x - mx - off).abs() < 0.8 && (y - my) * dir > 0.0 && (y - my).abs() < 15.0,
                [0.15, 0.15, 0.16, 1.0],
            );
        }
        c.fill(|x, y| ((x - mx) / 8.0).powi(2) + ((y - my) / 6.5).powi(2) <= 1.0, |_, y| 1.1 - 0.02 * (y - my), grey);
    }
    // 40 mm quad mounts in the sponsons and at bow and stern
    let quads = [
        (dl - 5.0, bow + 90.0),
        (dl - 5.0, bow + 210.0),
        (dl - 5.0, bow + 420.0),
        (dl - 5.0, stern - 70.0),
        (dr + 5.0, bow + 140.0),
        (dr + 5.0, bow + 480.0),
        (dr + 5.0, stern - 60.0),
        (cx - 18.0, bow + 4.0),
        (cx + 18.0, bow + 4.0),
        (cx - 30.0, stern + 6.0),
        (cx + 30.0, stern + 6.0),
    ];
    for (qx, qy) in quads {
        c.clip(qx - 8.0, qy - 10.0, qx + 8.0, qy + 8.0);
        c.solid(|x, y| ((x - qx).powi(2) + (y - qy).powi(2)) < 20.0, [0.4, 0.42, 0.45, 1.0]);
        for k in 0..4 {
            let bx = qx - 2.4 + k as f32 * 1.6;
            c.solid(|x, y| (x - bx).abs() < 0.35 && y < qy - 2.0 && y > qy - 8.0, [0.12, 0.12, 0.13, 1.0]);
        }
    }
    // Hellcats spotted aft, wings folded back along their sides
    let hellcat = |c: &mut Canvas, px: f32, py: f32| {
        c.clip(px - 12.0, py - 16.0, px + 12.0, py + 16.0);
        let blue = [0.18, 0.25, 0.4, 1.0];
        c.solid(
            |x, y| (y - py).abs() < 13.0 && (x - px).abs() < 3.4 - ((y - py + 4.0) / 13.0).abs().powi(2) * 1.6,
            blue,
        );
        c.solid(|x, y| y > py - 7.0 && y < py + 9.0 && ((x - px).abs() - 5.5).abs() < 1.6, [0.2, 0.28, 0.44, 1.0]);
        c.solid(|x, y| y > py + 9.0 && y < py + 12.5 && (x - px).abs() < 7.0, blue);
        c.solid(|x, y| ((x - px) / 1.6).powi(2) + ((y - py + 3.0) / 3.0).powi(2) <= 1.0, [0.5, 0.65, 0.8, 0.9]);
        c.solid(|x, y| ((x - px) / 3.2).powi(2) + ((y - py + 12.5) / 1.4).powi(2) <= 1.0, [0.12, 0.12, 0.12, 1.0]);
    };
    for (k, (dx, dy)) in [
        (-44.0f32, -104.0f32),
        (-22.0, -104.0),
        (0.0, -100.0),
        (22.0, -104.0),
        (44.0, -104.0),
        (-33.0, -74.0),
        (-11.0, -72.0),
        (11.0, -72.0),
        (33.0, -74.0),
    ]
    .iter()
    .enumerate()
    {
        let _ = k;
        hellcat(&mut c, cx + dx, stern + dy + 30.0);
    }
    c.unclip();
    c.finish(false)
}

/// An enemy boat, viewed from above: a pointed-bow hull (bow to the right;
/// the game flips the sprite horizontally when it's sailing the other way),
/// a small deckhouse amidships, and a wake ripple trailing the stern.
fn boat() -> Vec<u8> {
    let (w, h) = (BOAT_W, BOAT_H);
    let mut img = Image::new(w as u32, h as u32);
    let cy = h / 2;
    let bow_start = w as f32 * 0.72;
    for x in 0..w {
        let half_h = if x as f32 > bow_start {
            let t = (x as f32 - bow_start) / (w as f32 - bow_start);
            (h as f32 * 0.42) * (1.0 - t)
        } else {
            h as f32 * 0.42
        };
        for y in 0..h {
            if (y - cy).abs() as f32 <= half_h {
                img.set(x, y, 92, 86, 78); // drab hull
            }
        }
    }
    // deckhouse, amidships toward the stern
    let dh_x0 = (w as f32 * 0.30) as i32;
    let dh_x1 = (w as f32 * 0.50) as i32;
    for x in dh_x0..dh_x1 {
        for y in (cy - 3)..(cy + 3) {
            img.set(x, y, 58, 56, 53);
        }
    }
    // a couple of wake ripples trailing the stern
    img.set(2, cy, 190, 210, 230);
    img.set(4, cy - 2, 190, 210, 230);
    img.set(4, cy + 2, 190, 210, 230);
    img.encode_png()
}

// Clouds use masked value-noise fBm for billowing edges and lighting; variants are baked at build time.

const CLOUD_TEX_W: i32 = 40;
const CLOUD_TEX_H: i32 = 26;

/// Cheap 2D hash -> [0, 1). Different `seed`s give unrelated noise fields.
fn cloud_hash(x: i32, y: i32, seed: u32) -> f32 {
    let mut h = (x as u32)
        .wrapping_mul(374_761_393)
        .wrapping_add((y as u32).wrapping_mul(668_265_263))
        .wrapping_add(seed.wrapping_mul(2_246_822_519));
    h = (h ^ (h >> 13)).wrapping_mul(1_274_126_177);
    h ^= h >> 16;
    (h & 0x00FF_FFFF) as f32 / 0x0100_0000 as f32
}

/// Smoothstep-interpolated value noise at a continuous (x, y).
fn value_noise(x: f32, y: f32, seed: u32) -> f32 {
    let (x0, y0) = (x.floor(), y.floor());
    let (fx, fy) = (x - x0, y - y0);
    let (sx, sy) = (fx * fx * (3.0 - 2.0 * fx), fy * fy * (3.0 - 2.0 * fy));
    let (xi, yi) = (x0 as i32, y0 as i32);
    let n00 = cloud_hash(xi, yi, seed);
    let n10 = cloud_hash(xi + 1, yi, seed);
    let n01 = cloud_hash(xi, yi + 1, seed);
    let n11 = cloud_hash(xi + 1, yi + 1, seed);
    let nx0 = n00 + (n10 - n00) * sx;
    let nx1 = n01 + (n11 - n01) * sx;
    nx0 + (nx1 - nx0) * sy
}

/// Fractional Brownian motion: `octaves` layers of value noise, each at
/// double the frequency and half the weight of the last, normalised to
/// roughly [0, 1].
fn fbm(x: f32, y: f32, seed: u32, octaves: u32) -> f32 {
    let (mut total, mut amp, mut freq, mut norm) = (0.0, 0.5, 1.0, 0.0);
    for o in 0..octaves {
        total += value_noise(x * freq, y * freq, seed.wrapping_add(o * 101)) * amp;
        norm += amp;
        amp *= 0.5;
        freq *= 2.0;
    }
    total / norm
}

/// One cloud sprite variant. `seed` picks the noise field (so each variant
/// is a different cloud, not a recolour of the same one).
fn cloud_sprite(seed: u32) -> Vec<u8> {
    let (w, h) = (CLOUD_TEX_W, CLOUD_TEX_H);
    let mut img = Image::new(w as u32, h as u32);
    let cx = w as f32 * 0.5;
    let cy = h as f32 * 0.58; // envelope centred a little low: flatter base, more room to billow up top

    for y in 0..h {
        let vshade = (y as f32 / h as f32).clamp(0.0, 1.0); // 0 at top (lit) -> 1 at base (shadowed)
        for x in 0..w {
            let nx = (x as f32 - cx) / (w as f32 * 0.46);
            let ny = (y as f32 - cy) / (h as f32 * 0.40);
            // Envelope: an ellipse squashed harder below centre than above —
            // the flat-bottomed cumulus silhouette instead of a round blob.
            let ny_shaped = if ny > 0.0 { ny * 1.7 } else { ny * 0.9 };
            let env = (nx * nx + ny_shaped * ny_shaped).sqrt();

            let n = fbm(x as f32 * 0.22, y as f32 * 0.22, seed, 4);
            // More broken-up/turbulent silhouette toward the top than the base.
            let top_bias = (0.5 - y as f32 / h as f32).max(0.0) * 0.7;
            let shape = (1.0 - env) + (n - 0.5) * (0.5 + top_bias);

            if shape <= 0.05 {
                continue;
            }
            let edge = ((shape - 0.05) / 0.22).clamp(0.0, 1.0); // soft, diffused edge

            // Bright, near-white top; greyer, cooler base — the sunlit-top /
            // shadowed-underside look real cumulus has — with the noise
            // value itself brightening the "puffy" high points a little more.
            let lift = (n - 0.5) * 18.0;
            let r = (232.0 - vshade * 60.0 + lift).clamp(0.0, 255.0) as u8;
            let g = (238.0 - vshade * 52.0 + lift).clamp(0.0, 255.0) as u8;
            let b = (248.0 - vshade * 34.0 + lift).clamp(0.0, 255.0) as u8;
            let a = (edge * 235.0) as u8;
            img.set_rgba(x, y, r, g, b, a);
        }
    }
    img.encode_png()
}

// Islands reuse cloud fBm for coast, sand and inland texture; three sizes are baked.
fn island_sprite(w: i32, h: i32, seed: u32) -> Vec<u8> {
    let mut img = Image::new(w as u32, h as u32);
    let cx = w as f32 * 0.5;
    let cy = h as f32 * 0.5;

    for y in 0..h {
        for x in 0..w {
            let nx = (x as f32 - cx) / (w as f32 * 0.48);
            let ny = (y as f32 - cy) / (h as f32 * 0.44);
            let env = (nx * nx + ny * ny).sqrt();
            let n = fbm(x as f32 * 0.14, y as f32 * 0.14, seed, 4);
            // fBm-perturbed coastline: a lumpy, irregular silhouette rather
            // than a clean ellipse — same idea as the cloud edge above.
            let shape = (1.0 - env) + (n - 0.5) * 0.6;
            if shape <= 0.0 {
                continue;
            }

            let (r, g, b) = if shape < 0.16 {
                (200.0 - n * 20.0, 186.0 - n * 20.0, 142.0 - n * 16.0) // sand shoreline
            } else {
                // Mottled green/rock interior — darker in the noise's
                // "valleys" so it doesn't read as one flat colour.
                let dark = n * 46.0;
                (60.0 - dark * 0.5, 98.0 - dark, 50.0 - dark * 0.5)
            };
            img.set(x, y, r.max(20.0) as u8, g.max(30.0) as u8, b.max(18.0) as u8);
        }
    }

    // Turret: a round grey emplacement with a stubby barrel, mounted at the
    // island's high point. Static art — no barrel rotation — since it fires
    // straight at the player procedurally at runtime instead.
    let (tx, ty) = (cx as i32, cy as i32 - 3);
    for yy in -5..5 {
        for xx in -5..5 {
            if (xx * xx) as f32 * 0.7 + (yy * yy) as f32 > 17.0 {
                continue;
            }
            let (px, py) = (tx + xx, ty + yy);
            if px < 0 || py < 0 || px >= w || py >= h {
                continue;
            }
            img.set(px, py, 96, 98, 102);
        }
    }
    for i in 0..5 {
        let py = ty - 4 - i;
        if py < 0 {
            break;
        }
        img.set(tx, py, 62, 64, 68);
    }
    if ty - 6 >= 0 {
        img.set(tx - 1, ty - 6, 40, 42, 46);
        img.set(tx + 1, ty - 6, 40, 42, 46);
    }

    img.encode_png()
}

// Offline-rasterized katakana glyphs; the runtime font supports only A–Z and digits.

/// Katakana glyphs needed for the seven boss names, 10 wide x 12 tall.
const KATAKANA_CHARS: [char; 37] = [
    'ス', 'カ', 'ウ', 'ト', 'ボ', 'マ', 'ー', 'イ', 'ン', 'タ', 'セ', 'プ', 'ガ', 'シ', 'ッ', 'ド', 'レ', 'ノ', 'バ',
    'ル', 'ク', 'ザ', 'ゥ', 'ム', 'キ', 'ャ', 'リ', 'ア', 'ペ', 'デ', 'ロ', 'ヤ', 'ュ', 'ヒ', 'テ', 'コ', 'フ',
];
const KATAKANA_GLYPHS: [[u16; 12]; 37] = [
    [0x000, 0x000, 0x000, 0x0FC, 0x018, 0x008, 0x018, 0x038, 0x06C, 0x0C4, 0x080, 0x000], // ス
    [0x000, 0x000, 0x020, 0x020, 0x0FC, 0x064, 0x024, 0x064, 0x044, 0x0DC, 0x098, 0x000], // カ
    [0x000, 0x000, 0x020, 0x030, 0x0FC, 0x084, 0x084, 0x00C, 0x018, 0x030, 0x020, 0x000], // ウ
    [0x000, 0x000, 0x000, 0x060, 0x060, 0x070, 0x07C, 0x06C, 0x060, 0x060, 0x020, 0x000], // ト
    [0x000, 0x000, 0x006, 0x034, 0x0FC, 0x030, 0x030, 0x0B4, 0x1B6, 0x030, 0x060, 0x000], // ボ
    [0x000, 0x000, 0x000, 0x0FC, 0x0FE, 0x00C, 0x048, 0x078, 0x030, 0x018, 0x008, 0x000], // マ
    [0x000, 0x000, 0x000, 0x000, 0x000, 0x000, 0x0FC, 0x000, 0x000, 0x000, 0x000, 0x000], // ー
    [0x000, 0x000, 0x000, 0x00C, 0x018, 0x030, 0x0F0, 0x090, 0x010, 0x010, 0x010, 0x000], // イ
    [0x000, 0x000, 0x000, 0x0C0, 0x060, 0x006, 0x004, 0x00C, 0x038, 0x0E0, 0x0C0, 0x000], // ン
    [0x000, 0x000, 0x020, 0x03C, 0x07C, 0x0CC, 0x0B8, 0x018, 0x03C, 0x060, 0x040, 0x000], // タ
    [0x000, 0x000, 0x000, 0x040, 0x07C, 0x1FC, 0x0CC, 0x048, 0x040, 0x07C, 0x03C, 0x000], // セ
    [0x000, 0x000, 0x006, 0x0FE, 0x0FC, 0x00C, 0x008, 0x018, 0x010, 0x070, 0x040, 0x000], // プ
    [0x000, 0x000, 0x026, 0x024, 0x0FC, 0x07C, 0x024, 0x064, 0x044, 0x0DC, 0x098, 0x000], // ガ
    [0x000, 0x000, 0x000, 0x060, 0x020, 0x084, 0x0C4, 0x00C, 0x038, 0x0F0, 0x0C0, 0x000], // シ
    [0x000, 0x000, 0x000, 0x000, 0x000, 0x0A4, 0x0D4, 0x00C, 0x008, 0x030, 0x060, 0x000], // ッ
    [0x000, 0x000, 0x000, 0x04C, 0x040, 0x060, 0x078, 0x04C, 0x040, 0x040, 0x040, 0x000], // ド
    [0x000, 0x000, 0x000, 0x0C0, 0x0C0, 0x0C0, 0x0C4, 0x0CC, 0x0D8, 0x0F0, 0x040, 0x000], // レ
    [0x000, 0x000, 0x000, 0x00C, 0x008, 0x008, 0x018, 0x030, 0x060, 0x0C0, 0x000, 0x000], // ノ
    [0x000, 0x000, 0x006, 0x00C, 0x048, 0x048, 0x04C, 0x0C4, 0x084, 0x186, 0x000, 0x000], // バ
    [0x000, 0x000, 0x000, 0x050, 0x050, 0x050, 0x050, 0x052, 0x0DC, 0x098, 0x010, 0x000], // ル
    [0x000, 0x000, 0x020, 0x030, 0x07C, 0x0CC, 0x08C, 0x018, 0x010, 0x070, 0x040, 0x000], // ク
    [0x000, 0x000, 0x000, 0x04C, 0x0FC, 0x1FC, 0x048, 0x048, 0x018, 0x010, 0x020, 0x000], // ザ
    [0x000, 0x000, 0x000, 0x000, 0x030, 0x0FC, 0x0CC, 0x08C, 0x008, 0x018, 0x030, 0x000], // ゥ
    [0x000, 0x000, 0x000, 0x020, 0x020, 0x060, 0x048, 0x04C, 0x0CC, 0x1FE, 0x000, 0x000], // ム
    [0x000, 0x000, 0x020, 0x020, 0x0FC, 0x0F0, 0x03C, 0x0FC, 0x0B0, 0x010, 0x010, 0x000], // キ
    [0x000, 0x000, 0x000, 0x000, 0x040, 0x07C, 0x0FC, 0x028, 0x020, 0x020, 0x030, 0x000], // ャ
    [0x000, 0x000, 0x000, 0x0CC, 0x0CC, 0x0CC, 0x0CC, 0x008, 0x008, 0x038, 0x020, 0x000], // リ
    [0x000, 0x000, 0x000, 0x0FE, 0x004, 0x03C, 0x038, 0x020, 0x020, 0x060, 0x040, 0x000], // ア
    [0x000, 0x000, 0x000, 0x004, 0x06C, 0x070, 0x0D8, 0x18C, 0x004, 0x006, 0x000, 0x000], // ペ
    [0x000, 0x000, 0x006, 0x0FC, 0x000, 0x0FC, 0x0FC, 0x030, 0x020, 0x060, 0x040, 0x000], // デ
    [0x000, 0x000, 0x000, 0x0FC, 0x0FC, 0x084, 0x084, 0x084, 0x084, 0x0FC, 0x084, 0x000], // ロ
    [0x000, 0x000, 0x040, 0x044, 0x07C, 0x1EC, 0x068, 0x020, 0x020, 0x030, 0x030, 0x000], // ヤ
    [0x000, 0x000, 0x000, 0x000, 0x000, 0x0F8, 0x018, 0x018, 0x018, 0x1FC, 0x000, 0x000], // ュ
    [0x000, 0x000, 0x080, 0x080, 0x08C, 0x0F0, 0x080, 0x080, 0x080, 0x0FE, 0x07E, 0x000], // ヒ
    [0x000, 0x000, 0x078, 0x000, 0x1FE, 0x030, 0x030, 0x030, 0x060, 0x0C0, 0x000, 0x000], // テ
    [0x000, 0x000, 0x000, 0x0FC, 0x00C, 0x00C, 0x00C, 0x00C, 0x0FC, 0x000, 0x000, 0x000], // コ
    [0x000, 0x000, 0x000, 0x1FC, 0x00C, 0x00C, 0x018, 0x030, 0x060, 0x0C0, 0x000, 0x000], // フ
];

/// The seven boss aircraft's Japanese names, in katakana, in the order of
/// BOSS_SPECS.
const BOSS_NAMES_JA: [&[char]; 7] = [
    &['ド', 'ン', 'リ', 'ュ', 'ウ'], // Ki-49 Donryu
    &['ヒ', 'リ', 'ュ', 'ウ'],       // Ki-67 Hiryu
    &['リ', 'ッ', 'コ', 'ウ'],       // G4M, the "Rikko"
    &['レ', 'ン', 'ザ', 'ン'],       // G8N Renzan
    &['タ', 'イ', 'テ', 'イ'],       // H8K flying boat, "Taitei"
    &['シ', 'ン', 'ザ', 'ン'],       // G5N Shinzan
    &['フ', 'ガ', 'ク'],             // Fugaku
];

const KATAKANA_GW: u32 = 10;
const KATAKANA_GH: u32 = 12;
const KATAKANA_GAP: u32 = 1;

/// Render a katakana string as a bitmap: glyphs left to right with a 1px
/// gap, each pixel blown up `scale`x so it reads clearly at game resolution
/// despite the tiny native glyph size.
fn katakana_image(text: &[char], scale: u32) -> Image {
    let n = text.len() as u32;
    let w = (n * (KATAKANA_GW + KATAKANA_GAP)).saturating_sub(KATAKANA_GAP) * scale;
    let h = KATAKANA_GH * scale;
    let mut img = Image::new(w.max(1), h.max(1));
    for (i, c) in text.iter().enumerate() {
        let Some(gi) = KATAKANA_CHARS.iter().position(|k| k == c) else {
            continue;
        };
        let glyph = &KATAKANA_GLYPHS[gi];
        let ox = i as u32 * (KATAKANA_GW + KATAKANA_GAP) * scale;
        for row in 0..KATAKANA_GH {
            let bits = glyph[row as usize];
            for col in 0..KATAKANA_GW {
                if (bits >> (KATAKANA_GW - 1 - col)) & 1 == 0 {
                    continue;
                }
                for sy in 0..scale {
                    for sx in 0..scale {
                        img.set((ox + col * scale + sx) as i32, (row * scale + sy) as i32, 255, 255, 255);
                    }
                }
            }
        }
    }
    img
}

fn boss_name_ja(tier: usize) -> Vec<u8> {
    katakana_image(BOSS_NAMES_JA[tier], 2).encode_png()
}

// The title's sea and seeded clouds stand still until the launch begins.
fn title_backdrop(clouds: &[Vec<u8>; 3]) -> Vec<u8> {
    let mut image = Image::new(480, 540);
    for y in 0..540 {
        let t = (((y as f32 + 0.5) * 14.0 / 540.0).floor() / 13.0).min(1.0);
        let color =
            (((0.10 - 0.07 * t) * 255.0) as u8, ((0.24 - 0.13 * t) * 255.0) as u8, ((0.46 - 0.16 * t) * 255.0) as u8);
        for x in 0..480 {
            image.set(x, y, color.0, color.1, color.2);
        }
    }
    for row in -1..20 {
        let y = row as f32 * 30.0;
        let depth = (y / 540.0).clamp(0.0, 1.0);
        let color = [
            ((0.55 + depth * 0.15) * 255.0) as u8,
            ((0.72 + depth * 0.12) * 255.0) as u8,
            235,
            ((0.22 + depth * 0.30) * 255.0) as u8,
        ];
        let wave = |x: f32| y + (x * 0.045 + y * 0.05).sin() * 3.2 + (x * 0.11 - y * 0.14).sin() * 1.1;
        for segment in 0..12 {
            let x = segment as f32 * 40.0;
            image.line((x, wave(x)), (x + 40.0, wave(x + 40.0)), 1.0, color);
        }
    }
    for i in 0..26 {
        let seed = i as f32 * 53.7;
        let (x, y) = ((seed * 13.0) % 480.0, (seed * 29.0) % 580.0 - 20.0);
        let distance = (y - x * 0.62 - 40.0).abs();
        let twinkle = seed.sin() * 0.5 + 0.5;
        let (radius, alpha) = if distance < 60.0 {
            (1.9 - distance / 60.0, (0.55 + twinkle * 0.35) * (1.0 - distance / 60.0))
        } else {
            (1.1, 0.20 + twinkle * 0.10)
        };
        if radius > 0.2 {
            image.circle(x, y, radius, [217, 240, 255, (alpha * 255.0) as u8]);
        }
    }
    for i in 0..8 {
        let (x, y) = ((i as f32 * 71.0) % 480.0, (i as f32 * 53.0) % 540.0);
        let radius = if i % 3 != 0 { 22.0 } else { 36.0 };
        image.circle(x + 9.0, y + 15.0, radius * 0.75, [0, 0, 0, 26]);
    }
    let decoded = clouds.each_ref().map(|png| Image::from_png(png));
    for i in 0..8 {
        let (x, y) = ((i as f32 * 71.0) % 480.0, (i as f32 * 53.0) % 540.0);
        let radius = if i % 3 != 0 { 22.0 } else { 36.0 };
        image.blit(&decoded[i % 3], x - radius * 1.8, y - radius * 0.85, radius * 3.6, radius * 1.7);
    }
    for y in 0..540 {
        for x in 0..480 {
            image.blend(x, y, [3, 6, 20, 97]);
        }
    }
    image.encode_png()
}

pub fn generate() -> Vec<Asset> {
    let clouds = [cloud_sprite(0x1DE7_C10D), cloud_sprite(0x2ACE_C10D), cloud_sprite(0x3FAD_C10D)];
    let title = title_backdrop(&clouds);
    let [cloud1, cloud2, cloud3] = clouds;
    vec![
        ("images/title_backdrop.png", title),
        ("images/player_plane.png", player_plane()),
        ("images/enemy_grunt.png", enemy_plane(0)),
        ("images/enemy_weaver.png", enemy_plane(1)),
        ("images/enemy_ace.png", enemy_plane(2)),
        ("images/boss_1.png", boss_plane(0)),
        ("images/boss_2.png", boss_plane(1)),
        ("images/boss_3.png", boss_plane(2)),
        ("images/boss_4.png", boss_plane(3)),
        ("images/boss_5.png", boss_plane(4)),
        ("images/boss_6.png", boss_plane(5)),
        ("images/boss_7.png", boss_plane(6)),
        ("images/boss_name_ja_1.png", boss_name_ja(0)),
        ("images/boss_name_ja_2.png", boss_name_ja(1)),
        ("images/boss_name_ja_3.png", boss_name_ja(2)),
        ("images/boss_name_ja_4.png", boss_name_ja(3)),
        ("images/boss_name_ja_5.png", boss_name_ja(4)),
        ("images/boss_name_ja_6.png", boss_name_ja(5)),
        ("images/boss_name_ja_7.png", boss_name_ja(6)),
        ("images/powerup.png", powerup_capsule()),
        ("images/health_pack.png", health_pack()),
        ("images/carrier.png", carrier_ship()),
        ("images/boat.png", boat()),
        ("images/cloud_1.png", cloud1),
        ("images/cloud_2.png", cloud2),
        ("images/cloud_3.png", cloud3),
        ("images/island_small.png", island_sprite(ISLAND_SIZES[0].0, ISLAND_SIZES[0].1, 0x9A17_1DE0)),
        ("images/island_medium.png", island_sprite(ISLAND_SIZES[1].0, ISLAND_SIZES[1].1, 0x9A17_2DE0)),
        ("images/island_large.png", island_sprite(ISLAND_SIZES[2].0, ISLAND_SIZES[2].1, 0x9A17_3DE0)),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The two ends of the sea/sky gradient, copied from draw_sea() in main.rs:
    /// if it changes there, change them here.
    const SKY_HORIZON: (u8, u8, u8) = (25, 61, 117);
    const SKY_NEAR: (u8, u8, u8) = (7, 28, 76);

    /// Relative luminance, as WCAG defines it.
    fn luminance((r, g, b): (u8, u8, u8)) -> f64 {
        fn channel(v: u8) -> f64 {
            let c = v as f64 / 255.0;
            if c <= 0.04045 {
                c / 12.92
            } else {
                ((c + 0.055) / 1.055).powf(2.4)
            }
        }
        0.2126 * channel(r) + 0.7152 * channel(g) + 0.0722 * channel(b)
    }

    fn contrast(a: (u8, u8, u8), b: (u8, u8, u8)) -> f64 {
        let (la, lb) = (luminance(a), luminance(b));
        let (hi, lo) = if la > lb { (la, lb) } else { (lb, la) };
        (hi + 0.05) / (lo + 0.05)
    }

    /// An enemy you cannot see is not a hard enemy: contact costs HP, so
    /// every kind's body and wings must clear 3:1 against the sky, the WCAG
    /// floor for a graphical object whose shape carries meaning.
    #[test]
    fn enemy_planes_are_visible_against_the_sky() {
        for kind in 0..3 {
            let (body, wing) = enemy_colors(kind);
            for (what, color) in [("body", body), ("wing", wing)] {
                for (where_, sky) in [("horizon", SKY_HORIZON), ("near", SKY_NEAR)] {
                    let c = contrast(color, sky);
                    assert!(
                        c >= 3.0,
                        "enemy kind {kind}'s {what} is {c:.2}:1 against the {where_} sky \
                         — below the 3:1 floor, so the plane reads as a smudge rather than \
                         a shape a player can weave between"
                    );
                }
            }
        }
    }

    /// The wings, the widest surface, are no darker than the fuselage.
    #[test]
    fn a_planes_wings_are_no_darker_than_its_fuselage() {
        for kind in 0..3 {
            let (body, wing) = enemy_colors(kind);
            assert!(
                luminance(wing) >= luminance(body),
                "enemy kind {kind}'s wings are darker than its body, which hides the widest \
                 part of the silhouette against the sky"
            );
        }
    }

    /// Each kind still has to be tellable from the others at a glance —
    /// the weaver dodges, the ace always drops a power-up, and a player
    /// reacts to which is which before reading anything else.
    #[test]
    fn the_three_enemy_kinds_stay_distinguishable_from_each_other() {
        let colors: Vec<_> = (0..3).map(|k| enemy_colors(k).0).collect();
        for a in 0..colors.len() {
            for b in (a + 1)..colors.len() {
                let (x, y) = (colors[a], colors[b]);
                let far_apart =
                    (x.0 as i32 - y.0 as i32).abs() + (x.1 as i32 - y.1 as i32).abs() + (x.2 as i32 - y.2 as i32).abs();
                assert!(
                    far_apart >= 120,
                    "enemy kinds {a} and {b} are within {far_apart} of each other — lifting them \
                     out of the sky must not flatten them into each other"
                );
            }
        }
    }
}
