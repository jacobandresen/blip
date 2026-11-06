//! WARLOCK's artwork, painted in code: the cellar floor and wall tiles, the
//! character sheet (two robed wizards, the four monsters, the Warlock and a
//! dragon whelp, the props) and the title painting (title.rs).
//! Light, glow and spell effects are drawn at runtime; these are the still
//! pieces. The palette is candlelit stone: cool blue-grey shadow, warm amber
//! light, deep crimson and old gold accents.

use blip::image::Image;
use blip::Asset;

/// One sprite or tile cell, in pixels.
pub const CELL: u32 = 32;
/// Sprite sheet: columns of frames, one row per character.
pub const SHEET_COLS: u32 = 8;
pub const SHEET_ROWS: u32 = 9;
/// Tile sheet: eight columns, two rows (floors, walls).
pub const TILE_COLS: u32 = 8;
pub const TILE_ROWS: u32 = 2;

type C = [u8; 4];

const fn rgb(hex: u32) -> C {
    [(hex >> 16) as u8, (hex >> 8) as u8, hex as u8, 255]
}
const fn rgba(hex: u32, a: u8) -> C {
    [(hex >> 16) as u8, (hex >> 8) as u8, hex as u8, a]
}

const INK: C = rgb(0x07070b);

// ---- painting helpers -------------------------------------------------------

fn hash(x: i32, y: i32, s: u32) -> f32 {
    let mut h = (x as u32).wrapping_mul(374_761_393) ^ (y as u32).wrapping_mul(668_265_263) ^ s.wrapping_mul(2_246_822_519);
    h = (h ^ (h >> 13)).wrapping_mul(1_274_126_177);
    h ^= h >> 16;
    (h & 0xffff) as f32 / 65535.0
}

fn shade(c: C, k: f32) -> C {
    let f = |v: u8| (v as f32 * k).clamp(0.0, 255.0) as u8;
    [f(c[0]), f(c[1]), f(c[2]), c[3]]
}

fn mix(a: C, b: C, t: f32) -> C {
    let f = |x: u8, y: u8| (x as f32 + (y as f32 - x as f32) * t).round() as u8;
    [f(a[0], b[0]), f(a[1], b[1]), f(a[2], b[2]), f(a[3], b[3])]
}

fn put(img: &mut Image, x: i32, y: i32, c: C) {
    if c[3] == 255 {
        img.set_rgba(x, y, c[0], c[1], c[2], 255);
    } else {
        img.blend(x, y, c);
    }
}

fn get(img: &Image, x: i32, y: i32) -> C {
    if x < 0 || y < 0 || x >= img.w as i32 || y >= img.h as i32 {
        return [0; 4];
    }
    let o = ((y as u32 * img.w + x as u32) * 4) as usize;
    [img.px[o], img.px[o + 1], img.px[o + 2], img.px[o + 3]]
}

fn rect(img: &mut Image, x: i32, y: i32, w: i32, h: i32, c: C) {
    for yy in y..y + h {
        for xx in x..x + w {
            put(img, xx, yy, c);
        }
    }
}

/// A filled ellipse, sampled at pixel centres.
fn ellipse(img: &mut Image, cx: f32, cy: f32, rx: f32, ry: f32, c: C) {
    for y in (cy - ry - 1.0) as i32..=(cy + ry + 1.0) as i32 {
        for x in (cx - rx - 1.0) as i32..=(cx + rx + 1.0) as i32 {
            let dx = (x as f32 + 0.5 - cx) / rx;
            let dy = (y as f32 + 0.5 - cy) / ry;
            if dx * dx + dy * dy <= 1.0 {
                put(img, x, y, c);
            }
        }
    }
}

fn disc(img: &mut Image, cx: f32, cy: f32, r: f32, c: C) {
    ellipse(img, cx, cy, r, r, c);
}

/// A filled polygon (even-odd), sampled at pixel centres.
fn poly(img: &mut Image, pts: &[(f32, f32)], c: C) {
    let (mut x0, mut y0, mut x1, mut y1) = (f32::MAX, f32::MAX, f32::MIN, f32::MIN);
    for &(x, y) in pts {
        x0 = x0.min(x);
        y0 = y0.min(y);
        x1 = x1.max(x);
        y1 = y1.max(y);
    }
    for y in y0.floor() as i32..=y1.ceil() as i32 {
        for x in x0.floor() as i32..=x1.ceil() as i32 {
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
                put(img, x, y, c);
            }
        }
    }
}

/// A round-capped line `w` pixels wide.
fn line(img: &mut Image, a: (f32, f32), b: (f32, f32), w: f32, c: C) {
    let r = w / 2.0;
    let (x0, x1) = (a.0.min(b.0) - r - 1.0, a.0.max(b.0) + r + 1.0);
    let (y0, y1) = (a.1.min(b.1) - r - 1.0, a.1.max(b.1) + r + 1.0);
    let (dx, dy) = (b.0 - a.0, b.1 - a.1);
    let len2 = (dx * dx + dy * dy).max(0.0001);
    for y in y0 as i32..=y1 as i32 {
        for x in x0 as i32..=x1 as i32 {
            let (px, py) = (x as f32 + 0.5, y as f32 + 0.5);
            let t = (((px - a.0) * dx + (py - a.1) * dy) / len2).clamp(0.0, 1.0);
            let (qx, qy) = (a.0 + dx * t - px, a.1 + dy * t - py);
            if qx * qx + qy * qy <= r * r {
                put(img, x, y, c);
            }
        }
    }
}

/// Recolour every painted pixel from `f(x, y, colour)`; used for light falloff.
fn relight(img: &mut Image, f: impl Fn(i32, i32, C) -> C) {
    for y in 0..img.h as i32 {
        for x in 0..img.w as i32 {
            let c = get(img, x, y);
            if c[3] > 0 {
                let n = f(x, y, c);
                img.set_rgba(x, y, n[0], n[1], n[2], n[3]);
            }
        }
    }
}

/// A one-pixel ink outline around every painted shape: the crisp edge that
/// keeps the sprites readable against dark stone.
fn outline(img: &mut Image) {
    let src = Image { w: img.w, h: img.h, px: img.px.clone() };
    for y in 0..img.h as i32 {
        for x in 0..img.w as i32 {
            if get(&src, x, y)[3] > 40 {
                continue;
            }
            let near = [(-1, 0), (1, 0), (0, -1), (0, 1)].iter().any(|&(dx, dy)| get(&src, x + dx, y + dy)[3] > 120);
            if near {
                img.set_rgba(x, y, INK[0], INK[1], INK[2], 235);
            }
        }
    }
}

