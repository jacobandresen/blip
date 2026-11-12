//! The title painting, after the classic flyer's layout: a storm-lit sky
//! and mountains, a brick maze in true perspective (ray-marched), and on
//! the right the Warlock leaning out of a stone arch, reaching for the
//! young wizard below. Everything here is still; lightning, fog, eyes and
//! the staff's light are added by the game at runtime.

use super::*;

#[path = "runes.rs"]
mod runes;

const W: i32 = 480;
const H: i32 = 540;
const HORIZON: f32 = 232.0;

// ---- noise -----------------------------------------------------------------------

fn lattice(x: i32, y: i32, seed: u32) -> f32 {
    hash(x, y, seed)
}

/// Smooth value noise in `[0, 1]`.
fn noise(x: f32, y: f32, seed: u32) -> f32 {
    let (xi, yi) = (x.floor() as i32, y.floor() as i32);
    let (xf, yf) = (x - x.floor(), y - y.floor());
    let s = |t: f32| t * t * (3.0 - 2.0 * t);
    let (u, v) = (s(xf), s(yf));
    let a = lattice(xi, yi, seed);
    let b = lattice(xi + 1, yi, seed);
    let c = lattice(xi, yi + 1, seed);
    let d = lattice(xi + 1, yi + 1, seed);
    a + (b - a) * u + (c - a) * v + (a - b - c + d) * u * v
}

/// Fractal noise: `octaves` of value noise, each half as strong.
fn fbm(x: f32, y: f32, octaves: u32, seed: u32) -> f32 {
    let (mut sum, mut amp, mut f, mut norm) = (0.0, 1.0, 1.0, 0.0);
    for o in 0..octaves {
        sum += noise(x * f, y * f, seed + o * 17) * amp;
        norm += amp;
        amp *= 0.5;
        f *= 2.03;
    }
    sum / norm
}

type V3 = [f32; 3];

fn mul(a: V3, k: f32) -> V3 {
    [a[0] * k, a[1] * k, a[2] * k]
}
fn addv(a: V3, b: V3) -> V3 {
    [a[0] + b[0], a[1] + b[1], a[2] + b[2]]
}
fn tint(a: V3, b: V3) -> V3 {
    [a[0] * b[0], a[1] * b[1], a[2] * b[2]]
}
fn lerp3(a: V3, b: V3, t: f32) -> V3 {
    [a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t, a[2] + (b[2] - a[2]) * t]
}
fn norm(a: V3) -> V3 {
    let l = (a[0] * a[0] + a[1] * a[1] + a[2] * a[2]).sqrt().max(1e-6);
    [a[0] / l, a[1] / l, a[2] / l]
}
fn dot(a: V3, b: V3) -> f32 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

// ---- the world -------------------------------------------------------------------

const MOON_LIGHT: V3 = [0.36, 0.42, 0.62];
const TORCH_COL: V3 = [1.0, 0.56, 0.26];
const FOG: V3 = [0.10, 0.12, 0.19];
const MAZE_W: i32 = 14;
const MAZE_D: i32 = 46;
const CAM: V3 = [6.5, 1.85, 0.6];
const TORCH: V3 = [9.4, 1.5, 3.4];

/// A maze of brick blocks: long walls with gaps across the view, and some
/// running away from it, like a cabinet's playfield seen from above.
fn wall_at(i: i32, j: i32) -> bool {
    if !(0..MAZE_W).contains(&i) {
        return true;
    }
    if !(0..MAZE_D).contains(&j) {
        return j >= MAZE_D;
    }
    // a corridor runs away from the viewer between two long walls with
    // gaps, crossed by walls further off; past that, a random warren
    let near = matches!(
        (i, j),
        (4, 2..=7) | (4, 10..=15)
            | (9, 3..=6) | (9, 9..=14)
            | (0..=2, 9) | (1..=2, 4..=5)
            | (6..=7, 12) | (11..=13, 11)
            | (2..=6, 18) | (8..=12, 18)
            | (6, 19..=22) | (10, 20..=23)
            | (1..=4, 24) | (7..=11, 25)
    );
    if j < 26 {
        return near || i == 0 || i == MAZE_W - 1;
    }
    let across = j % 3 == 2 && hash(i / 2, j, 77) > 0.32;
    let along = i % 3 == 1 && j % 3 != 2 && hash(i, j / 3, 78) > 0.5;
    across || along || i == 0 || i == MAZE_W - 1
}

enum Hit {
    Side { p: V3, n: V3, t: f32 },
    Top { p: V3, t: f32 },
    Floor { p: V3, t: f32 },
    Sky,
}

fn march(o: V3, d: V3) -> Hit {
    let (mut i, mut j) = (o[0].floor() as i32, o[2].floor() as i32);
    let step_i = if d[0] > 0.0 { 1 } else { -1 };
    let step_j = if d[2] > 0.0 { 1 } else { -1 };
    let tdx = if d[0].abs() < 1e-6 { f32::MAX } else { (1.0 / d[0]).abs() };
    let tdz = if d[2].abs() < 1e-6 { f32::MAX } else { (1.0 / d[2]).abs() };
    let mut tx = if d[0] > 0.0 { (i as f32 + 1.0 - o[0]) * tdx } else { (o[0] - i as f32) * tdx };
    let mut tz = if d[2] > 0.0 { (j as f32 + 1.0 - o[2]) * tdz } else { (o[2] - j as f32) * tdz };
    let mut t0 = 0.0f32;
    let mut normal: V3 = [0.0, 0.0, -1.0];
    for _ in 0..160 {
        let t1 = tx.min(tz);
        let y0 = o[1] + d[1] * t0;
        let y1 = o[1] + d[1] * t1;
        if wall_at(i, j) {
            if y0 <= 1.0 && t0 > 0.0 {
                return Hit::Side { p: addv(o, mul(d, t0)), n: normal, t: t0 };
            }
            if y1 <= 1.0 && d[1] < 0.0 {
                let t = (1.0 - o[1]) / d[1];
                return Hit::Top { p: addv(o, mul(d, t)), t };
            }
        } else if d[1] < 0.0 && y1 <= 0.0 {
            let t = -o[1] / d[1];
            return Hit::Floor { p: addv(o, mul(d, t)), t };
        }
        if t1 > 70.0 {
            break;
        }
        t0 = t1;
        if tx < tz {
            tx += tdx;
            i += step_i;
            normal = [-step_i as f32, 0.0, 0.0];
        } else {
            tz += tdz;
            j += step_j;
            normal = [0.0, 0.0, -step_j as f32];
        }
    }
    Hit::Sky
}

fn light(p: V3, n: V3, base: V3) -> V3 {
    let moon = norm([-0.55, 0.75, 0.45]);
    let diff = dot(n, moon).max(0.0);
    let to_t = [TORCH[0] - p[0], TORCH[1] - p[1], TORCH[2] - p[2]];
    let dist2 = dot(to_t, to_t);
    let lt = dot(n, norm(to_t)).max(0.0) * 7.0 / (1.0 + dist2 * 0.55);
    let ambient = [0.10, 0.11, 0.16];
    let l = addv(addv(ambient, mul(MOON_LIGHT, diff * 0.75)), mul(TORCH_COL, lt));
    tint(base, l)
}

/// Bricks on a wall face: `u` along the face, `v` up it.
fn brick(u: f32, v: f32, p: V3) -> V3 {
    let rows = 7.0;
    let row = (v * rows).floor();
    let off = if row as i32 % 2 == 0 { 0.0 } else { 0.25 };
    let bu = (u * 2.0 + off).floor();
    let fu = (u * 2.0 + off).fract();
    let fv = (v * rows).fract();
    let edge = fu.min(1.0 - fu).min(fv.min(1.0 - fv) * 0.5);
    let grain = fbm(p[0] * 9.0 + p[2] * 9.0, p[1] * 11.0, 3, 5);
    if edge < 0.035 {
        return mul([0.10, 0.06, 0.05], 0.8 + grain * 0.4);
    }
    let h = hash(bu as i32, row as i32, 91);
    let base = lerp3([0.44, 0.22, 0.15], [0.52, 0.31, 0.22], h);
    let bevel = (edge / 0.09).min(1.0);
    let k = (0.72 + 0.28 * bevel) * (0.8 + grain * 0.45);
    // soot near the floor
    let soot = 0.6 + 0.4 * (v / 0.35).min(1.0);
    mul(base, k * soot)
}

fn cap(p: V3) -> V3 {
    let g = fbm(p[0] * 6.0, p[2] * 6.0, 4, 8);
    let seam = ((p[0] * 2.0).fract().min((p[2] * 2.0).fract())) < 0.04;
    let base = [0.42, 0.36, 0.32];
    if seam { mul(base, 0.4) } else { mul(base, 0.75 + g * 0.5) }
}

