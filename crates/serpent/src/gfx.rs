use blip::image::Image;
use blip::math::{hash, value_noise};
use blip::Asset;

const SPRITE: u32 = 24;

#[path = "field.rs"]
mod field;
use field::{CELL, CHEST, COLS, FIELD as SIZE, FOREST};
const BOARD: i32 = COLS * CELL;

/// A shaded apple: red body lit from the top left, a dimple, a brown stem and a green leaf.
fn food() -> Vec<u8> {
    let mut img = Image::new(SPRITE, SPRITE);
    let (cx, cy) = (11.5f32, 13.5f32);
    for y in 0..SPRITE as i32 {
        for x in 0..SPRITE as i32 {
            // Two overlapping lobes make the apple's shoulders.
            let (fx, fy) = (x as f32, y as f32);
            let lobe = |ox: f32| ((fx - (cx + ox)).powi(2) + ((fy - cy) * 1.08).powi(2)).sqrt();
            let d = lobe(-2.2).min(lobe(2.2));
            if d > 8.6 { continue; }
            if fy < 7.0 && (fx - cx).abs() < 1.5 { continue; } // the dimple
            let light = (((fx - 8.0).powi(2) + (fy - 9.0).powi(2)).sqrt() / 13.0).min(1.0);
            let k = 1.0 - light;
            let edge = d > 7.6;
            let (r, g, b) = if edge { (120.0, 20.0, 30.0) } else { (170.0 + 85.0 * k, 30.0 + 40.0 * k, 40.0 + 30.0 * k) };
            img.set(x, y, r as u8, g as u8, b as u8);
        }
    }
    for (x, y) in [(9, 9), (8, 10), (9, 10), (8, 11)] { img.set(x, y, 255, 220, 220); }
    for (x, y) in [(12, 2), (12, 3), (12, 4), (11, 5), (11, 6)] { img.set(x, y, 100, 60, 25); }
    for (x, y) in [(13, 3), (14, 3), (15, 2), (14, 4), (15, 3), (16, 2), (17, 2), (16, 3)] { img.set(x, y, 60, 180, 60); }
    img.outline(blip::look::INK);
    img.encode_png()
}

// ---- the field, painted once: Koholint's lawn and the wood round it --------------------------------------------

type Rgb = [f32; 3];

/// Smooth value noise, 0..1, varying over about `scale` pixels.
fn noise(x: f32, y: f32, scale: f32, seed: u32) -> f32 { value_noise(x / scale, y / scale, seed) }

fn mix(a: Rgb, b: Rgb, t: f32) -> Rgb {
    let t = t.clamp(0.0, 1.0);
    [a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t, a[2] + (b[2] - a[2]) * t]
}
fn scale(c: Rgb, k: f32) -> Rgb { [c[0] * k, c[1] * k, c[2] * k] }

/// A picture in floats, composited as it is painted.
struct Canvas { px: Vec<Rgb> }
impl Canvas {
    fn at(&self, x: i32, y: i32) -> Rgb { self.px[(y.clamp(0, SIZE - 1) * SIZE + x.clamp(0, SIZE - 1)) as usize] }
    fn put(&mut self, x: i32, y: i32, c: Rgb) {
        if (0..SIZE).contains(&x) && (0..SIZE).contains(&y) { self.px[(y * SIZE + x) as usize] = c; }
    }
    fn over(&mut self, x: i32, y: i32, c: Rgb, a: f32) {
        if (0..SIZE).contains(&x) && (0..SIZE).contains(&y) {
            let i = (y * SIZE + x) as usize;
            self.px[i] = mix(self.px[i], c, a);
        }
    }
}

/// A small repeatable random sequence for placing things.
struct Rand(u32);
impl Rand {
    fn next(&mut self) -> f32 {
        self.0 ^= self.0 << 13; self.0 ^= self.0 >> 17; self.0 ^= self.0 << 5;
        (self.0 & 0xffff) as f32 / 65535.0
    }
}

const LIGHT: [f32; 3] = [-0.48, -0.62, 0.62];