/// Copy `src` into `dst` at a pixel offset.
fn stamp(dst: &mut Image, src: &Image, ox: i32, oy: i32) {
    for y in 0..src.h as i32 {
        for x in 0..src.w as i32 {
            let c = get(src, x, y);
            if c[3] > 0 {
                dst.set_rgba(ox + x, oy + y, c[0], c[1], c[2], c[3]);
            }
        }
    }
}

fn mirror(img: &Image) -> Image {
    let mut out = Image::new(img.w, img.h);
    for y in 0..img.h as i32 {
        for x in 0..img.w as i32 {
            let c = get(img, x, y);
            out.set_rgba(img.w as i32 - 1 - x, y, c[0], c[1], c[2], c[3]);
        }
    }
    out
}

fn cell() -> Image {
    Image::new(CELL, CELL)
}

// ---- tiles ---------------------------------------------------------------------

const MORTAR: C = rgb(0x0b0c10);
const STONE: C = rgb(0x5d606b);
const FLAG: C = rgb(0x3d3832);

/// A flagstone floor tile: four uneven slabs with worn edges and grit.
fn floor_tile(seed: u32) -> Image {
    let mut img = cell();
    rect(&mut img, 0, 0, 32, 32, MORTAR);
    // slab layout: a split that wanders per seed
    let sx = 12 + (hash(1, 2, seed) * 8.0) as i32;
    let sy = 12 + (hash(3, 4, seed) * 8.0) as i32;
    let sy2 = 10 + (hash(5, 6, seed) * 12.0) as i32;
    let slabs = [(0, 0, sx, sy), (sx, 0, 32 - sx, sy2), (0, sy, sx, 32 - sy), (sx, sy2, 32 - sx, 32 - sy2)];
    for (i, &(x, y, w, h)) in slabs.iter().enumerate() {
        let tone = 0.82 + hash(i as i32, 9, seed) * 0.32;
        for yy in y + 1..y + h - 1 {
            for xx in x + 1..x + w - 1 {
                let n = hash(xx, yy, seed + i as u32 * 7);
                let mut c = shade(FLAG, tone * (0.92 + n * 0.14));
                // worn top-left lip catches light, bottom-right edge falls off
                if yy == y + 1 || xx == x + 1 {
                    c = shade(c, 1.22);
                }
                if yy == y + h - 2 || xx == x + w - 2 {
                    c = shade(c, 0.72);
                }
                put(&mut img, xx, yy, c);
            }
        }
    }
    // hairline cracks
    if hash(7, 7, seed) > 0.45 {
        let mut x = 4.0 + hash(8, 1, seed) * 24.0;
        let mut y = 3.0 + hash(8, 2, seed) * 8.0;
        for k in 0..10 {
            put(&mut img, x as i32, y as i32, shade(MORTAR, 1.3));
            x += hash(k, 3, seed) * 2.0 - 0.8;
            y += 1.4;
        }
    }
    img
}

/// A floor tile with something on it: bones, a drain grate, a candle stub, moss.
fn floor_decor(kind: u32) -> Image {
    let mut img = cell();
    let bone = rgb(0xcfc5ac);
    match kind {
        0 => {
            // a skull and a crossed bone
            line(&mut img, (8.0, 24.0), (20.0, 19.0), 2.0, shade(bone, 0.8));
            disc(&mut img, 8.0, 24.0, 1.6, shade(bone, 0.8));
            disc(&mut img, 20.0, 19.0, 1.6, shade(bone, 0.8));
            ellipse(&mut img, 21.0, 24.0, 4.2, 3.6, bone);
            rect(&mut img, 19, 26, 5, 2, shade(bone, 0.85));
            rect(&mut img, 19, 23, 2, 2, MORTAR);
            rect(&mut img, 22, 23, 2, 2, MORTAR);
        }
        1 => {
            // an iron drain grate
            rect(&mut img, 9, 9, 14, 14, rgb(0x16171c));
            for k in 0..4 {
                rect(&mut img, 10 + k * 3, 10, 2, 12, rgb(0x4a4d55));
            }
            rect(&mut img, 9, 9, 14, 1, rgb(0x5a5e68));
        }
        2 => {
            // moss creeping between slabs
            for y in 0..32 {
                for x in 0..32 {
                    let d = ((x - 6) * (x - 6) + (y - 26) * (y - 26)) as f32;
                    if d < 90.0 && hash(x, y, 77) > 0.35 + d / 200.0 {
                        put(&mut img, x, y, mix(rgb(0x2c3a26), rgb(0x46593a), hash(x, y, 78)));
                    }
                }
            }
        }
        _ => {
            // scattered straw / rubble
            for k in 0..14 {
                let x = 4.0 + hash(k, 1, 91) * 24.0;
                let y = 4.0 + hash(k, 2, 91) * 24.0;
                let a = hash(k, 3, 91) * std::f32::consts::TAU;
                line(&mut img, (x, y), (x + a.cos() * 3.0, y + a.sin() * 3.0), 1.0, rgb(0x5a4a2e));
            }
            disc(&mut img, 24.0, 8.0, 2.0, shade(STONE, 0.9));
            disc(&mut img, 9.0, 21.0, 1.5, shade(STONE, 0.8));
        }
    }
    img
}

/// The top of a wall seen from above: rough-cut capstones.
fn wall_cap(seed: u32) -> Image {
    let mut img = cell();
    rect(&mut img, 0, 0, 32, 32, MORTAR);
    for row in 0..4 {
        let off = if row % 2 == 0 { 0 } else { 8 };
        let mut x = -off;
        let mut k = 0;
        while x < 32 {
            let w = 14 + (hash(row, k, seed) * 6.0) as i32;
            let tone = 0.78 + hash(row, k + 30, seed) * 0.3;
            for yy in row * 8 + 1..row * 8 + 7 {
                for xx in x + 1..x + w - 1 {
                    let n = hash(xx, yy, seed);
                    let mut c = shade(rgb(0x1e2027), tone * (0.9 + n * 0.25));
                    if yy == row * 8 + 1 {
                        c = shade(c, 1.25);
                    }
                    put(&mut img, xx, yy, c);
                }
            }
            x += w;
            k += 1;
        }
    }
    img
}