fn floor_col(p: V3) -> V3 {
    let g = fbm(p[0] * 2.5, p[2] * 2.5, 4, 11);
    let su = (p[0] * 1.4 + (p[2] * 0.7).floor() * 0.5).fract();
    let sv = (p[2] * 0.7).fract();
    let seam = su < 0.03 || sv < 0.03;
    let base = [0.20, 0.18, 0.17];
    if seam { mul(base, 0.35) } else { mul(base, 0.7 + g * 0.6) }
}

fn sky(px: f32, py: f32) -> V3 {
    let k = (py / HORIZON).clamp(0.0, 1.0);
    let mut c = lerp3([0.02, 0.025, 0.05], [0.13, 0.15, 0.24], k.powf(1.6));
    // the moon, haloed
    let (mx, my) = (104.0, 70.0);
    let d = ((px - mx).powi(2) + (py - my).powi(2)).sqrt();
    c = addv(c, mul([0.55, 0.6, 0.75], (1.0 - d / 140.0).max(0.0).powi(3) * 0.6));
    // storm clouds, lit from the moon's side
    let cl = fbm(px / 90.0, py / 34.0, 5, 21);
    let dens = ((cl - 0.48) * 3.2).clamp(0.0, 1.0);
    let lit = (1.0 - d / 260.0).max(0.0);
    let cloud = lerp3([0.05, 0.055, 0.08], [0.42, 0.45, 0.55], lit * (0.4 + 0.6 * fbm(px / 40.0 + 3.0, py / 18.0, 3, 22)));
    c = lerp3(c, cloud, dens * 0.85);
    if d < 22.0 && dens < 0.6 {
        let crater = fbm(px / 6.0, py / 6.0, 3, 23);
        let limb = 1.0 - (d / 22.0).powi(4) * 0.3;
        c = lerp3(c, mul([0.92, 0.9, 0.84], limb * (0.78 + crater * 0.3)), 1.0 - dens / 0.6);
    }
    // the mountains, two ridges in the haze
    for (layer, base, col) in [(0.0f32, 182.0f32, [0.12, 0.13, 0.2]), (1.0, 206.0, [0.07, 0.075, 0.11])] {
        let ridge = base - (fbm(px / 70.0 + layer * 7.0, layer, 3, 31) - 0.3) * 120.0 - (fbm(px / 14.0, layer, 2, 33) - 0.5) * 6.0;
        if py > ridge {
            // moonlit faces on the slopes that lean left
            let slope = fbm((px - 3.0) / 70.0 + layer * 7.0, layer, 3, 31) - fbm((px + 3.0) / 70.0 + layer * 7.0, layer, 3, 31);
            let rim = if py - ridge < 2.5 { 0.07 } else { 0.0 };
            let fade = (1.0 - (py - ridge) / 40.0).max(0.0);
            let lit = (slope * 6.0).clamp(0.0, 1.0) * fade * 0.12;
            c = addv(col, [rim + lit, rim + lit * 1.1, rim + lit * 1.4]);
        }
    }
    c
}

fn render_maze(img: &mut [V3]) {
    let f = 330.0f32;
    let (pitch, yaw) = (0.30f32, -0.20f32);
    let (u0, v0) = (176.0f32, HORIZON + f * pitch.tan());
    for py in 0..H {
        for px in 0..W {
            // two samples per pixel, side by side, for smoother edges
            let mut acc = [0.0; 3];
            for s in 0..2 {
                let sx = px as f32 + 0.25 + 0.5 * s as f32;
                let sy = py as f32 + 0.5;
                let dc = [(sx - u0) / f, -(sy - v0) / f, 1.0];
                let y1 = dc[1] * pitch.cos() - dc[2] * pitch.sin();
                let z1 = dc[1] * pitch.sin() + dc[2] * pitch.cos();
                let x2 = dc[0] * yaw.cos() + z1 * yaw.sin();
                let z2 = -dc[0] * yaw.sin() + z1 * yaw.cos();
                let d = norm([x2, y1, z2]);
                let (col, t) = match march(CAM, d) {
                    Hit::Side { p, n, t } => {
                        let u = if n[0] != 0.0 { p[2] } else { p[0] };
                        (light(p, n, brick(u, p[1], p)), t)
                    }
                    Hit::Top { p, t } => (light(p, [0.0, 1.0, 0.0], cap(p)), t),
                    Hit::Floor { p, t } => {
                        // a little contact shadow beside the walls
                        let near = [(-0.18, 0.0), (0.18, 0.0), (0.0, -0.18), (0.0, 0.18)]
                            .iter()
                            .filter(|&&(dx, dz)| wall_at((p[0] + dx).floor() as i32, (p[2] + dz).floor() as i32))
                            .count();
                        let ao = 1.0 - near as f32 * 0.22;
                        (mul(light(p, [0.0, 1.0, 0.0], floor_col(p)), ao), t)
                    }
                    Hit::Sky => (sky(sx, sy), -1.0),
                };
                // the torch on the arch throws a warm pool over the nearer maze
                let col = if t > 0.0 {
                    let d = ((sx - 308.0).powi(2) + ((sy - 250.0) * 1.3).powi(2)).sqrt();
                    let w = (1.0 - d / 230.0).max(0.0).powi(2) * 0.7;
                    addv(col, tint(col, mul(TORCH_COL, w * 2.2)))
                } else {
                    col
                };
                let col = if t > 0.0 {
                    // aerial perspective, and a low mist over the floor
                    let fog = 1.0 - (-t * 0.055).exp();
                    lerp3(col, FOG, fog.min(0.92))
                } else {
                    col
                };
                acc = addv(acc, mul(col, 0.5));
            }
            img[(py * W + px) as usize] = acc;
        }
    }
}

// ---- the arch, the Warlock, the young wizard -------------------------------------------

/// Paint `col` (a closure of position) wherever the polygon covers.
fn paint_poly(img: &mut [V3], pts: &[(f32, f32)], f: &dyn Fn(f32, f32) -> Option<V3>) {
    let (mut x0, mut y0, mut x1, mut y1) = (f32::MAX, f32::MAX, f32::MIN, f32::MIN);
    for &(x, y) in pts {
        x0 = x0.min(x);
        y0 = y0.min(y);
        x1 = x1.max(x);
        y1 = y1.max(y);
    }
    for y in (y0.floor() as i32).max(0)..=(y1.ceil() as i32).min(H - 1) {
        for x in (x0.floor() as i32).max(0)..=(x1.ceil() as i32).min(W - 1) {
            let (px, py) = (x as f32 + 0.5, y as f32 + 0.5);
            let mut inside = false;
            let mut j = pts.len() - 1;
            for i in 0..pts.len() {
                let (xi, yi) = pts[i];
                let (xj, yj) = pts[j];
                if (yi > py) != (yj > py) && px < (xj - xi) * (py - yi) / (yj - yi) + xi {
                    inside = !inside;
                }
                j = i;
            }
            if inside {
                if let Some(c) = f(px, py) {
                    img[(y * W + x) as usize] = c;
                }
            }
        }
    }
}

fn paint_disc(img: &mut [V3], cx: f32, cy: f32, rx: f32, ry: f32, f: &dyn Fn(f32, f32, f32) -> V3) {
    for y in ((cy - ry) as i32).max(0)..=((cy + ry) as i32 + 1).min(H - 1) {
        for x in ((cx - rx) as i32).max(0)..=((cx + rx) as i32 + 1).min(W - 1) {
            let dx = (x as f32 + 0.5 - cx) / rx;
            let dy = (y as f32 + 0.5 - cy) / ry;
            let d = dx * dx + dy * dy;
            if d <= 1.0 {
                img[(y * W + x) as usize] = f(x as f32 + 0.5, y as f32 + 0.5, d);
            }
        }
    }
}

fn paint_line(img: &mut [V3], a: (f32, f32), b: (f32, f32), w: f32, f: &dyn Fn(f32, f32, f32) -> V3) {
    let r = w / 2.0;
    let (dx, dy) = (b.0 - a.0, b.1 - a.1);
    let len2 = (dx * dx + dy * dy).max(1e-4);
    for y in ((a.1.min(b.1) - r) as i32).max(0)..=((a.1.max(b.1) + r) as i32 + 1).min(H - 1) {
        for x in ((a.0.min(b.0) - r) as i32).max(0)..=((a.0.max(b.0) + r) as i32 + 1).min(W - 1) {
            let (px, py) = (x as f32 + 0.5, y as f32 + 0.5);
            let t = (((px - a.0) * dx + (py - a.1) * dy) / len2).clamp(0.0, 1.0);
            let (qx, qy) = (a.0 + dx * t - px, a.1 + dy * t - py);
            let d = (qx * qx + qy * qy).sqrt();
            if d <= r {
                img[(y * W + x) as usize] = f(px, py, d / r);
            }
        }
    }
}