/// The lawn: sunny yellow-green grass in broad soft patches, fine blades in it, the board's cells a shade apart so
/// the eye can count them.
fn lawn(cv: &mut Canvas) {
    let deep: Rgb = [108.0, 178.0, 58.0];
    let sun: Rgb = [184.0, 230.0, 92.0];
    for y in 0..SIZE {
        for x in 0..SIZE {
            let (fx, fy) = (x as f32, y as f32);
            let patch = noise(fx, fy, 90.0, 1) * 0.6 + noise(fx, fy, 34.0, 2) * 0.4;
            let mut c = mix(deep, sun, 0.25 + patch * 0.75);
            // Blades: a grain running mostly up and down, and a speckle of lit and shaded tips.
            let grain = noise(fx * 2.2, fy * 0.7, 3.0, 3) - 0.5;
            let speck = hash(x, y, 4);
            c = scale(c, 1.0 + grain * 0.07 + if speck > 0.96 { 0.06 } else if speck < 0.03 { -0.06 } else { 0.0 });
            let (bx, by) = (x - FOREST, y - FOREST);
            if (0..BOARD).contains(&bx) && (0..BOARD).contains(&by) && ((bx / CELL + by / CELL) & 1) == 1 {
                c = scale(c, 0.962);
            }
            cv.put(x, y, c);
        }
    }
}

/// Little V-shaped blade marks all over the lawn, lit and shaded, the way Koholint's grass is drawn.
fn blade_marks(cv: &mut Canvas) {
    let mut rnd = Rand(0x6a55_0001);
    for _ in 0..900 {
        let (x, y) = ((FOREST as f32 + rnd.next() * BOARD as f32) as i32, (FOREST as f32 + rnd.next() * BOARD as f32) as i32);
        let lit = rnd.next() < 0.55;
        let (c, a): (Rgb, f32) = if lit { ([214.0, 246.0, 120.0], 0.55) } else { ([70.0, 136.0, 44.0], 0.45) };
        for (dx, dy) in [(-1, -1), (0, 0), (1, -1), (1, -2)] { cv.over(x + dx, y + dy, c, a); }
    }
}

/// Darken a soft ellipse: what stands on the ground shades it.
fn ground_shadow(cv: &mut Canvas, cx: f32, cy: f32, rx: f32, ry: f32, a: f32) {
    for y in (cy - ry - 2.0) as i32..=(cy + ry + 2.0) as i32 {
        for x in (cx - rx - 2.0) as i32..=(cx + rx + 2.0) as i32 {
            let d = (((x as f32 - cx) / rx).powi(2) + ((y as f32 - cy) / ry).powi(2)).sqrt();
            if d < 1.3 { cv.over(x, y, [20.0, 46.0, 24.0], a * (1.0 - (d - 0.6).max(0.0) / 0.7).clamp(0.0, 1.0)); }
        }
    }
}