/// The front face of a wall: coursed ashlar blocks, lit from above.
fn wall_face(seed: u32) -> Image {
    let mut img = cell();
    rect(&mut img, 0, 0, 32, 32, MORTAR);
    for row in 0..3 {
        let y0 = row * 11;
        let off = if row % 2 == 0 { 0 } else { 9 };
        let mut x = -off;
        let mut k = 0;
        while x < 32 {
            let w = 15 + (hash(row, k, seed + 5) * 5.0) as i32;
            let tone = 0.85 + hash(row, k + 11, seed) * 0.25;
            for yy in y0 + 1..y0 + 10 {
                for xx in x + 1..x + w - 1 {
                    let n = hash(xx, yy, seed + 3);
                    let fall = 1.15 - (yy - y0) as f32 * 0.04;
                    let mut c = shade(STONE, tone * fall * (0.9 + n * 0.16));
                    if yy == y0 + 1 {
                        c = shade(c, 1.3);
                    }
                    if yy == y0 + 9 {
                        c = shade(c, 0.7);
                    }
                    put(&mut img, xx, yy, c);
                }
            }
            x += w;
            k += 1;
        }
    }
    // a damp streak
    if seed % 2 == 1 {
        for y in 4..30 {
            put(&mut img, 22, y, rgba(0x0e1218, 90));
        }
    }
    img
}

/// The whole cellar floor as one picture: irregular flagstones cut from a
/// jittered Voronoi pattern, so the slabs run across the cell grid instead of
/// repeating per tile. `w`, `h` are in pixels.
pub fn flagstones(w: u32, h: u32, seed: u32) -> Image {
    let mut img = Image::new(w, h);
    let step = 17.0f32;
    let gw = (w as f32 / step) as i32 + 2;
    let gh = (h as f32 / step) as i32 + 2;
    let mut seeds = Vec::new();
    for gy in -1..gh {
        for gx in -1..gw {
            let jx = hash(gx, gy, seed) * 0.8 + 0.1;
            let jy = hash(gx, gy, seed + 1) * 0.8 + 0.1;
            // slabs stretch a little sideways, like laid paving
            seeds.push(((gx as f32 + jx) * step * 1.15, (gy as f32 + jy) * step, gx, gy));
        }
    }
    for y in 0..h as i32 {
        for x in 0..w as i32 {
            let (px, py) = (x as f32 + 0.5, y as f32 + 0.5);
            let (mut d1, mut d2) = (f32::MAX, f32::MAX);
            let mut best = (0.0f32, 0.0f32, 0, 0);
            for &(sx, sy, gx, gy) in &seeds {
                let dx = (sx - px) * 0.87;
                let dy = sy - py;
                if dx.abs() > step * 2.0 || dy.abs() > step * 2.0 {
                    continue;
                }
                let d = (dx * dx + dy * dy).sqrt();
                if d < d1 {
                    d2 = d1;
                    d1 = d;
                    best = (sx, sy, gx, gy);
                } else if d < d2 {
                    d2 = d;
                }
            }
            let edge = d2 - d1;
            let (sx, sy, gx, gy) = best;
            let tone = 0.78 + hash(gx, gy, seed + 2) * 0.34;
            let warm = hash(gx, gy, seed + 3);
            let base = mix(FLAG, rgb(0x34363d), warm * 0.6);
            let grain = 0.93 + hash(x, y, seed + 4) * 0.12;
            let col = if edge < 1.3 {
                shade(MORTAR, 1.6)
            } else {
                // each slab is lit from the upper left: lift that side, sink the other
                let lit = ((sx - px) * -0.010 + (sy - py) * -0.012).clamp(-0.12, 0.12);
                let mut k = tone * grain * (1.0 - lit);
                if edge < 2.6 {
                    // a worn bevel: the lip toward the light is brighter
                    k *= if (px - sx) + (py - sy) < 0.0 { 1.18 } else { 0.78 };
                }
                shade(base, k)
            };
            img.set_rgba(x, y, col[0], col[1], col[2], 255);
        }
    }
    // a few hairline cracks
    for k in 0..40 {
        let mut x = hash(k, 1, seed + 9) * w as f32;
        let mut y = hash(k, 2, seed + 9) * h as f32;
        let dir = hash(k, 3, seed + 9) * std::f32::consts::TAU;
        for j in 0..(4 + (hash(k, 4, seed) * 8.0) as i32) {
            let a = dir + (hash(k, j, seed + 11) - 0.5) * 1.4;
            x += a.cos() * 1.3;
            y += a.sin() * 1.3;
            put(&mut img, x as i32, y as i32, rgba(0x0b0c10, 170));
        }
    }
    img
}

fn tiles() -> Vec<u8> {
    let mut sheet = Image::new(CELL * TILE_COLS, CELL * TILE_ROWS);
    let row0: Vec<Image> = (0..4).map(floor_tile).chain((0..4).map(floor_decor)).collect();
    for (i, t) in row0.iter().enumerate() {
        stamp(&mut sheet, t, i as i32 * 32, 0);
    }
    let row1 = [wall_cap(1), wall_cap(2), wall_cap(3), wall_face(1), wall_face(2), wall_face(3), wall_cap(4), wall_face(4)];
    for (i, t) in row1.iter().enumerate() {
        stamp(&mut sheet, t, i as i32 * 32, 32);
    }
    sheet.encode_png()
}

// ---- characters -------------------------------------------------------------------

/// A wizard's colours: robe, robe shadow, trim, hat band.
#[derive(Clone, Copy)]
struct Robes {
    robe: C,
    trim: C,
}

const P1: Robes = Robes { robe: rgb(0x2f63b8), trim: rgb(0xd8b25a) };
const P2: Robes = Robes { robe: rgb(0xa8322c), trim: rgb(0xe8cf86) };

const SKIN: C = rgb(0xe0b48c);
const BEARD: C = rgb(0xd9d6cf);
const WOOD: C = rgb(0x6b4a2b);

/// Light falls from the upper left: lift the top-left of a shape, sink the rest.
fn sculpt(img: &mut Image, cx: f32, cy: f32) {
    relight(img, |x, y, c| {
        let k = 1.0 + ((cx - x as f32) * 0.018 + (cy - y as f32) * 0.022).clamp(-0.35, 0.3);
        shade(c, k)
    });
}