const ARCH_X: f32 = 396.0;
const ARCH_Y: f32 = 176.0;
const ARCH_R: f32 = 74.0;

fn arch(img: &mut [V3]) {
    for y in 30..H {
        for x in 290..W {
            let (px, py) = (x as f32 + 0.5, y as f32 + 0.5);
            let (dx, dy) = (px - ARCH_X, py - ARCH_Y);
            let r = if py < ARCH_Y { (dx * dx + dy * dy).sqrt() } else { dx.abs() };
            if r < ARCH_R {
                // the doorway: black, with a faint violet breath from within
                let k = (1.0 - r / ARCH_R).max(0.0);
                img[(y * W + x) as usize] = [0.012 + 0.03 * k, 0.008 + 0.01 * k, 0.02 + 0.045 * k];
            } else if r < ARCH_R + 30.0 {
                let depth = r - ARCH_R;
                let (u, v, along, across) = if py < ARCH_Y {
                    let a = dy.atan2(dx);
                    let a = a + (fbm(px / 9.0, py / 9.0, 2, 42) - 0.5) * 0.06;
                    ((a * 4.6).floor() as i32, 0, (a * 4.6).fract(), depth / 30.0)
                } else {
                    let yy = py + (fbm(px / 9.0, py / 9.0, 2, 43) - 0.5) * 5.0;
                    let row = ((yy - ARCH_Y) / 26.0).floor() as i32;
                    let off = if row % 2 == 0 { 0.0 } else { 0.5 };
                    (row + 50, (depth / 30.0 + off) as i32, ((yy - ARCH_Y) / 26.0).fract(), (depth / 30.0 + off).fract())
                };
                let along = along.abs();
                let e = along.min(1.0 - along).min(across.min(1.0 - across));
                let g = fbm(px / 5.0, py / 5.0, 4, 40);
                let h = hash(u, v, 41);
                let base = lerp3([0.34, 0.31, 0.29], [0.46, 0.42, 0.38], h);
                // stones bevel at their edges; the left face catches the moon
                let k = if e < 0.035 { 0.2 } else { (0.6 + 0.4 * (e / 0.16).min(1.0)) * (0.65 + 0.55 * g) };
                let side = if px < ARCH_X { 1.0 } else { 0.55 };
                let warm = (1.0 - ((px - 300.0).powi(2) + (py - 250.0).powi(2)).sqrt() / 140.0).max(0.0);
                let c = addv(tint(mul(base, k * side), [0.55, 0.6, 0.75]), mul(TORCH_COL, warm * k * 0.55));
                img[(y * W + x) as usize] = c;
            }
        }
    }
}

/// The inscription cut into the arch's stones: a lit lower lip, then the
/// dark groove itself.
fn inscription(img: &mut [V3]) {
    let strokes = runes::strokes();
    for &(_, a, b) in &strokes {
        paint_line(img, (a.0 + 0.7, a.1 + 0.8), (b.0 + 0.7, b.1 + 0.8), 1.8, &|_, _, d| mul([0.50, 0.48, 0.52], 1.0 - d * 0.4));
    }
    for &(_, a, b) in &strokes {
        paint_line(img, a, b, 1.7, &|_, _, d| mul([0.05, 0.04, 0.06], 0.8 + d * 0.5));
    }
}

/// Cloth: soft vertical folds that sway, lit by the moon from the upper
/// left and warmed on the left by the torch.
fn cloth(px: f32, py: f32, base: V3, sway: f32) -> V3 {
    let fold = |x: f32, y: f32| {
        let bend = (y * 0.021).sin() * 10.0 + sway * (y - 200.0) * 0.08;
        (x + bend) * 0.11 + fbm(x / 30.0, y / 60.0, 3, 51) * 2.0
    };
    let h = |x: f32, y: f32| fold(x, y).sin() * 1.0 + (fold(x, y) * 2.3 + 1.0).sin() * 0.3;
    let dx = h(px + 1.0, py) - h(px - 1.0, py);
    let dy = h(px, py + 1.0) - h(px, py - 1.0);
    let n = norm([-dx * 1.6, -dy * 1.6, 1.0]);
    let moon = norm([-0.7, -0.5, 0.6]);
    let diff = dot(n, moon).max(0.0);
    let warm = (1.0 - ((px - 300.0).powi(2) + (py - 250.0).powi(2)).sqrt() / 170.0).max(0.0);
    let dark = 1.0 - ((px - 330.0) / 160.0).clamp(0.0, 1.0) * 0.55;
    let ao = 0.75 + 0.25 * (h(px, py) * 0.5 + 0.5);
    let k = (0.12 + diff.powf(1.4) * 1.15) * dark * ao;
    addv(tint(mul(base, k), [0.72, 0.78, 0.95]), mul(TORCH_COL, warm * 0.55 * diff))
}

/// A set of hanging folds: pipe folds that fan out from a point the cloth
/// hangs from (a shoulder, a belt, a clasp), deepening with distance from it.
/// `up` folds rise from the focus instead (cloth bloused above a belt);
/// `wind` bends them sideways the farther they fall.
#[derive(Clone, Copy)]
struct Folds {
    focus: (f32, f32),
    up: bool,
    count: f32,
    depth: f32,
    wind: f32,
    seed: u32,
}

impl Folds {
    fn height(&self, x: f32, y: f32) -> f32 {
        let dy = if self.up { self.focus.1 - y } else { y - self.focus.1 };
        if dy <= 0.0 {
            return 0.0;
        }
        let xb = x - self.focus.0 - self.wind * dy * dy / 100.0;
        let ang = xb.atan2(dy + 10.0);
        let phase = ang * self.count + fbm(x / 34.0, y / 70.0, 3, self.seed) * 2.2;
        // pipe folds: round on the ridge, pinched in the valley
        let amp = self.depth * (dy / 110.0).min(1.6) * (0.75 + 0.5 * fbm(x / 50.0, y / 50.0, 2, self.seed + 1));
        amp * phase.sin().abs().powf(0.6)
    }
}

/// Wool hanging in folds, shaded: lit from the moon (upper left), warmed by
/// the torch on the pillar, a little violet from the orb; dark in the
/// valleys, a soft sheen on the ridges as they turn away.
fn drape(x: f32, y: f32, base: V3, folds: &[(Folds, f32)], form: f32) -> V3 {
    let h = |x: f32, y: f32| folds.iter().map(|(f, w)| f.height(x, y) * w).sum::<f32>();
    let dx = h(x + 1.0, y) - h(x - 1.0, y);
    let dy = h(x, y + 1.0) - h(x, y - 1.0);
    let n = norm([-dx * 0.9, -dy * 0.9, 1.0]);
    let top = folds.iter().map(|(f, w)| f.depth * w * 1.6).sum::<f32>().max(0.01);
    let ao = 0.45 + 0.55 * (h(x, y) / top).clamp(0.0, 1.0).powf(0.7);
    let moon = norm([-0.6, -0.5, 0.62]);
    let diff = ((dot(n, moon) + 0.2) / 1.2).max(0.0);
    let to_t = [308.0 - x, 244.0 - y, 40.0];
    let td = dot(to_t, to_t).sqrt();
    let torch = dot(n, norm(to_t)).max(0.0) * (1.0 - td / 220.0).max(0.0).powi(2);
    let to_o = [ORB.0 - x, ORB.1 - y, 20.0];
    let odist = dot(to_o, to_o).sqrt();
    let orb = dot(n, norm(to_o)).max(0.0) * 30.0 / (odist + 30.0) * 0.25;
    let sheen = (1.0 - n[2]).powf(1.5) * 0.35 * diff;
    let weave = 0.94 + 0.06 * (x * 2.3).sin() * (y * 1.9).cos() + (fbm(x / 1.4, y / 1.4, 2, 93) - 0.5) * 0.08;
    let b = mul(base, weave);
    let light = addv(
        addv([0.12, 0.11, 0.15], mul([0.42, 0.47, 0.66], diff * 1.9 * form)),
        addv(mul(TORCH_COL, torch * 1.1), mul([0.7, 0.45, 1.0], orb)),
    );
    addv(mul(tint(b, light), ao), mul([0.5, 0.52, 0.68], sheen * form))
}