/// Tall grass tufts, daisies and clover here and there on the lawn, in fixed places.
fn lawn_life(cv: &mut Canvas) {
    let mut rnd = Rand(0x5eed_1234);
    for _ in 0..64 {
        let (x, y) = (FOREST as f32 + 6.0 + rnd.next() * (BOARD - 12) as f32, FOREST as f32 + 10.0 + rnd.next() * (BOARD - 14) as f32);
        let kind = rnd.next();
        if kind < 0.5 {
            // A tuft of tall grass: seven to nine blades fanning from one root, dark at the root, lit at the tips.
            let blades = 7 + (rnd.next() * 3.0) as i32;
            ground_shadow(cv, x + 2.0, y + 1.0, 6.5, 2.4, 0.32);
            for b in 0..blades {
                let f = b as f32 / (blades - 1) as f32 - 0.5;
                let h = 7.0 + rnd.next() * 5.0 - f.abs() * 4.0;
                let (bx, tx, ty) = (x + f * 7.0, x + f * 12.0 + (rnd.next() - 0.5) * 2.0, y - h);
                for s in 0..=14 {
                    let t = s as f32 / 14.0;
                    let px = bx + (tx - bx) * t * t;
                    let py = y + (ty - y) * t;
                    let c = mix([34.0, 100.0, 36.0], [156.0, 218.0, 88.0], t * 1.05);
                    cv.put(px as i32, py as i32, c);
                    if t < 0.55 { cv.put(px as i32 + 1, py as i32, scale(c, 0.85)); }
                }
            }
        } else if kind < 0.82 {
            // A daisy: five round petals round a yellow eye, its shade under it; mostly white, some pink.
            let (petal, rim): (Rgb, Rgb) = if rnd.next() < 0.7 { ([252.0, 250.0, 238.0], [196.0, 196.0, 186.0]) } else { ([252.0, 176.0, 204.0], [206.0, 120.0, 150.0]) };
            ground_shadow(cv, x + 1.5, y + 2.0, 4.0, 2.4, 0.3);
            let turn = rnd.next();
            for k in 0..5 {
                let a = turn + k as f32 * 1.2566;
                let (px, py) = (x + a.cos() * 2.4, y + a.sin() * 2.4);
                for oy in -2..=2 { for ox in -2..=2 {
                    let d = ((ox * ox + oy * oy) as f32).sqrt();
                    if d <= 1.6 { cv.put((px + ox as f32) as i32, (py + oy as f32) as i32, if oy > 0 && d > 0.9 { rim } else { petal }); }
                } }
            }
            for (ox, oy, c) in [(0, 0, [250.0, 206.0, 60.0]), (1, 0, [236.0, 176.0, 40.0]), (0, 1, [236.0, 176.0, 40.0]), (1, 1, [200.0, 140.0, 30.0])] {
                cv.put(x as i32 + ox, y as i32 + oy, c);
            }
        } else {
            // A clover patch: darker trefoils.
            for _ in 0..5 {
                let (cx, cy) = (x + (rnd.next() - 0.5) * 10.0, y + (rnd.next() - 0.5) * 7.0);
                for (dx, dy) in [(0, -1), (-1, 0), (1, 0), (0, 0), (-1, -1), (1, -1)] { cv.over(cx as i32 + dx, cy as i32 + dy, [54.0, 124.0, 44.0], 0.7); }
            }
        }
    }
}

/// A tree seen from above as on Koholint: a dome of leaf clumps in tiers, a ring at the rim, a smaller ring above it
/// and a cap, each clump round and lit from the upper left, the whole dome darker toward its far side.
struct Tree { x: f32, y: f32, r: f32, lobes: Vec<(f32, f32, f32, f32)>, young: bool }

fn tree(x: f32, y: f32, r: f32, rnd: &mut Rand) -> Tree {
    let mut lobes = vec![];
    // (distance from the middle, clump radius, clumps, height): the rim, a middle ring, the cap
    for (d, lr, n, h) in [(0.62, 0.40, 9, 0.0), (0.30, 0.36, 6, 0.35), (0.0, 0.34, 1, 0.7)] {
        let turn = rnd.next() * 1.0;
        for k in 0..n {
            let a = turn + k as f32 / n as f32 * std::f32::consts::TAU;
            let dd = r * d * (0.94 + rnd.next() * 0.1);
            // the upper tiers sit a little toward the light, as a dome seen from just south of overhead
            lobes.push((a.cos() * dd - h * r * 0.08, a.sin() * dd - h * r * 0.14, r * lr * (0.92 + rnd.next() * 0.14), h));
        }
    }
    // One in five a younger, yellower green.
    Tree { x, y, r, lobes, young: rnd.next() < 0.2 }
}

impl Tree {
    /// The crown's height at a point (none outside it) and which clump it is on.
    fn height(&self, px: f32, py: f32) -> Option<(f32, usize)> {
        let mut best: Option<(f32, usize)> = None;
        for (i, &(lx, ly, lr, lift)) in self.lobes.iter().enumerate() {
            let (dx, dy) = (px - self.x - lx, py - self.y - ly);
            let d2 = dx * dx + dy * dy;
            if d2 < lr * lr {
                let h = (lr * lr - d2).sqrt() / lr * 0.6 + lift;
                if best.is_none_or(|(bh, _)| h > bh) { best = Some((h, i)); }
            }
        }
        best
    }