/// The wizard, facing the viewer (`back` for walking away).
/// `step` swings the hem; `cast` raises the staff.
fn wizard_front(r: Robes, step: i32, back: bool, cast: bool) -> Image {
    let mut img = cell();
    let dark = shade(r.robe, 0.62);
    let s = step as f32;
    // robe body: a flared bell
    poly(&mut img, &[(12.0, 14.0), (20.0, 14.0), (24.0 + s * 0.5, 30.0), (8.0 + s * 0.5, 30.0)], r.robe);
    // fold shadows
    line(&mut img, (14.0, 17.0), (12.0 + s * 0.5, 29.0), 1.4, dark);
    line(&mut img, (18.5, 17.0), (20.0 + s * 0.5, 29.0), 1.0, shade(r.robe, 0.8));
    // hem trim and front panel
    line(&mut img, (8.5 + s * 0.5, 29.5), (23.5 + s * 0.5, 29.5), 1.6, r.trim);
    if !back {
        line(&mut img, (16.0, 18.0), (16.0 + s * 0.3, 29.0), 1.2, r.trim);
        rect(&mut img, 12, 19, 8, 1, shade(r.trim, 0.8)); // belt
    }
    // feet peeking out under the hem
    if step != 0 {
        ellipse(&mut img, 16.0 - s * 3.0, 30.5, 2.2, 1.2, rgb(0x2a1d14));
    }
    // sleeves
    ellipse(&mut img, 11.0, 19.0, 2.6, 4.0, shade(r.robe, 0.85));
    ellipse(&mut img, 21.0, 19.0 - if cast { 4.0 } else { 0.0 }, 2.6, 4.0, shade(r.robe, 0.85));
    if back {
        // hood draped down the back
        poly(&mut img, &[(12.0, 12.0), (20.0, 12.0), (18.0, 19.0), (14.0, 19.0)], dark);
    } else {
        // face and beard
        ellipse(&mut img, 16.0, 12.0, 3.2, 3.0, SKIN);
        poly(&mut img, &[(12.8, 12.5), (19.2, 12.5), (16.0, 21.0)], BEARD);
        rect(&mut img, 14, 11, 1, 1, INK);
        rect(&mut img, 17, 11, 1, 1, INK);
        line(&mut img, (14.5, 14.0), (17.5, 14.0), 1.0, shade(BEARD, 0.75));
    }
    // the hat: a broad brim and a tall crooked cone
    ellipse(&mut img, 16.0, 9.5, 8.5, 2.2, dark);
    poly(&mut img, &[(11.0, 9.5), (21.0, 9.5), (18.0, 4.0), (21.5, -0.5), (15.0, 3.0)], r.robe);
    line(&mut img, (11.5, 8.6), (20.5, 8.6), 1.4, r.trim);
    // staff, in the right hand
    let (top, bot) = if cast { ((25.0, 2.0), (22.0, 22.0)) } else { ((24.0, 5.0), (24.0, 30.0)) };
    line(&mut img, top, bot, 1.8, WOOD);
    line(&mut img, (top.0 - 2.0, top.1 + 1.0), (top.0 + 2.0, top.1 + 1.0), 1.2, shade(r.trim, 0.85));
    disc(&mut img, top.0, top.1 - 0.5, 1.9, rgb(0xe9f6ff));
    disc(&mut img, 21.0, if cast { 16.0 } else { 20.5 }, 1.5, SKIN); // hand
    sculpt(&mut img, 12.0, 8.0);
    outline(&mut img);
    img
}

/// The wizard in profile, facing right.
fn wizard_side(r: Robes, step: i32, cast: bool) -> Image {
    let mut img = cell();
    let dark = shade(r.robe, 0.62);
    let s = step as f32;
    // trailing back of the robe, swinging with the stride
    poly(&mut img, &[(13.0, 14.0), (19.0, 14.0), (22.0 + s, 30.0), (8.0 - s, 30.0)], r.robe);
    line(&mut img, (14.0, 16.0), (10.0 - s, 29.0), 1.5, dark);
    line(&mut img, (17.0, 18.0), (19.0 + s, 29.0), 1.0, shade(r.robe, 0.82));
    line(&mut img, (8.5 - s, 29.5), (21.5 + s, 29.5), 1.6, r.trim);
    if step != 0 {
        ellipse(&mut img, 18.0 + s * 2.5, 30.5, 2.4, 1.2, rgb(0x2a1d14));
    }
    // face, nose and beard, looking right
    ellipse(&mut img, 18.0, 12.0, 2.6, 2.8, SKIN);
    rect(&mut img, 20, 12, 2, 1, shade(SKIN, 0.9));
    poly(&mut img, &[(16.0, 13.0), (21.0, 13.5), (19.5, 21.0), (16.5, 18.0)], BEARD);
    rect(&mut img, 19, 11, 1, 1, INK);
    // hat: brim and a cone that bends back over the shoulders
    ellipse(&mut img, 16.5, 9.5, 8.0, 2.0, dark);
    poly(&mut img, &[(11.5, 9.5), (21.0, 9.5), (17.0, 4.0), (9.0, 0.5), (13.0, 4.5)], r.robe);
    line(&mut img, (12.0, 8.6), (20.5, 8.6), 1.4, r.trim);
    // arm and staff, held out in front
    let (top, bot) = if cast { ((29.0, 7.0), (21.0, 24.0)) } else { ((24.0, 5.0), (24.0, 30.0)) };
    line(&mut img, top, bot, 1.8, WOOD);
    disc(&mut img, top.0, top.1 - 0.5, 1.9, rgb(0xe9f6ff));
    let hand = if cast { (26.0, 13.0) } else { (23.5, 19.0) };
    line(&mut img, (17.0, 16.0), hand, 3.4, shade(r.robe, 0.85));
    disc(&mut img, hand.0, hand.1, 1.4, SKIN);
    sculpt(&mut img, 12.0, 8.0);
    outline(&mut img);
    img
}

/// Imp: a crimson little devil with bat wings, horns and a barbed tail.
fn imp(frame: i32) -> Image {
    let mut img = cell();
    let red = rgb(0xc43a2a);
    let deep = rgb(0x6e1712);
    let up = frame == 0;
    // wings behind
    let wy = if up { 6.0 } else { 14.0 };
    poly(&mut img, &[(13.0, 16.0), (3.0, wy), (5.0, wy + 6.0), (2.0, wy + 9.0), (10.0, 20.0)], deep);
    poly(&mut img, &[(19.0, 16.0), (29.0, wy), (27.0, wy + 6.0), (30.0, wy + 9.0), (22.0, 20.0)], deep);
    // tail
    line(&mut img, (20.0, 24.0), (27.0, 26.0 - frame as f32), 1.4, red);
    poly(&mut img, &[(26.0, 24.0 - frame as f32), (30.0, 26.0 - frame as f32), (26.0, 28.0 - frame as f32)], red);
    // body and head
    ellipse(&mut img, 16.0, 22.0, 6.0, 5.5, red);
    disc(&mut img, 16.0, 14.0, 6.0, red);
    // horns
    poly(&mut img, &[(11.0, 11.0), (8.0, 4.0), (13.5, 9.0)], rgb(0xe8d9b0));
    poly(&mut img, &[(21.0, 11.0), (24.0, 4.0), (18.5, 9.0)], rgb(0xe8d9b0));
    // glowing eyes and a grin
    rect(&mut img, 12, 13, 3, 2, rgb(0xffe066));
    rect(&mut img, 17, 13, 3, 2, rgb(0xffe066));
    rect(&mut img, 13, 13, 1, 2, INK);
    rect(&mut img, 18, 13, 1, 2, INK);
    line(&mut img, (12.5, 17.5), (19.5, 17.5), 1.0, deep);
    put(&mut img, 13, 18, rgb(0xf4f0e0));
    put(&mut img, 18, 18, rgb(0xf4f0e0));
    // feet and claws
    rect(&mut img, 12, 27, 3, 2, deep);
    rect(&mut img, 17, 27, 3, 2, deep);
    sculpt(&mut img, 11.0, 10.0);
    outline(&mut img);
    img
}