/// A pitch torch in an iron bracket on the arch's left pillar (its flame is runtime).
fn sconce(img: &mut [V3]) {
    let iron = |_: f32, _: f32, d: f32| mul([0.16, 0.15, 0.16], 1.2 - d * 0.6);
    paint_line(img, (306.0, 286.0), (314.0, 266.0), 4.0, &iron);
    paint_line(img, (300.0, 262.0), (316.0, 262.0), 4.0, &iron);
    paint_poly(img, &[(301.0, 250.0), (315.0, 250.0), (313.0, 262.0), (303.0, 262.0)], &|x, y| {
        let g = fbm(x / 2.0, y / 2.0, 2, 80);
        Some(mul([0.30, 0.18, 0.10], 0.7 + g * 0.5))
    });
}

fn warlock(img: &mut [V3]) {
    let grey = [0.21, 0.19, 0.26];
    // the robe pouring out of the arch, its ragged hem trailing in the mist
    let mut robe = vec![(330.0, 264.0), (352.0, 240.0), (440.0, 240.0), (464.0, 264.0), (470.0, 400.0), (476.0, 520.0)];
    for k in 0..12 {
        let x = 476.0 - k as f32 * 13.8;
        robe.push((x, if k % 2 == 0 { 540.0 } else { 518.0 + hash(k, 1, 60) * 10.0 }));
    }
    robe.push((310.0, 520.0));
    robe.push((322.0, 400.0));
    let robe_folds = [
        // bloused above the belt, gathered from the shoulders
        (Folds { focus: (398.0, 372.0), up: true, count: 9.0, depth: 4.5, wind: 0.0, seed: 101 }, 1.0),
        // falling from the belt to the hem
        (Folds { focus: (398.0, 352.0), up: false, count: 7.0, depth: 7.0, wind: 0.0, seed: 103 }, 1.0),
        // the raised arm drags the cloth up across the chest, from the armpit toward the far hip
        (Folds { focus: (300.0, 230.0), up: false, count: 14.0, depth: 3.0, wind: -0.3, seed: 105 }, 0.7),
    ];
    paint_poly(img, &robe, &|x, y| {
        // the body is round: the far side turns from the moon into shadow
        let form = 1.0 - ((x - 340.0) / 140.0).clamp(0.0, 1.0) * 0.6;
        let arm_w = (1.0 - ((y - 250.0) / 110.0).abs()).max(0.0) * (1.0 - ((x - 360.0) / 70.0).abs()).max(0.0);
        let weights = [if y < 372.0 { 1.0 } else { 0.0 }, if y >= 352.0 { 1.0 } else { 0.0 }, arm_w];
        let folds = [0, 1, 2].map(|k| (robe_folds[k].0, robe_folds[k].1 * weights[k]));
        Some(drape(x, y, grey, &folds, form))
    });
    // a rope belt, knotted, its end hanging
    let rope = |x: f32, y: f32, d: f32| {
        let twist = ((x * 1.4 + y * 1.4).sin() * 0.5 + 0.5) * 0.35;
        mul([0.40, 0.30, 0.18], (1.0 - d * 0.55) * (0.7 + twist) * (1.0 - ((x - 330.0) / 150.0).clamp(0.0, 1.0) * 0.5))
    };
    let mut prev = (330.0f32, 366.0f32);
    for k in 1..=20 {
        let q = k as f32 / 20.0;
        let p = (330.0 + q * 140.0, 366.0 + (q * std::f32::consts::PI).sin() * 8.0);
        paint_line(img, prev, p, 4.0, &rope);
        prev = p;
    }
    paint_disc(img, 372.0, 373.0, 5.0, 4.5, &|x, y, d| rope(x, y, d * 0.8));
    let mut prev = (372.0f32, 376.0f32);
    for k in 1..=10 {
        let q = k as f32 / 10.0;
        let p = (372.0 - (q * 2.6).sin() * 5.0, 376.0 + q * 52.0);
        paint_line(img, prev, p, 3.0, &rope);
        prev = p;
    }
    paint_poly(img, &[(368.0, 426.0), (376.0, 426.0), (380.0, 444.0), (364.0, 444.0)], &|x, y| Some(rope(x, y, 0.2)));
    // the hood
    let mut hood = Vec::new();
    for k in 0..=24 {
        let a = std::f32::consts::PI * (1.0 + k as f32 / 24.0);
        hood.push((396.0 + a.cos() * 50.0, 202.0 + a.sin() * 86.0));
    }
    hood.push((448.0, 252.0));
    hood.push((396.0, 266.0));
    hood.push((344.0, 252.0));
    let hood_folds = [(Folds { focus: (396.0, 112.0), up: false, count: 8.0, depth: 4.0, wind: 0.0, seed: 107 }, 1.0)];
    paint_poly(img, &hood, &|x, y| Some(drape(x, y, mul(grey, 1.05), &hood_folds, 1.0 - ((x - 360.0) / 100.0).clamp(0.0, 1.0) * 0.5)));
    // the dark within the hood: deepest at the centre
    paint_disc(img, 397.0, 200.0, 22.0, 36.0, &|_, _, d| {
        let k = d.powi(2);
        [0.01 + 0.07 * k, 0.008 + 0.06 * k, 0.016 + 0.08 * k]
    });
    // old gold worked round the hood's opening, gone dark with age
    for k in 0..40 {
        let a0 = k as f32 / 40.0 * std::f32::consts::TAU;
        let a1 = (k + 1) as f32 / 40.0 * std::f32::consts::TAU;
        let at = |a: f32| (397.0 + a.cos() * 25.0, 202.0 + a.sin() * 39.0);
        if (k % 20) > 15 {
            continue;
        }
        paint_line(img, at(a0), at(a1), 2.5, &|x, y, d| {
            let g = fbm(x / 1.5, y / 1.5, 2, 82);
            mul([0.46, 0.32, 0.14], (1.0 - d * 0.5) * (0.4 + g * 0.6) * if x < 397.0 { 1.0 } else { 0.55 })
        });
    }
    // the cowl's lip overhangs the dark: a lit fold across the brow
    paint_line(img, (374.0, 176.0), (397.0, 160.0), 6.0, &|x, y, d| cloth(x, y, mul(grey, 1.3 - d * 0.4), 0.0));
    paint_line(img, (397.0, 160.0), (420.0, 176.0), 6.0, &|x, y, d| cloth(x, y, mul(grey, 0.9 - d * 0.3), 0.0));
    arm(img);
}

// ---- his right arm: a small ray-marched model --------------------------------------
//
// He faces us out of the arch and reaches across to our left with his right
// arm: the shoulder high under the robe, the elbow a little bent and dropped
// by the arm's weight, the forearm coming slightly toward us, the wrist
// relaxed. The bell sleeve hangs from the top of the arm, so its cloth falls
// below the forearm and opens wide at the wrist; it bunches where the elbow
// bends. Out of it comes the hand, palm down, as God's reaches for Adam on
// the Sistine ceiling: the index finger long and nearly straight toward the
// void he has called up, the others falling away in a loose curl, the thumb
// beneath and nearest us. Pixel units, y down, z toward the viewer.

fn sub3(a: V3, b: V3) -> V3 {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}
fn cross(a: V3, b: V3) -> V3 {
    [a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0]]
}
fn len3(a: V3) -> f32 {
    dot(a, a).sqrt()
}
fn smin(a: f32, b: f32, k: f32) -> f32 {
    let h = (0.5 + 0.5 * (b - a) / k).clamp(0.0, 1.0);
    b + (a - b) * h - k * h * (1.0 - h)
}

const SHOULDER: V3 = [354.0, 252.0, -8.0];
const ELBOW: V3 = [314.0, 284.0, 3.0];
/// The sleeve's open end.
const CUFF: V3 = [268.0, 296.0, 9.0];

/// One bone of the hand: a capsule from `a` to `b` whose cross-section is
/// an ellipse, flatter across the back (`w`) than it is wide.
struct Seg {
    a: V3,
    b: V3,
    ra: f32,
    rb: f32,
    w: V3,
    flat: f32,
    /// 0 the palm, 1..=4 index to little finger, 5 the thumb
    finger: u8,
    /// which joint of the finger (2 the last)
    joint: u8,
}

impl Seg {
    fn dist(&self, p: V3) -> (f32, f32) {
        let ab = sub3(self.b, self.a);
        let t = (dot(sub3(p, self.a), ab) / dot(ab, ab)).clamp(0.0, 1.0);
        let q = sub3(p, addv(self.a, mul(ab, t)));
        let qw = dot(q, self.w);
        let qv2 = (dot(q, q) - qw * qw).max(0.0);
        let r = self.ra + (self.rb - self.ra) * t;
        let e = (qv2 + (qw / self.flat).powi(2)).sqrt();
        ((e - r) * self.flat.min(1.0), t)
    }
}