    fn paint(&self, cv: &mut Canvas, seed: u32) {
        let reach = self.r * 1.2;
        let (deep, mid, lit, shine): (Rgb, Rgb, Rgb, Rgb) = if self.young {
            ([34.0, 84.0, 36.0], [86.0, 158.0, 52.0], [158.0, 212.0, 78.0], [218.0, 240.0, 132.0])
        } else {
            ([16.0, 66.0, 44.0], [40.0, 132.0, 58.0], [104.0, 190.0, 70.0], [190.0, 232.0, 120.0])
        };
        for y in (self.y - reach) as i32..=(self.y + reach) as i32 {
            for x in (self.x - reach) as i32..=(self.x + reach) as i32 {
                let (px, py) = (x as f32 + 0.5, y as f32 + 0.5);
                let Some((h, i)) = self.height(px, py) else { continue };
                let edge = [(1, 0), (-1, 0), (0, 1), (0, -1)].iter().any(|&(dx, dy)| self.height(px + dx as f32, py + dy as f32).is_none());
                if edge { cv.put(x, y, [14.0, 34.0, 28.0]); continue; }
                // The clump's own surface and the dome's, half each, lit from the upper left.
                let (lx, ly, lr, _) = self.lobes[i];
                let (cx, cy) = ((px - self.x - lx) / lr, (py - self.y - ly) / lr);
                let (dx, dy) = ((px - self.x) / (self.r * 1.1), (py - self.y) / (self.r * 1.1));
                let (nx, ny) = (cx * 0.5 + dx * 0.6, cy * 0.5 + dy * 0.6);
                let nz = (1.0 - nx * nx - ny * ny).max(0.0).sqrt();
                let lam = (nx * LIGHT[0] + ny * LIGHT[1] + nz * LIGHT[2]).max(0.0);
                // a dark line where a clump tucks under a higher one
                let under = [(1.5, 0.0), (-1.5, 0.0), (0.0, 1.5), (0.0, -1.5)].iter()
                    .any(|&(ox, oy)| self.height(px + ox, py + oy).is_some_and(|(oh, oi)| oi != i && oh > h + 0.08));
                let leaf = noise(px, py, 3.5, seed) - 0.5;
                let k = (lam * 1.3 - 0.08 + leaf * 0.12 - if under { 0.4 } else { 0.0 }).clamp(0.0, 1.0);
                let mut c = if k < 0.42 { mix(deep, mid, k / 0.42) } else { mix(mid, lit, (k - 0.42) / 0.42) };
                if k > 0.9 { c = mix(c, shine, (k - 0.9) * 6.0); }
                cv.put(x, y, c);
            }
        }
    }

    /// The crown's shadow, thrown down and to the right.
    fn shade(&self, cv: &mut Canvas) {
        let (ox, oy) = (self.r * 0.28, self.r * 0.38);
        let reach = self.r * 1.4;
        for y in (self.y - reach) as i32..=(self.y + reach + oy) as i32 {
            for x in (self.x - reach) as i32..=(self.x + reach + ox) as i32 {
                let d = ((x as f32 - self.x - ox).powi(2) + (y as f32 - self.y - oy).powi(2)).sqrt() / (self.r * 1.02);
                if d < 1.25 { cv.over(x, y, [10.0, 34.0, 22.0], 0.6 * (1.0 - (d - 0.85).max(0.0) / 0.4).clamp(0.0, 1.0)); }
            }
        }
    }
}

fn on_board(x: i32, y: i32) -> bool { (FOREST..FOREST + BOARD).contains(&x) && (FOREST..FOREST + BOARD).contains(&y) }