/// Wraith: a hooded shade in tattered grey robes, eyes burning in the hood.
fn wraith(frame: i32) -> Image {
    let mut img = cell();
    let cloth = rgba(0xb9c3d6, 235);
    let fold = rgba(0x6e7890, 235);
    // tattered hem: a zigzag that ripples with the frame
    let mut pts = vec![(9.0, 9.0), (23.0, 9.0), (26.0, 24.0)];
    for k in 0..6 {
        let x = 26.0 - k as f32 * 3.4;
        let y = if (k + frame) % 2 == 0 { 31.0 } else { 26.5 };
        pts.push((x, y));
    }
    pts.push((6.0, 24.0));
    poly(&mut img, &pts, cloth);
    line(&mut img, (13.0, 14.0), (10.0, 27.0), 1.5, fold);
    line(&mut img, (19.0, 14.0), (22.0, 27.0), 1.5, fold);
    // reaching arms of rag
    let a = frame as f32 * 1.5;
    line(&mut img, (9.0, 16.0), (3.0, 20.0 - a), 2.4, cloth);
    line(&mut img, (23.0, 16.0), (29.0, 20.0 - a), 2.4, cloth);
    poly(&mut img, &[(2.0, 19.0 - a), (4.0, 23.0 - a), (1.0, 23.0 - a)], fold);
    poly(&mut img, &[(30.0, 19.0 - a), (28.0, 23.0 - a), (31.0, 23.0 - a)], fold);
    // the hood and its darkness
    poly(&mut img, &[(9.0, 13.0), (16.0, 1.0), (23.0, 13.0), (21.0, 16.0), (11.0, 16.0)], cloth);
    ellipse(&mut img, 16.0, 11.0, 4.5, 4.0, rgb(0x0d0e14));
    rect(&mut img, 13, 10, 2, 2, rgb(0x8ff0ff));
    rect(&mut img, 17, 10, 2, 2, rgb(0x8ff0ff));
    sculpt(&mut img, 12.0, 4.0);
    outline(&mut img);
    img
}

/// Spellhound: a lean ash-grey hound with ember eyes and runes along its back.
/// Faces right; `frame` is the gait (0..2), `howl` lifts the head.
fn hound(frame: i32) -> Image {
    let mut img = cell();
    // ash-pale, so it reads on the dark flagstones
    let fur = rgb(0x9aa0a8);
    let dark = rgb(0x5a5f68);
    let g = frame as f32;
    // legs: a trotting gait
    let legs = [(9.0, 2.0 - g * 2.0), (12.0, -1.0 + g * 2.0), (20.0, 1.0 - g * 2.0), (23.0, -2.0 + g * 2.0)];
    for (i, &(x, sw)) in legs.iter().enumerate() {
        let c = if i % 2 == 0 { dark } else { fur };
        line(&mut img, (x, 20.0), (x + sw, 28.5), 2.2, c);
        rect(&mut img, (x + sw - 1.0) as i32, 28, 3, 2, dark);
    }
    // tail
    line(&mut img, (7.0, 16.0), (2.0, 11.0 + g), 2.0, fur);
    // body
    ellipse(&mut img, 15.0, 18.0, 9.0, 4.8, fur);
    ellipse(&mut img, 15.0, 21.0, 7.0, 2.0, dark); // belly shadow
    // neck and head
    poly(&mut img, &[(20.0, 14.0), (25.0, 10.0), (27.0, 16.0), (22.0, 20.0)], fur);
    ellipse(&mut img, 25.0, 12.5, 3.6, 3.0, fur);
    poly(&mut img, &[(26.0, 11.0), (31.0, 13.0), (30.5, 15.0), (26.0, 15.5)], fur); // snout
    put(&mut img, 31, 13, INK);
    poly(&mut img, &[(22.5, 10.5), (23.5, 5.0), (25.5, 9.5)], dark); // ear
    rect(&mut img, 26, 11, 2, 2, rgb(0xff8a2a)); // ember eye
    // embers smoulder in cracks along the neck
    line(&mut img, (21.0, 15.0), (24.0, 12.0), 1.0, rgb(0xff7a2a));
    line(&mut img, (19.0, 17.5), (21.5, 15.5), 1.0, rgb(0xffa040));
    // glowing runes along the spine
    for k in 0..3 {
        let x = 10 + k * 4;
        rect(&mut img, x, 14, 2, 1, rgb(0xffb347));
        put(&mut img, x + 1, 13, rgb(0xffb347));
    }
    sculpt(&mut img, 14.0, 8.0);
    outline(&mut img);
    img
}