fn hand_model() -> Vec<Seg> {
    // along the hand (the forearm's line, the wrist letting it droop a little),
    // the back of the hand's normal (up, tipped toward us), and across the
    // knuckles from the index (near us) to the little finger (away)
    let fore = norm(sub3(CUFF, ELBOW));
    let u = norm(addv(fore, [0.0, -0.12, 0.0]));
    let w0 = [0.0, -1.0, 0.7];
    let w = norm(sub3(w0, mul(u, dot(w0, u))));
    let mut v = cross(w, u);
    if v[2] > 0.0 {
        v = mul(v, -1.0);
    }
    let wrist = addv(CUFF, mul(fore, -5.0));
    const S: f32 = 1.2;
    let at = |along: f32, across: f32, up: f32| addv(addv(addv(wrist, mul(u, along * S)), mul(v, across * S)), mul(w, up * S));
    let mut segs = Vec::new();
    // the wrist and the back of the hand: broad, flat, narrowing to the wrist
    segs.push(Seg { a: at(-6.0, 0.5, -0.5), b: at(4.0, 0.5, 0.0), ra: 5.6 * S, rb: 6.2 * S, w, flat: 0.6, finger: 0, joint: 0 });
    segs.push(Seg { a: at(4.0, -3.4, 0.3), b: at(25.0, -4.8, 0.6), ra: 4.8 * S, rb: 4.4 * S, w, flat: 0.5, finger: 0, joint: 0 });
    segs.push(Seg { a: at(4.0, 3.2, 0.0), b: at(24.0, 4.8, -0.2), ra: 4.6 * S, rb: 4.0 * S, w, flat: 0.5, finger: 0, joint: 0 });
    // the fingers: knuckle across-position, along-position, bone lengths, base radius, and each joint's bend
    type Finger = (f32, f32, [f32; 3], f32, [f32; 3]);
    let fingers: [Finger; 4] = [
        (-6.9, 26.5, [12.5, 8.0, 6.5], 2.7, [0.12, 0.16, 0.1]),
        (-2.3, 26.5, [13.0, 8.5, 6.5], 2.8, [0.3, 0.42, 0.25]),
        (2.3, 25.5, [12.0, 8.0, 6.0], 2.65, [0.4, 0.5, 0.28]),
        (6.4, 23.5, [9.5, 6.0, 5.0], 2.3, [0.5, 0.52, 0.3]),
    ];
    for (k, &(across, along, lens, r, bends)) in fingers.iter().enumerate() {
        let mut p = at(along, across, 0.4 - k as f32 * 0.25);
        // the fingers fan a little: index and little splay outward
        let splay = (k as f32 - 1.3) * 0.07;
        let mut d = norm(addv(u, mul(v, splay)));
        let mut n = w;
        let mut rr = r * S;
        for j in 0..3 {
            let (s, c) = bends[j].sin_cos();
            let nd = norm(sub3(mul(d, c), mul(n, s)));
            n = norm(addv(mul(n, c), mul(d, s)));
            d = nd;
            let q = addv(p, mul(d, lens[j] * S));
            let rq = rr * if j == 2 { 0.78 } else { 0.9 };
            segs.push(Seg { a: p, b: q, ra: rr, rb: rq, w: n, flat: 0.8, finger: k as u8 + 1, joint: j as u8 });
            p = q;
            rr = rq;
        }
    }
    // the thumb: from the heel of the hand forward along the index finger's
    // near side, a little below it, its tip resting by the index's middle joint
    let t0 = at(3.0, -4.5, -2.0);
    let t1 = at(14.0, -9.2, -3.4);
    let t2 = at(23.0, -10.4, -4.2);
    let t3 = at(29.5, -9.4, -3.6);
    segs.push(Seg { a: t0, b: t1, ra: 5.2 * S, rb: 3.2 * S, w, flat: 0.7, finger: 5, joint: 0 });
    segs.push(Seg { a: t1, b: t2, ra: 3.0 * S, rb: 2.7 * S, w, flat: 0.8, finger: 5, joint: 1 });
    segs.push(Seg { a: t2, b: t3, ra: 2.6 * S, rb: 2.1 * S, w, flat: 0.8, finger: 5, joint: 2 });
    segs
}

/// What a ray met: the hand (which bone, and where along it), the sleeve's
/// outside, or the dark inside of the sleeve.
#[derive(Clone, Copy, PartialEq)]
enum Part {
    Bone(usize, f32),
    Sleeve(f32),
    Lining,
}

/// The sleeve: a tube round the arm whose lower side is drawn down by its
/// own weight, more and more toward the open bell at the wrist, with folds
/// hanging from the top of the arm and bunching at the elbow.
fn sleeve(p: V3) -> (f32, Part) {
    let mut best = (f32::MAX, Part::Lining);
    let gravity = [0.0, 1.0, 0.0];
    for (k, (a, b, r0, r1, h0, h1)) in [(SHOULDER, ELBOW, 12.5f32, 11.0f32, 0.45f32, 0.7f32), (ELBOW, CUFF, 10.0, 14.0, 0.7, 2.9)].into_iter().enumerate() {
        let ab = sub3(b, a);
        let len = len3(ab);
        let ax = mul(ab, 1.0 / len);
        let along = dot(sub3(p, a), ax);
        let t = (along / len).clamp(0.0, 1.0);
        let o = sub3(p, addv(a, mul(ax, t * len)));
        let down = norm(sub3(gravity, mul(ax, dot(gravity, ax))));
        let side = cross(ax, down);
        let od = dot(o, down);
        let os = dot(o, side);
        let oa = dot(o, ax);
        let hang = h0 + (h1 - h0) * t;
        // the cloth below the arm hangs: its section is a drop, not a circle
        let od2 = if od > 0.0 { od / (1.0 + hang) } else { od };
        let theta = os.atan2(od);
        let below = (od / (od.abs() + os.abs() + 0.01)).max(0.0);
        let r = r0 + (r1 - r0) * t;
        // hanging folds, deeper where more cloth hangs
        let fold = below * (0.6 + hang * 0.8) * (theta * 7.0 + t * 4.0 + fbm(t * 3.0, theta, 2, 91) * 2.0).sin();
        // the elbow: the cloth bunches in rings on the inside of the bend
        let near_elbow = if k == 0 { ((t - 0.75) / 0.25).max(0.0) } else { (1.0 - t / 0.3).max(0.0) };
        let bunch = near_elbow * 0.9 * (along * 0.95).sin();
        let r = r + fold + bunch;
        let outer = (od2 * od2 + os * os + oa * oa).sqrt() - r;
        let (d, part) = if k == 1 {
            // the forearm's end is open: a shell of cloth, cut at the cuff
            let shell = (outer + 1.2).abs() - 1.2;
            let cut = along - len;
            let d = shell.max(cut);
            (d, if outer < -1.0 { Part::Lining } else { Part::Sleeve(t + 1.0) })
        } else {
            (outer, Part::Sleeve(t))
        };
        if d < best.0 {
            best = (d, part);
        }
    }
    best
}