/// The wood round the lawn: two rows of round trees in a ring, their floor in deep shade. The lawn is kept clear.
fn forest(cv: &mut Canvas) {
    for y in 0..SIZE {
        for x in 0..SIZE {
            if on_board(x, y) { continue; }
            let n = noise(x as f32, y as f32, 9.0, 7);
            cv.put(x, y, mix([24.0, 62.0, 32.0], [44.0, 96.0, 40.0], n));
        }
    }
    let mut rnd = Rand(0x00f0_7e57);
    let mut trees = vec![];
    // Two rows along each side: an outer one half out of the picture, an inner one at the lawn's edge.
    for (row, inset, gap) in [(0, 2.0f32, 44.0f32), (1, FOREST as f32 - 22.0, 46.0)] {
        let span = SIZE as f32;
        let mut t = 0.0;
        while t < span * 4.0 {
            let (side, s) = ((t / span) as i32, t % span);
            let jitter = (rnd.next() - 0.5) * 6.0;
            let (x, y) = match side {
                0 => (s, inset + jitter),
                1 => (span - inset + jitter, s),
                2 => (span - s, span - inset + jitter),
                _ => (inset + jitter, span - s),
            };
            let rad = if row == 0 { 28.0 + rnd.next() * 5.0 } else { 22.0 + rnd.next() * 3.0 };
            trees.push(tree(x, y, rad, &mut rnd));
            t += gap + (rnd.next() - 0.5) * 8.0;
        }
    }
    // A glade round the chest: no tree stands on it.
    let (gx, gy) = (CHEST.0 as f32 + 10.0, CHEST.1 as f32 + 8.0);
    trees.retain(|t| (t.x - gx).hypot(t.y - gy) > t.r + 8.0);
    // Back to front: the higher on the picture, the farther.
    trees.sort_by(|a, b| a.y.partial_cmp(&b.y).unwrap());
    let lawn: Vec<Rgb> = cv.px.clone();
    for t in &trees { t.shade(cv); }
    for (i, t) in trees.iter().enumerate() { t.paint(cv, 11 + i as u32); }
    for y in FOREST..FOREST + BOARD { for x in FOREST..FOREST + BOARD { cv.put(x, y, lawn[(y * SIZE + x) as usize]); } }
}

/// A low flowering bush at each of the lawn's corners, in the wood's edge: round leaves, pink and white flowers.
fn corner_bushes(cv: &mut Canvas) {
    let mut rnd = Rand(0xb05_4e5);
    let (a, b) = (FOREST as f32, (FOREST + BOARD) as f32);
    for (cx, cy) in [(a - 10.0, a - 8.0), (b + 10.0, a - 8.0), (a - 10.0, b + 10.0), (b + 10.0, b + 10.0)] {
        let mut bush = tree(cx, cy, 19.0, &mut rnd);
        bush.young = false;
        bush.shade(cv);
        bush.paint(cv, 99);
        for _ in 0..14 {
            let (fx, fy) = (cx + (rnd.next() - 0.5) * 28.0, cy + (rnd.next() - 0.6) * 24.0);
            if bush.height(fx, fy).is_none() { continue; }
            let (c, rim): (Rgb, Rgb) = if rnd.next() < 0.6 { ([252.0, 150.0, 192.0], [190.0, 80.0, 120.0]) } else { ([255.0, 250.0, 240.0], [180.0, 180.0, 170.0]) };
            for (dx, dy) in [(0, -1), (-1, 0), (1, 0), (0, 1), (-1, -1), (1, -1), (-1, 1), (1, 1)] {
                cv.put(fx as i32 + dx, fy as i32 + dy, if dy > 0 { rim } else { c });
            }
            cv.put(fx as i32, fy as i32, [250.0, 210.0, 70.0]);
        }
    }
}

/// A treasure chest left under the trees at the bottom right, as in any wood worth the name: dark wood bands, a gold
/// lock and corners, its lid lit from above.
fn chest(cv: &mut Canvas, x0: i32, y0: i32) {
    let (w, h) = (20, 15);
    ground_shadow(cv, x0 as f32 + 12.0, y0 as f32 + h as f32 + 1.0, 13.0, 3.5, 0.55);
    for y in 0..h {
        for x in 0..w {
            let edge = x == 0 || y == 0 || x == w - 1 || y == h - 1 || y == 6;
            let band = x == 5 || x == 14;
            let lid = y < 6;
            let c: Rgb = if edge { [40.0, 22.0, 18.0] }
                else if band { [214.0, 160.0, 52.0] }
                else if lid { mix([186.0, 108.0, 48.0], [132.0, 70.0, 32.0], y as f32 / 6.0) }
                else { mix([150.0, 82.0, 38.0], [104.0, 54.0, 26.0], (y - 6) as f32 / 8.0) };
            cv.put(x0 + x, y0 + y, c);
        }
    }
    for (dx, dy, c) in [(9, 5, [250.0, 214.0, 90.0]), (10, 5, [250.0, 214.0, 90.0]), (9, 6, [214.0, 160.0, 52.0]), (10, 6, [214.0, 160.0, 52.0]),
                        (9, 7, [214.0, 160.0, 52.0]), (10, 7, [120.0, 80.0, 20.0]), (9, 8, [40.0, 22.0, 18.0])] {
        cv.put(x0 + dx, y0 + dy, c);
    }
    for x in 2..w - 2 { cv.over(x0 + x, y0 + 1, [255.0, 230.0, 180.0], 0.35); }
}