/// Gargoyle: a crouched stone beast, facing the viewer. `spread` lifts the
/// wings (its firing tell); `frame` breathes.
fn gargoyle(frame: i32, spread: bool) -> Image {
    let mut img = cell();
    let stone = rgb(0x6f7a6c);
    let deep = rgb(0x3c443c);
    let b = frame as f32;
    // wings
    if spread {
        poly(&mut img, &[(11.0, 14.0), (1.0, 2.0), (2.0, 10.0), (0.0, 15.0), (4.0, 17.0), (9.0, 21.0)], deep);
        poly(&mut img, &[(21.0, 14.0), (31.0, 2.0), (30.0, 10.0), (32.0, 15.0), (28.0, 17.0), (23.0, 21.0)], deep);
        line(&mut img, (11.0, 14.0), (2.0, 4.0), 1.0, stone);
        line(&mut img, (21.0, 14.0), (30.0, 4.0), 1.0, stone);
    } else {
        poly(&mut img, &[(10.0, 12.0), (5.0, 8.0 + b), (4.0, 20.0), (9.0, 26.0)], deep);
        poly(&mut img, &[(22.0, 12.0), (27.0, 8.0 + b), (28.0, 20.0), (23.0, 26.0)], deep);
    }
    // crouched body and haunches
    ellipse(&mut img, 16.0, 21.0, 7.0, 6.5, stone);
    ellipse(&mut img, 10.5, 26.0, 3.5, 3.0, stone);
    ellipse(&mut img, 21.5, 26.0, 3.5, 3.0, stone);
    rect(&mut img, 8, 28, 4, 2, deep);
    rect(&mut img, 20, 28, 4, 2, deep);
    // head with horns and a glowing maw
    ellipse(&mut img, 16.0, 12.0 + b * 0.5, 5.5, 4.8, stone);
    poly(&mut img, &[(12.0, 9.0), (9.0, 2.0), (14.0, 7.0)], deep);
    poly(&mut img, &[(20.0, 9.0), (23.0, 2.0), (18.0, 7.0)], deep);
    let maw = if spread { rgb(0xffd060) } else { rgb(0xff7a20) };
    rect(&mut img, 13, 11, 2, 2, maw);
    rect(&mut img, 17, 11, 2, 2, maw);
    rect(&mut img, 14, 15, 4, if spread { 2 } else { 1 }, maw);
    // moss and cracks
    for k in 0..12 {
        let x = 10 + (hash(k, 1, 4) * 12.0) as i32;
        let y = 16 + (hash(k, 2, 4) * 10.0) as i32;
        put(&mut img, x, y, rgb(0x4d6b3c));
    }
    line(&mut img, (18.0, 18.0), (20.0, 23.0), 1.0, deep);
    sculpt(&mut img, 12.0, 8.0);
    outline(&mut img);
    img
}

/// The Warlock: a towering grey-cowled sorcerer with violet eyes and clawed
/// hands spread to cast.
fn warlock(frame: i32, cast: bool) -> Image {
    let mut img = cell();
    let robe = rgb(0x4b4458);
    let cowl = rgb(0x8f8a9c);
    let violet = rgb(0xd68cff);
    let f = frame as f32;
    // great robe sweeping to the floor
    poly(&mut img, &[(11.0, 10.0), (21.0, 10.0), (27.0, 31.0), (5.0, 31.0)], robe);
    line(&mut img, (16.0, 14.0), (16.0, 30.0), 1.2, shade(robe, 0.6));
    line(&mut img, (12.0, 16.0), (8.0, 30.0), 1.2, shade(robe, 0.7));
    line(&mut img, (20.0, 16.0), (24.0, 30.0), 1.2, shade(robe, 0.7));
    // sigil trim at the hem
    for k in 0..6 {
        rect(&mut img, 7 + k * 3, 29, 2, 1, rgb(0xa77bd0));
    }
    // arms: raised for a spell, else held wide
    let (ly, ry) = if cast { (3.0, 3.0) } else { (14.0 + f, 15.0 - f) };
    line(&mut img, (11.0, 13.0), (3.0, ly), 3.2, cowl);
    line(&mut img, (21.0, 13.0), (29.0, ry), 3.2, cowl);
    for &(x, y, d) in &[(3.0, ly, -1.0), (29.0, ry, 1.0)] {
        line(&mut img, (x, y), (x + d * 2.0, y - 2.5), 1.0, rgb(0xd8d0c0));
        line(&mut img, (x, y), (x + d * 2.5, y + 0.5), 1.0, rgb(0xd8d0c0));
    }
    // cowl and the void inside it
    poly(&mut img, &[(10.0, 13.0), (13.0, 2.0), (16.0, 0.0), (19.0, 2.0), (22.0, 13.0), (16.0, 16.0)], cowl);
    ellipse(&mut img, 16.0, 8.5, 3.8, 4.0, rgb(0x0b0a10));
    rect(&mut img, 14, 8, 1, 1, violet);
    rect(&mut img, 17, 8, 1, 1, violet);
    put(&mut img, 14, 7, shade(violet, 0.6));
    put(&mut img, 17, 7, shade(violet, 0.6));
    sculpt(&mut img, 12.0, 4.0);
    outline(&mut img);
    img
}

/// A dragon whelp in flight, facing right: bronze scales, gold belly, wings
/// beating through three frames.
fn whelp(frame: i32) -> Image {
    let mut img = cell();
    let scale = rgb(0xa8642a);
    let dark = rgb(0x5a2c14);
    let belly = rgb(0xe3b65a);
    let wing_y = [3.0, 12.0, 22.0][frame as usize % 3];
    // far wing
    poly(&mut img, &[(13.0, 15.0), (8.0, wing_y + 2.0), (19.0, 14.0)], dark);
    // tail curling back
    line(&mut img, (9.0, 18.0), (2.0, 20.0), 2.4, scale);
    line(&mut img, (2.0, 20.0), (1.0, 16.0), 1.4, scale);
    poly(&mut img, &[(0.0, 14.0), (3.0, 16.0), (0.0, 18.0)], dark);
    // body
    ellipse(&mut img, 14.0, 18.0, 7.0, 4.2, scale);
    ellipse(&mut img, 15.0, 20.0, 5.0, 2.0, belly);
    // neck and head
    line(&mut img, (19.0, 16.0), (24.0, 11.0), 3.4, scale);
    ellipse(&mut img, 26.0, 10.5, 3.6, 2.6, scale);
    poly(&mut img, &[(27.0, 9.0), (31.5, 10.5), (28.0, 12.5)], scale);
    poly(&mut img, &[(24.0, 8.5), (21.0, 5.0), (25.5, 7.5)], belly); // horn
    rect(&mut img, 26, 9, 2, 1, rgb(0xfff0a0));
    // near wing, membrane over bone
    poly(&mut img, &[(15.0, 15.0), (20.0, wing_y), (26.0, wing_y + 3.0), (19.0, 17.0)], scale);
    line(&mut img, (15.0, 15.0), (20.0, wing_y), 1.0, belly);
    // legs tucked
    rect(&mut img, 12, 22, 2, 2, dark);
    rect(&mut img, 17, 22, 2, 2, dark);
    sculpt(&mut img, 14.0, 8.0);
    outline(&mut img);
    img
}

/// A cursed rune stone: a standing slab with a violet glyph (or gold, cleansed).
fn rune_stone(clean: bool) -> Image {
    let mut img = cell();
    let slab = if clean { rgb(0x6d6a64) } else { rgb(0x4a4652) };
    poly(&mut img, &[(9.0, 30.0), (10.0, 8.0), (16.0, 3.0), (22.0, 8.0), (23.0, 30.0)], slab);
    line(&mut img, (10.5, 9.0), (16.0, 4.0), 1.0, shade(slab, 1.4));
    let glow = if clean { rgb(0xffd66b) } else { rgb(0xb65cff) };
    // the glyph: a vertical stave with crossing branches
    line(&mut img, (16.0, 9.0), (16.0, 25.0), 1.4, glow);
    line(&mut img, (16.0, 12.0), (12.5, 16.0), 1.2, glow);
    line(&mut img, (16.0, 12.0), (19.5, 16.0), 1.2, glow);
    line(&mut img, (16.0, 19.0), (12.5, 23.0), 1.2, glow);
    line(&mut img, (16.0, 19.0), (19.5, 23.0), 1.2, glow);
    sculpt(&mut img, 12.0, 6.0);
    // glyphs stay at full brightness after the shading
    for (x, y) in [(16, 10), (16, 16), (16, 22)] {
        put(&mut img, x, y, glow);
    }
    outline(&mut img);
    img
}