fn arm(img: &mut [V3]) {
    let segs = hand_model();
    let hand_sdf = |p: V3| -> (f32, usize, f32) {
        let mut best = (f32::MAX, 0usize, 0.0f32);
        let mut d = f32::MAX;
        for (i, s) in segs.iter().enumerate() {
            let (di, t) = s.dist(p);
            if di < best.0 {
                best = (di, i, t);
            }
            // skin webs a little between bones; fingers stay apart past their first joint
            // and the thumb's base is one with the palm, the web of skin to the index
            let k = if s.finger == 5 && s.joint == 0 { 3.5 } else if s.joint == 0 { 1.4 } else { 0.5 };
            d = if d == f32::MAX { di } else { smin(d, di, k) };
        }
        (d, best.1, best.2)
    };
    let sdf = |p: V3| -> (f32, Part) {
        let (dh, i, t) = hand_sdf(p);
        let (ds, part) = sleeve(p);
        if dh < ds {
            (dh, Part::Bone(i, t))
        } else {
            (ds, part)
        }
    };
    let dist = |p: V3| sdf(p).0;
    let normal = |p: V3| {
        let e = 0.25;
        norm([
            dist([p[0] + e, p[1], p[2]]) - dist([p[0] - e, p[1], p[2]]),
            dist([p[0], p[1] + e, p[2]]) - dist([p[0], p[1] - e, p[2]]),
            dist([p[0], p[1], p[2] + e]) - dist([p[0], p[1], p[2] - e]),
        ])
    };
    let soft_shadow = |p: V3, l: V3, max: f32| {
        let mut sh = 1.0f32;
        let mut tt = 1.0;
        while tt < max {
            let h = dist(addv(p, mul(l, tt)));
            sh = sh.min((4.0 * h / tt).max(0.0));
            if sh < 0.02 {
                break;
            }
            tt += h.max(0.5);
        }
        sh
    };
    let orb: V3 = [ORB.0, ORB.1, 8.0];
    let moon_dir = norm([-0.45, -0.65, 0.6]);
    let torch: V3 = [308.0, 244.0, 30.0];
    let skin = [0.64, 0.52, 0.47];
    let wool = [0.21, 0.19, 0.26];
    // a camera a little in front of the picture, so the nearer hand is a touch larger
    let cam: V3 = [240.0, 300.0, 320.0];
    let shade = |x: f32, y: f32| -> Option<(V3, f32)> {
        let dir = norm(sub3([x, y, 0.0], cam));
        let mut s = (40.0 - cam[2]) / dir[2];
        let mut hit = None;
        for _ in 0..72 {
            let p = addv(cam, mul(dir, s));
            if p[2] < -40.0 {
                break;
            }
            let (d, part) = sdf(p);
            if d < 0.05 {
                hit = Some((p, part));
                break;
            }
            s += d.max(0.08) * 0.9;
        }
        let (p, part) = hit?;
        let n = normal(p);
        let view = mul(dir, -1.0);
        // ambient occlusion from how close the rest is along the normal
        let mut ao = 1.0f32;
        for k in 1..=3 {
            let h = k as f32 * 1.5;
            ao -= (h - dist(addv(p, mul(n, h)))).max(0.0) * 0.14 / k as f32;
        }
        let ao = ao.clamp(0.2, 1.0);
        let to_o = sub3(orb, p);
        let od = len3(to_o);
        let lo = mul(to_o, 1.0 / od);
        let to_t = norm(sub3(torch, p));
        let wrap = |l: V3, k: f32| ((dot(n, l) + k) / (1.0 + k)).max(0.0);
        let sh_o = soft_shadow(p, lo, od - 9.0);
        let sh_m = soft_shadow(p, moon_dir, 40.0);
        let sh_t = soft_shadow(p, to_t, 50.0);
        let fade = match part {
            // the upper sleeve melts into the painted robe at the shoulder
            Part::Sleeve(t) if t < 1.0 => (t / 0.35).clamp(0.0, 1.0),
            _ => 1.0,
        };
        let c = match part {
            Part::Bone(i, t) => {
                let seg = &segs[i];
                let orb_l = wrap(lo, 0.35) * sh_o * 1100.0 / (od * od + 300.0);
                let g = fbm(x / 1.6, y / 1.6, 3, 71);
                let mut base = mul(skin, 0.88 + g * 0.24);
                if seg.finger > 0 && seg.joint < 2 && t > 0.85 {
                    base = lerp3(base, [0.62, 0.40, 0.38], 0.4);
                }
                if seg.finger == 0 {
                    let vein = (fbm(x / 5.0, y / 2.2, 2, 72) - 0.5).abs();
                    if vein < 0.03 {
                        base = lerp3(base, [0.45, 0.45, 0.55], 0.4);
                    }
                }
                let lw = dot(sub3(p, addv(seg.a, mul(sub3(seg.b, seg.a), t))), seg.w);
                let nail = seg.joint == 2 && t > 0.45 && lw > 0.0;
                if nail {
                    base = [0.70, 0.62, 0.58];
                }
                // light bounced up off the flagstones finds the underside of the fingers
                let bounce = mul([0.22, 0.19, 0.2], wrap(norm([0.0, 1.0, 0.5]), 0.3) * ao);
                let light = addv(
                    addv(addv(mul([0.13, 0.11, 0.12], ao), bounce), mul([0.36, 0.40, 0.55], wrap(moon_dir, 0.35) * sh_m * 0.8 * ao)),
                    addv(mul([1.0, 0.56, 0.26], wrap(to_t, 0.35) * sh_t * 0.45 * ao), mul([0.72, 0.48, 1.0], orb_l)),
                );
                let mut c = tint(base, light);
                // light through thin skin comes out warm and red
                let thin = (1.0 - dot(n, lo).abs()).powi(3);
                c = addv(c, mul([0.30, 0.07, 0.05], thin * orb_l * 0.6));
                let half = norm(addv(lo, view));
                let spec = dot(n, half).max(0.0).powf(if nail { 40.0 } else { 12.0 }) * if nail { 0.5 } else { 0.08 } * sh_o;
                addv(c, mul([0.9, 0.8, 1.0], spec * (orb_l.min(1.5) + 0.2)))
            }
            Part::Sleeve(t) => {
                // wool: a fine weave, a velvet sheen where the cloth turns away,
                // and a band of old gold at the cuff
                let weave = 0.92 + 0.08 * ((x * 2.1).sin() * (y * 2.3).cos());
                let g = fbm(x / 7.0, y / 7.0, 3, 92);
                let gold = t > 1.0 && dist_to_cuff(p) < 3.0;
                let base = if gold { mul([0.66, 0.48, 0.2], 0.7 + g * 0.5) } else { mul(wool, weave * (0.85 + g * 0.3)) };
                let orb_l = wrap(lo, 0.2) * sh_o * 700.0 / (od * od + 400.0);
                let light = addv(
                    addv(mul([0.08, 0.08, 0.11], ao), mul([0.40, 0.45, 0.62], wrap(moon_dir, 0.2) * sh_m * 0.95 * ao)),
                    addv(mul([1.0, 0.56, 0.26], wrap(to_t, 0.2) * sh_t * 0.55 * ao), mul([0.72, 0.45, 1.0], orb_l)),
                );
                let mut c = tint(base, light);
                let sheen = (1.0 - dot(n, view).abs()).powi(3) * 0.18 * ao;
                c = addv(c, mul([0.55, 0.55, 0.7], sheen));
                if gold {
                    let half = norm(addv(moon_dir, view));
                    let sp = dot(n, half).max(0.0).powf(16.0) * 0.4 * sh_m;
                    c = addv(c, [sp, sp * 0.85, sp * 0.5]);
                }
                c
            }
            Part::Lining => {
                // the dark inside of the sleeve, a little of the orb's light reaching in
                let orb_l = wrap(lo, 0.2) * sh_o * 300.0 / (od * od + 400.0);
                tint([0.30, 0.08, 0.10], addv(mul([0.04, 0.035, 0.05], ao), mul([0.7, 0.45, 1.0], orb_l)))
            }
        };
        Some((c, fade))
    };
    let (x0, x1, y0, y1) = (180, 372, 232, 372);
    let mut covered = vec![false; ((x1 - x0) * (y1 - y0)) as usize];
    for y in y0..y1 {
        for x in x0..x1 {
            let mut acc = [0.0f32; 3];
            let mut cover = 0.0;
            for sy in 0..2 {
                for sx in 0..2 {
                    if let Some((c, fade)) = shade(x as f32 + 0.25 + sx as f32 * 0.5, y as f32 + 0.25 + sy as f32 * 0.5) {
                        acc = addv(acc, mul(c, fade));
                        cover += fade;
                    }
                }
            }
            if cover > 0.0 {
                let i = (y * W + x) as usize;
                img[i] = lerp3(img[i], mul(acc, 1.0 / cover), cover / 4.0);
                covered[((y - y0) * (x1 - x0) + (x - x0)) as usize] = cover > 3.5;
            }
        }
    }
    // the arm's shadow on the robe behind it: from the torch, falling to the
    // right and down, and a softer one from the moon
    for y in y0..y1 {
        for x in 280..x1 {
            if covered[((y - y0) * (x1 - x0) + (x - x0)) as usize] {
                continue;
            }
            let p = [x as f32 + 0.5, y as f32 + 0.5, -16.0];
            let st = soft_shadow(p, norm(sub3(torch, p)), 60.0);
            let sm = soft_shadow(p, moon_dir, 40.0);
            let k = (0.45 + 0.55 * st) * (0.7 + 0.3 * sm);
            let i = (y * W + x) as usize;
            img[i] = mul(img[i], k);
        }
    }
}

/// How far a point on the forearm's sleeve is from its open end.
fn dist_to_cuff(p: V3) -> f32 {
    let ax = norm(sub3(CUFF, ELBOW));
    dot(sub3(CUFF, p), ax)
}

/// The young wizard, seen from behind, ray-marched like the Warlock's arm:
/// a broad felt hat whose crown has slumped and bent back with age, a band
/// of old gold round it; a sapphire wool cloak hanging from his shoulders in
/// pipe folds that the cellar wind takes to the left; his right arm raised
/// in its sleeve, the gloved fist round a gnarled staff whose crystal lights
/// him cold blue from above.
#[derive(Clone, Copy, PartialEq)]
enum HeroPart {
    Brim,
    Crown(f32),
    Cloak,
    Sleeve,
    Glove,
    Staff,
}