/// The lawn's edge, the snake's wall: the wood's shade on the grass, deepest along the top and left where the light
/// comes over the trees, and an ink line where the lawn stops.
fn edge(cv: &mut Canvas) {
    let (a, b) = (FOREST, FOREST + BOARD);
    for y in a..b {
        for x in a..b {
            let (left, top, right, bottom) = ((x - a) as f32, (y - a) as f32, (b - 1 - x) as f32, (b - 1 - y) as f32);
            let shade = |d: f32, reach: f32| if d < reach { (1.0 - d / reach).powf(1.6) } else { 0.0 };
            let k = shade(left, 16.0).max(shade(top, 18.0)).max(shade(right, 7.0)).max(shade(bottom, 7.0));
            if k > 0.0 { cv.over(x, y, [18.0, 52.0, 26.0], 0.45 * k); }
        }
    }
    for i in a - 1..=b {
        for (x, y) in [(i, a - 1), (i, b), (a - 1, i), (b, i)] { cv.put(x, y, [24.0, 26.0, 36.0]); }
    }
}

/// Like a model photographed close, the picture's top and bottom edges are out of focus.
fn tilt_shift(cv: &mut Canvas) {
    let src = cv.px.clone();
    for y in 0..SIZE {
        let off = if y < FOREST { FOREST - y } else if y >= FOREST + BOARD { y - FOREST - BOARD + 1 } else { 0 };
        let r = (off as f32 / FOREST as f32 * 2.6).round() as i32;
        if r == 0 { continue; }
        for x in 0..SIZE {
            let mut acc = [0.0; 3];
            let mut n = 0.0;
            for dy in -r..=r {
                for dx in -r..=r {
                    let c = src[((y + dy).clamp(0, SIZE - 1) * SIZE + (x + dx).clamp(0, SIZE - 1)) as usize];
                    acc = [acc[0] + c[0], acc[1] + c[1], acc[2] + c[2]];
                    n += 1.0;
                }
            }
            cv.put(x, y, scale(acc, 1.0 / n));
        }
    }
}

/// The whole field: lawn, life on it, the wood, its edge, and the picture's focus; and the wood alone, to draw over
/// what goes in under the trees.
fn field() -> (Vec<u8>, Vec<u8>) {
    let mut cv = Canvas { px: vec![[0.0; 3]; (SIZE * SIZE) as usize] };
    lawn(&mut cv);
    blade_marks(&mut cv);
    lawn_life(&mut cv);
    forest(&mut cv);
    corner_bushes(&mut cv);
    edge(&mut cv);
    tilt_shift(&mut cv);
    // the chest stays sharp: the eye finds it
    chest(&mut cv, CHEST.0, CHEST.1);
    let (mut img, mut wood) = (Image::new(SIZE as u32, SIZE as u32), Image::new(SIZE as u32, SIZE as u32));
    for y in 0..SIZE {
        for x in 0..SIZE {
            let c = cv.at(x, y);
            let (r, g, b) = (c[0].clamp(0.0, 255.0) as u8, c[1].clamp(0.0, 255.0) as u8, c[2].clamp(0.0, 255.0) as u8);
            img.set(x, y, r, g, b);
            if !on_board(x, y) { wood.set(x, y, r, g, b); }
        }
    }
    (img.encode_png(), wood.encode_png())
}

pub fn generate() -> Vec<Asset> {
    let (field, wood) = field();
    vec![("images/food.png", food()), ("images/field.png", field), ("images/wood.png", wood)]
}