/// The exit: a stair well down to the next cellar, behind an iron grate when shut.
fn stairs(open: bool) -> Image {
    let mut img = cell();
    rect(&mut img, 3, 3, 26, 26, rgb(0x050507));
    for k in 0..5 {
        let y = 5 + k * 5;
        let tone = 0.95 - k as f32 * 0.17;
        rect(&mut img, 4, y, 24, 4, shade(rgb(0x5a5650), tone));
        rect(&mut img, 4, y, 24, 1, shade(rgb(0x7a756c), tone));
    }
    // stone frame
    for (x, y, w, h) in [(1, 1, 30, 3), (1, 28, 30, 3), (1, 1, 3, 30), (28, 1, 3, 30)] {
        rect(&mut img, x, y, w, h, rgb(0x3b3f48));
    }
    rect(&mut img, 1, 1, 30, 1, rgb(0x6a6e78));
    if !open {
        for k in 0..6 {
            rect(&mut img, 5 + k * 4, 3, 2, 26, rgb(0x2a2c32));
            rect(&mut img, 5 + k * 4, 3, 1, 26, rgb(0x5b5f68));
        }
        rect(&mut img, 3, 15, 26, 2, rgb(0x2a2c32));
    }
    outline(&mut img);
    img
}

/// A rune gate sigil carved into the floor.
fn gate_sigil() -> Image {
    let mut img = cell();
    let c = rgb(0x6fd6ff);
    for k in 0..12 {
        let a = k as f32 / 12.0 * std::f32::consts::TAU;
        let (x, y) = (16.0 + a.cos() * 12.0, 16.0 + a.sin() * 12.0);
        disc(&mut img, x, y, 1.0, c);
    }
    // a three-armed spiral at the centre
    for arm in 0..3 {
        let base = arm as f32 / 3.0 * std::f32::consts::TAU;
        for k in 0..14 {
            let f = k as f32 / 14.0;
            let a = base + f * 3.2;
            let r = 2.0 + f * 7.5;
            disc(&mut img, 16.0 + a.cos() * r, 16.0 + a.sin() * r, 0.9, c);
        }
    }
    img
}

/// A cluster of tallow candles on a puddle of old wax (flames are runtime).
fn candles() -> Image {
    let mut img = cell();
    let wax = rgb(0xd8cdb0);
    ellipse(&mut img, 16.0, 27.0, 11.0, 3.5, rgb(0xb8ab8c));
    for &(x, top, w) in &[(9.0f32, 17.0f32, 3.0f32), (14.0, 11.0, 3.5), (19.0, 15.0, 3.0), (23.0, 20.0, 2.6), (12.0, 21.0, 2.4)] {
        rect(&mut img, (x - w / 2.0) as i32, top as i32, w as i32 + 1, (27.0 - top) as i32, wax);
        rect(&mut img, (x - w / 2.0) as i32, top as i32, 1, (27.0 - top) as i32, shade(wax, 1.12));
        // a drip
        rect(&mut img, (x + w / 2.0) as i32, (top + 2.0) as i32, 1, 3, shade(wax, 0.85));
        put(&mut img, x as i32, top as i32 - 1, rgb(0x2a1d14)); // wick
    }
    sculpt(&mut img, 10.0, 10.0);
    outline(&mut img);
    img
}

/// A crimson banner with a gold sigil of our own: a flame over a crown.
fn banner() -> Image {
    let mut img = cell();
    let red = rgb(0x7d1a17);
    let gold = rgb(0xd8b25a);
    rect(&mut img, 7, 1, 18, 2, rgb(0x2a2c32)); // the rod
    poly(&mut img, &[(9.0, 3.0), (23.0, 3.0), (23.0, 27.0), (16.0, 31.0), (9.0, 27.0)], red);
    line(&mut img, (10.0, 4.0), (10.0, 27.0), 1.0, shade(red, 1.3));
    line(&mut img, (22.0, 4.0), (22.0, 27.0), 1.0, shade(red, 0.6));
    // flame
    poly(&mut img, &[(16.0, 7.0), (19.5, 13.0), (18.0, 17.0), (16.0, 15.0), (14.0, 17.0), (12.5, 13.0)], gold);
    // crown
    poly(&mut img, &[(12.0, 23.0), (12.0, 18.5), (14.0, 21.0), (16.0, 18.0), (18.0, 21.0), (20.0, 18.5), (20.0, 23.0)], gold);
    line(&mut img, (9.5, 27.0), (16.0, 30.5), 1.0, gold);
    line(&mut img, (16.0, 30.5), (22.5, 27.0), 1.0, gold);
    outline(&mut img);
    img
}

/// A great dragon skull, two cells wide, facing right: the bones of some
/// old beast left to the cellar.
fn dragon_skull() -> Image {
    let mut img = Image::new(64, 32);
    let bone = rgb(0xcfc3a4);
    let dark = rgb(0x6d6450);
    // horns sweeping back
    poly(&mut img, &[(20.0, 12.0), (2.0, 4.0), (8.0, 9.0), (18.0, 16.0)], shade(bone, 0.8));
    poly(&mut img, &[(22.0, 18.0), (4.0, 22.0), (10.0, 24.0), (22.0, 22.0)], shade(bone, 0.7));
    // cranium and long snout
    ellipse(&mut img, 26.0, 16.0, 10.0, 8.0, bone);
    poly(&mut img, &[(30.0, 9.0), (58.0, 13.0), (62.0, 17.0), (58.0, 20.0), (30.0, 22.0)], bone);
    // lower jaw, a little agape
    poly(&mut img, &[(24.0, 22.0), (56.0, 22.0), (59.0, 26.0), (30.0, 27.0)], shade(bone, 0.85));
    // teeth
    for k in 0..7 {
        let x = 34 + k * 3;
        put(&mut img, x, 21, rgb(0xf1ead6));
        put(&mut img, x, 22, rgb(0xf1ead6));
    }
    // eye socket, nostril, cheek ridge
    ellipse(&mut img, 29.0, 13.5, 4.0, 3.0, rgb(0x0c0b0a));
    ellipse(&mut img, 55.0, 14.5, 1.6, 1.0, rgb(0x0c0b0a));
    line(&mut img, (20.0, 18.0), (44.0, 18.0), 1.0, dark);
    line(&mut img, (36.0, 11.0), (52.0, 13.0), 1.0, shade(bone, 1.15));
    sculpt(&mut img, 24.0, 8.0);
    outline(&mut img);
    img
}