fn hero_sdf(p: V3) -> (f32, HeroPart) {
    let mut best = (f32::MAX, HeroPart::Cloak);
    let mut take = |d: f32, part: HeroPart| {
        if d < best.0 {
            best = (d, part);
        }
    };
    // the brim: a wide disc of felt, sagging toward its edge (tilted a little toward us)
    let (bx, by, bz) = (124.0f32, 362.0f32, 0.0f32);
    let (rx, rz) = (p[0] - bx, p[2] - bz);
    let rr = (rx * rx + rz * rz).sqrt();
    let sag = (rr / 50.0).powi(2) * 5.0 - rz * 0.38;
    let brim_y = p[1] - (by + sag);
    let brim = (rr - 50.0).max(brim_y.abs() - 1.2) - 0.7;
    take(brim, HeroPart::Brim);
    // the crown: a cone along a bent spine, its felt creased
    let spine: [(V3, f32); 6] = [
        ([124.0, 362.0, 0.0], 21.0),
        ([122.0, 334.0, -2.0], 16.0),
        ([115.0, 308.0, -4.0], 11.0),
        ([102.0, 287.0, -6.0], 7.0),
        ([84.0, 274.0, -6.0], 3.8),
        ([66.0, 271.0, -4.0], 1.4),
    ];
    let mut crown = f32::MAX;
    let mut along = 0.0;
    for k in 0..5 {
        let (a, ra) = spine[k];
        let (b, rb) = spine[k + 1];
        let ab = sub3(b, a);
        let t = (dot(sub3(p, a), ab) / dot(ab, ab)).clamp(0.0, 1.0);
        let q = sub3(p, addv(a, mul(ab, t)));
        let crease = (fbm(t * 2.0 + k as f32, (q[0] + q[2]) * 0.2, 2, 120) - 0.5) * 2.2;
        let d = len3(q) - (ra + (rb - ra) * t + crease);
        if d < crown {
            crown = d;
            along = (k as f32 + t) / 5.0;
        }
    }
    // the crown sits on the brim; nothing of it hangs below
    take(crown.max(p[1] - (by - 1.0)), HeroPart::Crown(along));
    // the cloak: from narrow shoulders under the brim out to a wide hem, blown left
    let top = 368.0;
    if p[1] > top - 6.0 {
        let dy = (p[1] - top).max(0.0);
        let wind = -0.003 * dy * dy;
        let cx = 124.0 + wind;
        let r = 30.0 + dy * 0.42;
        let (ox, oz) = (p[0] - cx, (p[2] + 4.0) * 1.25);
        let theta = oz.atan2(ox);
        let folds = (0.8 + dy * 0.075) * (theta * 9.0 + fbm(dy / 60.0, theta, 2, 121) * 2.5 + dy * 0.012).sin().abs().powf(0.6);
        let cloak = (ox * ox + oz * oz).sqrt() - (r + folds);
        take(cloak.max(top - 6.0 - p[1]), HeroPart::Cloak);
    }
    // the raised right arm in its sleeve, up and in to the fist on the staff
    let shoulder: V3 = [148.0, 390.0, 2.0];
    let elbow: V3 = [178.0, 376.0, 8.0];
    let fist: V3 = [161.0, 330.0, 13.0];
    let capsule = |p: V3, a: V3, b: V3, ra: f32, rb: f32| {
        let ab = sub3(b, a);
        let t = (dot(sub3(p, a), ab) / dot(ab, ab)).clamp(0.0, 1.0);
        len3(sub3(p, addv(a, mul(ab, t)))) - (ra + (rb - ra) * t)
    };
    // the forearm's sleeve is wide and hangs: its section drawn down by its weight
    let hanging = |p: V3, a: V3, b: V3, ra: f32, rb: f32, h0: f32, h1: f32| {
        let ab = sub3(b, a);
        let t = (dot(sub3(p, a), ab) / dot(ab, ab)).clamp(0.0, 1.0);
        let mut o = sub3(p, addv(a, mul(ab, t)));
        if o[1] > 0.0 {
            o[1] /= 1.0 + h0 + (h1 - h0) * t;
        }
        len3(o) - (ra + (rb - ra) * t)
    };
    let sleeve = smin(capsule(p, shoulder, elbow, 8.0, 6.5), hanging(p, elbow, sub3(fist, [0.0, -7.0, 0.0]), 6.0, 8.0, 0.4, 1.1), 2.0);
    take(sleeve, HeroPart::Sleeve);
    take(len3(sub3(p, fist)) - 5.2, HeroPart::Glove);
    // the staff: a gnarled shaft from the floor to the crystal
    let foot: V3 = [140.0, 560.0, 14.0];
    let tip: V3 = [STAFF_TIP.0, STAFF_TIP.1 + 10.0, 12.0];
    let ab = sub3(tip, foot);
    let t = (dot(sub3(p, foot), ab) / dot(ab, ab)).clamp(0.0, 1.0);
    let axis = addv(foot, mul(ab, t));
    let wobble = [(t * 19.0).sin() * 1.5, 0.0, (t * 13.0).cos() * 1.0];
    let knots = ((t * 9.0).fract() - 0.5).abs() < 0.06;
    let staff = len3(sub3(p, addv(axis, wobble))) - (2.6 - t * 0.6 + if knots { 0.9 } else { 0.0 });
    take(staff, HeroPart::Staff);
    // the fork that cradles the crystal
    for side in [-1.0f32, 1.0] {
        let a = [STAFF_TIP.0, STAFF_TIP.1 + 12.0, 12.0];
        let b = [STAFF_TIP.0 + side * 6.0, STAFF_TIP.1 + 2.0, 12.0];
        let c = [STAFF_TIP.0 + side * 3.0, STAFF_TIP.1 - 7.0, 12.0];
        take(capsule(p, a, b, 2.0, 1.6).min(capsule(p, b, c, 1.6, 1.0)), HeroPart::Staff);
    }
    best
}

fn hero(img: &mut [V3]) {
    let dist = |p: V3| hero_sdf(p).0;
    let normal = |p: V3| {
        let e = 0.3;
        norm([
            dist([p[0] + e, p[1], p[2]]) - dist([p[0] - e, p[1], p[2]]),
            dist([p[0], p[1] + e, p[2]]) - dist([p[0], p[1] - e, p[2]]),
            dist([p[0], p[1], p[2] + e]) - dist([p[0], p[1], p[2] - e]),
        ])
    };
    let crystal: V3 = [STAFF_TIP.0, STAFF_TIP.1, 14.0];
    let orb: V3 = [ORB.0, ORB.1, 8.0];
    let moon_dir = norm([-0.45, -0.65, 0.6]);
    let torch: V3 = [308.0, 244.0, 30.0];
    let cam: V3 = [240.0, 330.0, 360.0];
    let shade = |x: f32, y: f32| -> Option<V3> {
        let dir = norm(sub3([x, y, 0.0], cam));
        let mut s = (60.0 - cam[2]) / dir[2];
        let mut hit = None;
        for _ in 0..80 {
            let p = addv(cam, mul(dir, s));
            if p[2] < -70.0 {
                break;
            }
            let (d, part) = hero_sdf(p);
            if d < 0.08 {
                hit = Some((p, part));
                break;
            }
            s += d.max(0.1) * 0.85;
        }
        let (p, part) = hit?;
        let n = normal(p);
        let view = mul(dir, -1.0);
        let mut ao = 1.0f32;
        for k in 1..=3 {
            let h = k as f32 * 2.0;
            ao -= (h - dist(addv(p, mul(n, h)))).max(0.0) * 0.12 / k as f32;
        }
        let ao = ao.clamp(0.25, 1.0);
        let wrap = |l: V3, k: f32| ((dot(n, l) + k) / (1.0 + k)).max(0.0);
        let point = |at: V3, power: f32| {
            let to = sub3(at, p);
            let d = len3(to);
            wrap(mul(to, 1.0 / d), 0.25) * power / (d * d + 400.0)
        };
        let crystal_l = point(crystal, 9000.0);
        let orb_l = point(orb, 2500.0);
        let torch_l = point(torch, 9000.0);
        let moon_l = wrap(moon_dir, 0.25);
        let bounce = wrap([0.0, 1.0, 0.3], 0.3) * 0.15;
        let light = addv(
            addv(mul([0.09, 0.09, 0.12], ao), mul([0.40, 0.45, 0.62], moon_l * 0.75 * ao)),
            addv(addv(mul([0.35, 0.62, 1.0], crystal_l), mul([0.72, 0.45, 1.0], orb_l)), addv(mul(TORCH_COL, torch_l * ao), mul([0.2, 0.18, 0.2], bounce * ao))),
        );
        let g = fbm(x / 6.0, y / 6.0, 3, 122);
        let weave = 0.94 + 0.06 * (x * 2.2).sin() * (y * 2.0).cos();
        let felt = [0.12, 0.20, 0.46];
        let gold = [0.70, 0.52, 0.22];
        let (base, gloss) = match part {
            HeroPart::Brim => (mul(felt, 0.85 + g * 0.3), 0.0),
            HeroPart::Crown(a) => {
                if (0.02..0.11).contains(&a) {
                    // the gold band, and the sapphire set in it
                    if ((x - HAT_GEM.0).powi(2) + (y - HAT_GEM.1).powi(2)) < 10.0 {
                        ([0.4, 0.7, 1.0], 0.6)
                    } else {
                        (mul(gold, 0.7 + g * 0.6), 0.45)
                    }
                } else {
                    (mul(felt, 0.9 + g * 0.25), 0.0)
                }
            }
            HeroPart::Cloak => {
                // the hem is worked in gold
                let hem = y > 528.0;
                if hem { (mul(gold, 0.6 + g * 0.6), 0.3) } else { (mul([0.12, 0.21, 0.48], weave * (0.85 + g * 0.3)), 0.0) }
            }
            HeroPart::Sleeve => (mul([0.13, 0.22, 0.50], weave * (0.85 + g * 0.3)), 0.0),
            HeroPart::Glove => ([0.26, 0.16, 0.10], 0.15),
            HeroPart::Staff => {
                let grain = fbm(x / 1.5, y / 5.0, 3, 123);
                (mul([0.34, 0.22, 0.13], 0.6 + grain * 0.7), 0.1)
            }
        };
        let mut c = tint(base, light);
        // wool's soft sheen as it turns from us; gold and leather catch a highlight
        if matches!(part, HeroPart::Cloak | HeroPart::Sleeve | HeroPart::Brim | HeroPart::Crown(_)) {
            c = addv(c, mul([0.35, 0.45, 0.75], (1.0 - dot(n, view).abs()).powi(3) * 0.22 * (moon_l + crystal_l * 0.5)));
        }
        if gloss > 0.0 {
            let lc = norm(sub3(crystal, p));
            let sp = dot(n, norm(addv(lc, view))).max(0.0).powf(20.0) * gloss * (0.4 + crystal_l);
            c = addv(c, [sp * 0.8, sp * 0.9, sp]);
        }
        Some(c)
    };
    for y in 262..H {
        for x in 0..236 {
            let mut acc = [0.0f32; 3];
            let mut cover = 0.0;
            for sy in 0..2 {
                for sx in 0..2 {
                    if let Some(c) = shade(x as f32 + 0.25 + sx as f32 * 0.5, y as f32 + 0.25 + sy as f32 * 0.5) {
                        acc = addv(acc, c);
                        cover += 1.0;
                    }
                }
            }
            if cover > 0.0 {
                let i = (y * W + x) as usize;
                img[i] = lerp3(img[i], mul(acc, 1.0 / cover), cover / 4.0);
            }
        }
    }
}