/// A wall sconce: an iron bracket holding a pitch torch (the flame is runtime).
fn sconce() -> Image {
    let mut img = cell();
    line(&mut img, (16.0, 26.0), (16.0, 18.0), 2.0, rgb(0x2a2c32));
    poly(&mut img, &[(12.0, 14.0), (20.0, 14.0), (18.0, 19.0), (14.0, 19.0)], rgb(0x3c3f46));
    rect(&mut img, 13, 12, 6, 3, rgb(0x4a3020));
    rect(&mut img, 12, 14, 8, 1, rgb(0x6a6e78));
    outline(&mut img);
    img
}

/// A wizard's life marker for the HUD: a small pointed hat.
fn hat_icon(r: Robes) -> Image {
    let mut img = cell();
    ellipse(&mut img, 16.0, 24.0, 12.0, 3.5, shade(r.robe, 0.62));
    poly(&mut img, &[(8.0, 24.0), (24.0, 24.0), (19.0, 12.0), (25.0, 3.0), (14.0, 10.0)], r.robe);
    line(&mut img, (9.0, 22.5), (23.0, 22.5), 2.0, r.trim);
    disc(&mut img, 25.0, 3.0, 1.6, r.trim);
    sculpt(&mut img, 12.0, 8.0);
    outline(&mut img);
    img
}

/// The character sheet, `CELL`-square frames:
/// row 0, 1 — wizard one, two: front 0..2, back 0..2, side 0..2, cast front, cast side
///   (frames: 0 stand, 1 left step, 2 right step; side frames face right)
/// row 2 — imp: wings up, down; wraith: 0, 1; hound: gait 0..2; spare
/// row 3 — gargoyle: rest 0, 1, spread 0, 1; warlock: 0, 1, cast; spare
/// row 4 — whelp: 0..2; rune cursed, rune clean; stairs shut, open; gate
/// row 5 — sconce, hat one, hat two, candles, banner, dragon skull (two cells)
/// row 7 — the dragon skull facing left (two cells)
fn sprites() -> Vec<u8> {
    let mut sheet = Image::new(CELL * SHEET_COLS, CELL * SHEET_ROWS);
    let at = |sheet: &mut Image, col: u32, row: u32, img: &Image| stamp(sheet, img, (col * CELL) as i32, (row * CELL) as i32);
    for (row, r) in [P1, P2].iter().enumerate() {
        let row = row as u32;
        // three cells each: stand, step one way, step the other
        let fronts = [wizard_front(*r, 0, false, false), wizard_front(*r, -1, false, false), wizard_front(*r, 1, false, false)];
        // front and back share columns 0..2 and 3..5 would need 9; pack as:
        // 0 front stand, 1 front step, 2 back stand, 3 back step, 4 side stand,
        // 5 side step, 6 cast front, 7 cast side. Steps mirror for the other foot.
        at(&mut sheet, 0, row, &fronts[0]);
        at(&mut sheet, 1, row, &fronts[1]);
        at(&mut sheet, 2, row, &wizard_front(*r, 0, true, false));
        at(&mut sheet, 3, row, &wizard_front(*r, -1, true, false));
        at(&mut sheet, 4, row, &wizard_side(*r, 0, false));
        at(&mut sheet, 5, row, &wizard_side(*r, 1, false));
        at(&mut sheet, 6, row, &wizard_front(*r, 0, false, true));
        at(&mut sheet, 7, row, &wizard_side(*r, 0, true));
    }
    at(&mut sheet, 0, 2, &imp(0));
    at(&mut sheet, 1, 2, &imp(1));
    at(&mut sheet, 2, 2, &wraith(0));
    at(&mut sheet, 3, 2, &wraith(1));
    at(&mut sheet, 4, 2, &hound(0));
    at(&mut sheet, 5, 2, &hound(1));
    at(&mut sheet, 6, 2, &hound(2));
    at(&mut sheet, 0, 3, &gargoyle(0, false));
    at(&mut sheet, 1, 3, &gargoyle(1, false));
    at(&mut sheet, 2, 3, &gargoyle(0, true));
    at(&mut sheet, 3, 3, &gargoyle(1, true));
    at(&mut sheet, 4, 3, &warlock(0, false));
    at(&mut sheet, 5, 3, &warlock(1, false));
    at(&mut sheet, 6, 3, &warlock(0, true));
    at(&mut sheet, 0, 4, &whelp(0));
    at(&mut sheet, 1, 4, &whelp(1));
    at(&mut sheet, 2, 4, &whelp(2));
    at(&mut sheet, 3, 4, &rune_stone(false));
    at(&mut sheet, 4, 4, &rune_stone(true));
    at(&mut sheet, 5, 4, &stairs(false));
    at(&mut sheet, 6, 4, &stairs(true));
    at(&mut sheet, 7, 4, &gate_sigil());
    at(&mut sheet, 0, 5, &sconce());
    at(&mut sheet, 1, 5, &hat_icon(P1));
    at(&mut sheet, 2, 5, &hat_icon(P2));
    at(&mut sheet, 3, 5, &candles());
    at(&mut sheet, 4, 5, &banner());
    stamp(&mut sheet, &dragon_skull(), (5 * CELL) as i32, (5 * CELL) as i32);
    stamp(&mut sheet, &mirror(&dragon_skull()), 0, (7 * CELL) as i32);
    // mirrored side views (facing left) on row 6, so the BASIC tape can use them directly
    for (col, src) in [(0, wizard_side(P1, 0, false)), (1, wizard_side(P1, 1, false)), (2, wizard_side(P2, 0, false)), (3, wizard_side(P2, 1, false)), (4, hound(0)), (5, hound(1)), (6, hound(2)), (7, whelp(1))] {
        at(&mut sheet, col, 6, &mirror(&src));
    }
    // row 7, 8: spare for future frames
    sheet.encode_png()
}

#[path = "title.rs"]
mod title;

pub fn generate() -> Vec<Asset> {
    vec![
        ("images/floor.png", flagstones(15 * CELL, 13 * CELL, 7).encode_png()),
        ("images/title.png", title::title_art()),
        ("images/tiles.png", tiles()),
        ("images/sprites.png", sprites()),
    ]
}