/// Where the staff's crystal sits, and the gem in the hat band.
pub const STAFF_TIP: (f32, f32) = (164.0, 296.0);
pub const HAT_GEM: (f32, f32) = (124.0, 347.0);

/// The Warlock's circle, cut into the flagstones before the arch.
pub const CIRCLE: (f32, f32, f32, f32) = (250.0, 452.0, 62.0, 15.0);

/// Black candles round the circle: (x, foot, height). The flames are runtime.
pub const CANDLES: [(f32, f32, f32); 5] = [(197.0, 445.0, 9.0), (303.0, 445.0, 8.0), (192.0, 460.0, 14.0), (250.0, 468.0, 11.0), (308.0, 460.0, 16.0)];
/// The amulet on his breast.
pub const AMULET: (f32, f32) = (396.0, 304.0);

fn candles(img: &mut [V3]) {
    for (k, &(x, foot, h)) in CANDLES.iter().enumerate() {
        let w = 2.6 + h * 0.08;
        // a little pool of spilt wax, then the stick, lit from the flame above and the circle below
        paint_disc(img, x, foot, w * 1.9, 1.6, &|_, _, d| mul([0.14, 0.12, 0.13], 1.0 - d * 0.4));
        paint_poly(img, &[(x - w, foot), (x - w, foot - h), (x + w, foot - h), (x + w, foot)], &|px, py| {
            let u = (px - x) / w;
            let round = (1.0 - u * u).max(0.0).sqrt();
            let drip = ((px * 1.7 + k as f32).sin() * 0.5 + 0.5) * 4.0;
            let near_top = (1.0 - (py - (foot - h)) / (h * 0.6 + drip)).clamp(0.0, 1.0);
            let warm = 0.25 + 0.75 * near_top;
            Some(addv(mul([0.16, 0.13, 0.14], 0.4 + round * 0.8), mul([1.0, 0.62, 0.3], warm * round * 0.35)))
        });
        paint_line(img, (x, foot - h), (x, foot - h - 2.5), 1.0, &|_, _, _| [0.05, 0.04, 0.04]);
    }
}

fn amulet(img: &mut [V3]) {
    let gold = |x: f32, y: f32, d: f32| {
        let g = fbm(x / 1.2, y / 1.2, 2, 84);
        mul([0.78, 0.58, 0.26], (1.0 - d * 0.6) * (0.55 + g * 0.6) * if x < 396.0 { 1.0 } else { 0.7 })
    };
    let (ax, ay) = AMULET;
    for side in [-1.0f32, 1.0] {
        let mut prev = (ax + side * 26.0, 262.0);
        for k in 1..=8 {
            let q = k as f32 / 8.0;
            let p = (ax + side * 26.0 * (1.0 - q), 262.0 + (ay - 8.0 - 262.0) * q + (q * std::f32::consts::PI).sin() * 6.0);
            if k % 2 == 0 {
                paint_disc(img, p.0, p.1, 1.6, 1.6, &gold);
            }
            paint_line(img, prev, p, 1.4, &gold);
            prev = p;
        }
    }
    // a gold setting holding a dark stone; its fire is runtime
    paint_disc(img, ax, ay, 8.0, 9.5, &gold);
    paint_disc(img, ax, ay, 5.0, 6.5, &|_, _, d| mul([0.10, 0.02, 0.16], 0.6 + (1.0 - d) * 0.8));
    paint_line(img, (ax, ay + 9.0), (ax, ay + 15.0), 2.5, &gold);
}

fn circle(img: &mut [V3]) {
    let (cx, cy, rx, ry) = CIRCLE;
    for y in (cy - ry - 2.0) as i32..=(cy + ry + 2.0) as i32 {
        for x in (cx - rx - 2.0) as i32..=(cx + rx + 2.0) as i32 {
            let (px, py) = (x as f32 + 0.5, y as f32 + 0.5);
            let (dx, dy) = ((px - cx) / rx, (py - cy) / ry);
            let r = (dx * dx + dy * dy).sqrt();
            let a = dy.atan2(dx);
            let band = (0.84..1.0).contains(&r);
            let inner = (0.6..0.66).contains(&r);
            let glyph = (0.88..0.96).contains(&r) && ((a * 12.0).fract() < 0.18);
            let i = (y * W + x) as usize;
            if band || inner {
                let k = if glyph { 0.3 } else { 0.55 };
                img[i] = addv(mul(img[i], k), [0.025, 0.0, 0.04]);
            }
        }
    }
}

/// The void he holds out, just past his fingertip.
pub const ORB: (f32, f32) = (190.0, 300.0);

pub fn title_art() -> Vec<u8> {
    let mut img = vec![[0.0f32; 3]; (W * H) as usize];
    render_maze(&mut img);
    arch(&mut img);
    inscription(&mut img);
    sconce(&mut img);
    circle(&mut img);
    candles(&mut img);
    warlock(&mut img);
    amulet(&mut img);
    hero(&mut img);
    // grade: teal in the shadows, amber in the light, a vignette, a little grain
    let mut out = Image::new(W as u32, H as u32);
    for y in 0..H {
        for x in 0..W {
            let mut c = img[(y * W + x) as usize];
            let l = 0.3 * c[0] + 0.59 * c[1] + 0.11 * c[2];
            let shadow = (1.0 - l * 3.0).clamp(0.0, 1.0);
            c = addv(c, [-0.01 * shadow, 0.012 * shadow, 0.02 * shadow]);
            let (dx, dy) = (x as f32 / W as f32 - 0.5, y as f32 / H as f32 - 0.45);
            let v = 1.0 - (dx * dx + dy * dy) * 1.1;
            let grain = (hash(x, y, 99) - 0.5) * 0.025;
            let f = |v0: f32| (((v0 * v.max(0.3)) + grain).clamp(0.0, 1.0).powf(1.0 / 1.15) * 255.0) as u8;
            out.set_rgba(x, y, f(c[0]), f(c[1]), f(c[2]), 255);
        }
    }
    out.encode_png()
}
