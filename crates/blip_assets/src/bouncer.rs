//! Bouncer (Breakout) assets: sprites, glockenspiel music (see `song`), and
//! effects from a toy box of rubber boings and glockenspiel (see `cosy`).

use std::f32::consts::PI;

use crate::image::Image;
use crate::cosy::{self, Tone, Voice, H};
use crate::song::{major, minor, Chord, Groove, Song, R};
use crate::Asset;

/// Bat cap width in source pixels; the game stretches only the slice
/// between the caps, so the ends keep their shape at any bat width.
pub const PADDLE_CAP: u32 = 12;

/// Brushed-steel bar (drawn 2x) with a glossy blue top face and dark rubber
/// bumpers at both ends.
fn paddle() -> Vec<u8> {
    let (w, h) = (120i32, 24i32);
    let mut img = Image::new(w as u32, h as u32);
    let r = 9.0f32;
    let cap = PADDLE_CAP as f32;
    const N: i32 = 3;
    for py in 0..h {
        for px in 0..w {
            let (mut cr, mut cg, mut cb, mut cov) = (0.0f32, 0.0f32, 0.0f32, 0.0f32);
            for sy in 0..N {
                for sx in 0..N {
                    let x = px as f32 + (sx as f32 + 0.5) / N as f32;
                    let y = py as f32 + (sy as f32 + 0.5) / N as f32;
                    // Rounded-rectangle distance (negative inside).
                    let qx = (x - w as f32 / 2.0).abs() - (w as f32 / 2.0 - r);
                    let qy = (y - h as f32 / 2.0).abs() - (h as f32 / 2.0 - r);
                    let d = qx.max(0.0).hypot(qy.max(0.0)) + qx.max(qy).min(0.0) - r;
                    if d > 0.0 { continue; }
                    let t = y / h as f32;
                    let end = x < cap || x > w as f32 - cap;
                    let mut c = if end {
                        // Rubber: near-black with a soft top sheen.
                        let v = 0.16 - 0.08 * t + 0.14 * (1.0 - (t * 3.0).min(1.0));
                        (v, v, v * 1.1)
                    } else if t < 0.5 {
                        // Lit top face: light steel-blue fading to mid blue.
                        let k = t / 0.5;
                        (0.62 - 0.4 * k, 0.86 - 0.36 * k, 1.0 - 0.18 * k)
                    } else {
                        // Rounded underside: darkening navy.
                        let k = (t - 0.5) / 0.5;
                        (0.22 - 0.17 * k, 0.5 - 0.36 * k, 0.82 - 0.5 * k)
                    };
                    // Specular line just under the top edge, on the body only.
                    if !end && (2.2..4.2).contains(&y) {
                        c = (c.0 + (1.0 - c.0) * 0.7, c.1 + (1.0 - c.1) * 0.7, c.2 + (1.0 - c.2) * 0.7);
                    }
                    // Seam where the bumper meets the bar.
                    if (x - cap).abs() < 0.9 || (x - (w as f32 - cap)).abs() < 0.9 {
                        c = (c.0 * 0.35, c.1 * 0.35, c.2 * 0.35);
                    }
                    if d > -1.3 { c = (0.05, 0.07, 0.12); }
                    cr += c.0; cg += c.1; cb += c.2; cov += 1.0;
                }
            }
            if cov > 0.0 {
                let k = |v: f32| (v / cov * 255.0).clamp(0.0, 255.0) as u8;
                img.set_rgba(px, py, k(cr), k(cg), k(cb), (cov / (N * N) as f32 * 255.0) as u8);
            }
        }
    }
    img.encode_png()
}

/// Ball sheet layout: columns step the yaw through one panel-pair period
/// (120°), rows step the pitch from -90° to +90°. The game applies the
/// third axis (roll) by rotating the sprite.
pub const BALL_YAW_N: u32 = 12;
pub const BALL_PITCH_N: u32 = 19;
const BALL_PX: u32 = 36;
const SS: i32 = 3;

/// Unit-sphere point under a view-space pixel, or None outside the disc.
fn sphere_point(px: i32, py: i32, sx: i32, sy: i32) -> Option<(f32, f32, f32)> {
    let n = (BALL_PX as i32 * SS) as f32;
    let x = ((px * SS + sx) as f32 + 0.5) / n * 2.0 - 1.0;
    let y = ((py * SS + sy) as f32 + 0.5) / n * 2.0 - 1.0;
    let d2 = x * x + y * y;
    if d2 >= 1.0 { None } else { Some((x, y, (1.0 - d2).sqrt())) }
}

/// Six red/white panels with white pole caps, unlit. Frame (col, row) shows
/// the ball at yaw `col`, pitch `row`; lighting lives in `ball_shade`.
fn ball() -> Vec<u8> {
    let n = BALL_PX as i32;
    let mut img = Image::new(BALL_PX * BALL_YAW_N, BALL_PX * BALL_PITCH_N);
    for row in 0..BALL_PITCH_N as i32 {
        let beta = -PI / 2.0 + row as f32 * PI / (BALL_PITCH_N - 1) as f32;
        let (sb, cb) = beta.sin_cos();
        for col in 0..BALL_YAW_N as i32 {
            let alpha = col as f32 * (2.0 * PI / 3.0) / BALL_YAW_N as f32;
            let (sa, ca) = alpha.sin_cos();
            for py in 0..n {
                for px in 0..n {
                    let (mut r, mut g, mut b, mut cov) = (0.0f32, 0.0f32, 0.0f32, 0.0f32);
                    for sy in 0..SS {
                        for sx in 0..SS {
                            let Some((x, y, z)) = sphere_point(px, py, sx, sy) else { continue };
                            // Into pattern space: Ry(-alpha) * Rx(-beta).
                            let (y1, z1) = (y * cb + z * sb, -y * sb + z * cb);
                            let (x2, z2) = (x * ca - z1 * sa, x * sa + z1 * ca);
                            let t = (x2.atan2(z2) + PI) / (PI / 3.0);
                            let cap = y1.abs() > 0.88;
                            let seam = t.fract();
                            let (mut cr, mut cg, mut cb_) = if cap || t.floor() as i32 % 2 == 0 {
                                (0.97, 0.96, 0.92)
                            } else {
                                (0.9, 0.17, 0.13)
                            };
                            if !cap && (seam < 0.025 || seam > 0.975) {
                                cr *= 0.7; cg *= 0.7; cb_ *= 0.7;
                            }
                            r += cr; g += cg; b += cb_; cov += 1.0;
                        }
                    }
                    if cov > 0.0 {
                        let k = |v: f32| (v / cov * 255.0) as u8;
                        let a = (cov / (SS * SS) as f32 * 255.0) as u8;
                        img.set_rgba(col * n + px, row * n + py, k(r), k(g), k(b), a);
                    }
                }
            }
        }
    }
    img.encode_png()
}

/// Fixed lighting overlay (shadow + specular) drawn over the rolling pattern.
fn ball_shade() -> Vec<u8> {
    let n = BALL_PX as i32;
    let mut img = Image::new(BALL_PX, BALL_PX);
    let light = { let (x, y, z) = (-0.5f32, -0.6, 0.62); let m = (x * x + y * y + z * z).sqrt(); (x / m, y / m, z / m) };
    let half = { let (x, y, z) = (light.0, light.1, light.2 + 1.0); let m = (x * x + y * y + z * z).sqrt(); (x / m, y / m, z / m) };
    for py in 0..n {
        for px in 0..n {
            let (mut a_sum, mut c_sum, mut cov) = (0.0f32, 0.0f32, 0.0f32);
            for sy in 0..SS {
                for sx in 0..SS {
                    let Some((x, y, z)) = sphere_point(px, py, sx, sy) else { continue };
                    let diff = (x * light.0 + y * light.1 + z * light.2).max(0.0);
                    let spec = ((x * half.0 + y * half.1 + z * half.2).max(0.0).powf(40.0) * 0.85).min(1.0);
                    let lit = (0.32 + 0.78 * diff - (1.0 - z).powf(2.0) * 0.25).clamp(0.0, 1.0);
                    // Black at (1-lit), then white at `spec` over it, as one grey layer.
                    let a = 1.0 - lit * (1.0 - spec);
                    a_sum += a;
                    c_sum += spec; // colour * alpha
                    cov += 1.0;
                }
            }
            if cov > 0.0 {
                let a = a_sum / cov;
                let c = if a > 0.0 { (c_sum / cov / a).min(1.0) } else { 0.0 };
                let v = (c * 255.0) as u8;
                img.set_rgba(px, py, v, v, v, (a * cov / (SS * SS) as f32 * 255.0) as u8);
            }
        }
    }
    img.encode_png()
}

fn brick(color: (u8, u8, u8)) -> Vec<u8> {
    let w: i32 = 72;
    let h: i32 = 22;
    let mut img = Image::new(w as u32, h as u32);
    let (br, bg, bb) = color;
    for y in 1..h - 1 {
        for x in 1..w - 1 {
            let mut shade = 1.0_f32;
            if y < 3 { shade = 1.3; }
            if y > h - 4 { shade = 0.6; }
            if x < 2 { shade *= 1.2; }
            if x > w - 3 { shade *= 0.7; }
            let r = ((br as f32 * shade).min(255.0)) as u8;
            let g = ((bg as f32 * shade).min(255.0)) as u8;
            let b = ((bb as f32 * shade).min(255.0)) as u8;
            img.set(x, y, r, g, b);
        }
    }
    for x in 0..w {
        img.set(x, 0, 20, 20, 20);
        img.set(x, h - 1, 20, 20, 20);
    }
    for y in 0..h {
        img.set(0, y, 20, 20, 20);
        img.set(w - 1, y, 20, 20, 20);
    }
    img.encode_png()
}

/// Falling pickups: glossy capsules, drawn at 56x32 and shown at half size.
/// Good ones are smooth and cool/bright with a plain icon; the bad one is
/// red with hazard stripes, so it reads by pattern as well as by colour.
#[derive(Copy, Clone, PartialEq)]
enum Pickup { Wide, Narrow, Slow, Life, Multi }

fn in_tri(p: (f32, f32), a: (f32, f32), b: (f32, f32), c: (f32, f32)) -> bool {
    let s = |p: (f32, f32), q: (f32, f32), r: (f32, f32)| (p.0 - r.0) * (q.1 - r.1) - (q.0 - r.0) * (p.1 - r.1);
    let (d1, d2, d3) = (s(p, a, b), s(p, b, c), s(p, c, a));
    !((d1 < 0.0 || d2 < 0.0 || d3 < 0.0) && (d1 > 0.0 || d2 > 0.0 || d3 > 0.0))
}

fn in_bar(p: (f32, f32), x0: f32, x1: f32, half: f32) -> bool {
    p.0 >= x0 && p.0 <= x1 && p.1.abs() <= half
}

/// Icon coverage at `p`, relative to the capsule centre (y down).
fn pickup_icon(kind: Pickup, p: (f32, f32)) -> bool {
    // Arrow along x from `tail` to `tip`: 3-thick shaft, 6-long 9-tall head.
    let arrow = |tail: f32, tip: f32, p: (f32, f32)| {
        let head = tip - (tip - tail).signum() * 6.0;
        in_bar(p, tail.min(head), tail.max(head), 1.5)
            || in_tri(p, (tip, 0.0), (head, -4.5), (head, 4.5))
    };
    match kind {
        Pickup::Wide => arrow(0.0, -13.0, p) || arrow(0.0, 13.0, p),
        Pickup::Narrow => arrow(-14.0, -2.5, p) || arrow(14.0, 2.5, p),
        Pickup::Slow => {
            let r = p.0.hypot(p.1);
            (7.6..=10.0).contains(&r)
                || (p.0.abs() <= 1.3 && (-7.0..=1.0).contains(&p.1))
                || (p.1.abs() <= 1.3 && (-1.0..=5.0).contains(&p.0))
        }
        Pickup::Life => {
            let (x, y) = (p.0 / 9.5, -(p.1 + 1.0) / 9.5);
            let q = x * x + y * y - 1.0;
            q * q * q - x * x * y * y * y <= 0.0
        }
        // three balls in a row
        Pickup::Multi => [-12.0f32, 0.0, 12.0].iter().any(|&cx| (p.0 - cx).hypot(p.1) <= 4.6),
    }
}

fn pickup(kind: Pickup) -> Vec<u8> {
    let (w, h) = (56i32, 32i32);
    let mut img = Image::new(w as u32, h as u32);
    let r = h as f32 / 2.0 - 1.0;
    let (cx, cy) = (w as f32 / 2.0, h as f32 / 2.0);
    let (top, bot): ((f32, f32, f32), (f32, f32, f32)) = match kind {
        Pickup::Wide => ((0.45, 0.95, 0.5), (0.05, 0.5, 0.2)),
        Pickup::Slow => ((0.5, 0.78, 1.0), (0.05, 0.28, 0.75)),
        Pickup::Life => ((1.0, 0.9, 0.4), (0.85, 0.5, 0.0)),
        Pickup::Narrow => ((1.0, 0.4, 0.35), (0.6, 0.03, 0.05)),
        Pickup::Multi => ((0.85, 0.6, 1.0), (0.42, 0.12, 0.72)),
    };
    const N: i32 = 3;
    for py in 0..h {
        for px in 0..w {
            let (mut cr, mut cg, mut cb, mut cov) = (0.0f32, 0.0f32, 0.0f32, 0.0f32);
            for sy in 0..N {
                for sx in 0..N {
                    let x = px as f32 + (sx as f32 + 0.5) / N as f32;
                    let y = py as f32 + (sy as f32 + 0.5) / N as f32;
                    // Distance to the capsule's core segment.
                    let nx = x.clamp(r + 1.0, w as f32 - r - 1.0);
                    let d = (x - nx).hypot(y - cy) - r;
                    if d > 0.0 { continue; }
                    let t = ((y - (cy - r)) / (2.0 * r)).clamp(0.0, 1.0);
                    let mut c = (
                        top.0 + (bot.0 - top.0) * t,
                        top.1 + (bot.1 - top.1) * t,
                        top.2 + (bot.2 - top.2) * t,
                    );
                    if kind == Pickup::Narrow && ((x + y) / 7.0).floor() as i32 % 2 == 0 {
                        c = (c.0 * 0.28, c.1 * 0.28, c.2 * 0.28);
                    }
                    // Gloss strip along the top edge, fading toward the ends.
                    let inner = (x - cx).abs() / (w as f32 / 2.0 - r);
                    if (y - cy + r) > 3.0 && (y - cy + r) < 9.5 && inner < 1.0 {
                        let k = 0.4 * (1.0 - inner * inner);
                        c = (c.0 + (1.0 - c.0) * k, c.1 + (1.0 - c.1) * k, c.2 + (1.0 - c.2) * k);
                    }
                    if d > -1.8 { c = (c.0 * 0.35, c.1 * 0.35, c.2 * 0.35); }
                    let p = (x - cx, y - cy);
                    let sh = (p.0 - 1.4, p.1 - 1.6);
                    if pickup_icon(kind, sh) { c = (c.0 * 0.35, c.1 * 0.35, c.2 * 0.35); }
                    if pickup_icon(kind, p) {
                        c = if kind == Pickup::Life { (0.85, 0.08, 0.15) } else { (1.0, 1.0, 1.0) };
                    }
                    cr += c.0; cg += c.1; cb += c.2; cov += 1.0;
                }
            }
            if cov > 0.0 {
                let k = |v: f32| (v / cov * 255.0).min(255.0) as u8;
                img.set_rgba(px, py, k(cr), k(cg), k(cb), (cov / (N * N) as f32 * 255.0) as u8);
            }
        }
    }
    img.encode_png()
}

/// The bomb brick: a dark casing with a lit fuse-coloured burst on it, so
/// it reads as "this one goes off" among the plain colours.
fn brick_bomb() -> Vec<u8> {
    let (w, h): (i32, i32) = (72, 22);
    let mut img = Image::new(w as u32, h as u32);
    let (cx, cy) = (w as f32 / 2.0, h as f32 / 2.0);
    for y in 1..h - 1 {
        for x in 1..w - 1 {
            let mut shade = 1.0_f32;
            if y < 3 { shade = 1.4; }
            if y > h - 4 { shade = 0.6; }
            let (mut r, mut g, mut b) = (46.0 * shade, 42.0 * shade, 52.0 * shade);
            // an eight-pointed burst: orange, with a yellow heart
            let (dx, dy) = ((x as f32 + 0.5 - cx) / 1.5, y as f32 + 0.5 - cy);
            let d = dx.hypot(dy);
            let ang = dy.atan2(dx);
            let reach = 6.5 + 2.5 * (ang * 8.0).cos();
            if d < reach { (r, g, b) = (255.0, 140.0, 30.0); }
            if d < 3.4 { (r, g, b) = (255.0, 235.0, 120.0); }
            img.set(x, y, r.min(255.0) as u8, g.min(255.0) as u8, b.min(255.0) as u8);
        }
    }
    for x in 0..w {
        img.set(x, 0, 15, 15, 18);
        img.set(x, h - 1, 15, 15, 18);
    }
    for y in 0..h {
        img.set(0, y, 15, 15, 18);
        img.set(w - 1, y, 15, 15, 18);
    }
    img.encode_png()
}

/// Steel-plated brick: takes two hits to break. A cool, riveted metal tone
/// keeps it visually distinct from the six single-hit color rows, and the
/// `cracked` variant (shown after the first hit) darkens it and adds a
/// jagged fracture so the damage — and the fact one more hit will do it — is
/// obvious at a glance.
fn brick_steel(cracked: bool) -> Vec<u8> {
    let w: i32 = 72;
    let h: i32 = 22;
    let mut img = Image::new(w as u32, h as u32);
    let base: (u8, u8, u8) = if cracked { (108, 122, 136) } else { (150, 170, 190) };
    for y in 1..h - 1 {
        for x in 1..w - 1 {
            let mut shade = 1.0_f32;
            if y < 3 { shade = 1.3; }
            if y > h - 4 { shade = 0.6; }
            if x < 2 { shade *= 1.2; }
            if x > w - 3 { shade *= 0.7; }
            let r = ((base.0 as f32 * shade).min(255.0)) as u8;
            let g = ((base.1 as f32 * shade).min(255.0)) as u8;
            let b = ((base.2 as f32 * shade).min(255.0)) as u8;
            img.set(x, y, r, g, b);
        }
    }
    // corner rivets sell the "reinforced plate" look
    for &(rx, ry) in &[(6, 6), (w - 7, 6), (6, h - 7), (w - 7, h - 7)] {
        img.set(rx, ry, 55, 60, 65);
        img.set(rx + 1, ry, 225, 230, 235);
    }
    if cracked {
        let mut x = 5;
        let mut toggle = 0i32;
        while x < w - 5 {
            let y = (h / 2 + toggle) as i32;
            img.set(x, y, 18, 18, 22);
            img.set(x, (y + 1).min(h - 2), 18, 18, 22);
            toggle = if toggle <= 0 { 3 } else { -3 };
            x += 4;
        }
    }
    for x in 0..w {
        img.set(x, 0, 15, 15, 18);
        img.set(x, h - 1, 15, 15, 18);
    }
    for y in 0..h {
        img.set(0, y, 15, 15, 18);
        img.set(w - 1, y, 15, 15, 18);
    }
    img.encode_png()
}

// ---- music: two tunes on the glockenspiel -------------------------------
// Synthesised on the device (see `song`). Octave leaps for the ball's
// bounce; the game alternates the two by level.

const C: Chord = major(48);
const D: Chord = major(50);
const F: Chord = major(41);
const G: Chord = major(43);
const AM: Chord = minor(45);
const EM: Chord = minor(40);

/// Odd levels: C major, bouncing up the octave and skipping back down.
pub fn bounce_wav() -> Vec<u8> {
    Song {
        bpm: 124.0,
        melody: &[
            [72, 84, 79, H, 76, H, 79, H], [77, H, 76, H, 74, H, R, R],
            [71, 83, 79, H, 74, H, 77, H], [76, H, 74, H, 72, H, R, R],
            [72, 84, 79, H, 76, H, 79, 81], [84, H, 81, H, 77, H, 81, H],
            [79, H, 77, 76, 74, H, 71, H], [72, H, H, H, R, R, R, R],
            [81, H, 79, H, 77, H, 76, H], [74, H, 76, 77, 79, H, R, R],
            [79, H, 77, H, 76, H, 74, H], [72, H, 74, 76, 77, H, R, R],
            [76, 88, 84, H, 79, H, 84, H], [86, H, 84, H, 83, H, 79, H],
            [77, H, 76, H, 74, H, 79, H], [72, H, H, H, R, R, R, R],
        ],
        chords: &[
            [C, C], [F, G], [G, G], [C, C], [C, C], [F, F], [G, G], [C, C],
            [F, F], [G, G], [C, G], [AM, F], [C, C], [G, G], [F, G], [C, C],
        ],
        lead: Voice::Glock,
        lead_vol: 0.36,
        harmony: Some((Voice::Pluck, -12)),
        seed: 0xB0DE_1234,
        ..Song::DEFAULT
    }
    .render()
}

/// Even levels: G major, quicker, climbing in thirds.
pub fn rebound_wav() -> Vec<u8> {
    Song {
        bpm: 138.0,
        melody: &[
            [79, H, 83, 86, H, 83, 79, H], [81, H, 84, H, 83, H, 81, H],
            [79, H, 83, 86, H, 88, 86, H], [84, H, 83, H, 81, H, R, R],
            [76, H, 79, 83, H, 79, 76, H], [78, H, 81, H, 84, H, 83, 81],
            [79, H, 78, H, 76, H, 74, H], [79, H, H, H, R, R, R, R],
            [86, H, 84, 83, 81, H, 79, H], [84, H, 83, 81, 79, H, 78, H],
            [79, 81, 83, 84, 86, H, 83, H], [81, H, H, H, R, R, R, R],
            [86, H, 91, H, 88, H, 86, H], [84, H, 88, H, 86, H, 84, H],
            [83, H, 81, H, 78, H, 74, H], [79, H, H, H, R, R, R, R],
        ],
        chords: &[
            [G, G], [D, D], [G, G], [C, D], [EM, EM], [D, D], [C, D], [G, G],
            [G, D], [C, D], [G, G], [D, D], [G, C], [C, C], [D, D], [G, G],
        ],
        lead: Voice::Glock,
        lead_vol: 0.34,
        harmony: Some((Voice::Pluck, -12)),
        groove: Groove::FourFloor,
        seed: 0xB0DE_5678,
        ..Song::DEFAULT
    }
    .render()
}

// ---- effects: a toy box ----------------------------------------------------
// Bubbler's warm, rounded style on Bouncer's own instruments: a rubber ball
// that boings and a glockenspiel in C major. Three takes of each hit, a
// little apart in pitch, so rapid hits do not machine-gun.
pub const IMPACT_VARIANTS: usize = 3;

fn paddle_hit(v: usize) -> Vec<u8> {
    cosy::sfx(&cosy::boing(0.16, [196.0, 208.0, 220.0][v]))
}

/// A brick taking a hit without breaking: a muted glockenspiel tap.
fn brick_hit(v: usize) -> Vec<u8> {
    let mut tap = cosy::run(&[79 + 2 * v as i32], 0.05, Voice::Glock, 0.45);
    tap.truncate((0.25 * cosy::SR) as usize);
    cosy::sfx(&tap)
}

/// A brick breaking: a bright glockenspiel note on a soft pop.
fn brick_break(v: usize) -> Vec<u8> {
    let pop = cosy::snap(0.08, 1400.0 + 120.0 * v as f32, 0xB0 + v as u32);
    let bell = cosy::run(&[[84, 88, 91][v]], 0.05, Voice::Glock, 0.6);
    cosy::sfx(&cosy::mix(pop, &bell, 0.01, 1.0))
}

fn wall_hit() -> Vec<u8> {
    cosy::finish_warm(cosy::boing(0.1, 150.0), 11_000.0)
}

/// A ball lost: a rubbery sigh falling away.
fn life_lost() -> Vec<u8> {
    cosy::sfx(&cosy::glide(1.0, 520.0, 170.0, Tone::Sine, 0.8, 20.0))
}

fn win() -> Vec<u8> {
    cosy::jingle_with(&[72, 76, 79, 84, H, 79, 84, 88, H, H], 0.09, Voice::Glock, 0, Some((Voice::Bell, -1)))
}

/// Pickups caught: a good one sparkles up the glockenspiel, the narrow bat
/// tumbles down it, and a life is a little fanfare.
fn pickup_good() -> Vec<u8> {
    cosy::sfx(&cosy::run(&[84, 88, 91, 96], 0.045, Voice::Glock, 0.5))
}

fn pickup_bad() -> Vec<u8> {
    cosy::sfx(&cosy::run(&[88, 84, 79, 72], 0.06, Voice::Glock, 0.45))
}

fn pickup_life() -> Vec<u8> {
    cosy::jingle_with(&[79, 84, 88, 91, H, 96, H, H], 0.07, Voice::Glock, 0, Some((Voice::Bell, -1)))
}

/// A bomb brick going off: a soft pop over a low boom tumbling down. One
/// sound for the whole blast, not nine bricks at once.
fn bomb_sfx() -> Vec<u8> {
    let pop = cosy::snap(0.07, 700.0, 0xB0B1);
    cosy::sfx(&cosy::mix(pop, &cosy::bloop(0.42, 220.0, 55.0, 0xB0B2), 0.01, 1.0))
}

pub fn generate() -> Vec<Asset> {
    vec![
        ("images/paddle.png", paddle()),
        ("images/ball.png", ball()),
        ("images/drop_wide.png", pickup(Pickup::Wide)),
        ("images/drop_narrow.png", pickup(Pickup::Narrow)),
        ("images/drop_slow.png", pickup(Pickup::Slow)),
        ("images/drop_life.png", pickup(Pickup::Life)),
        ("images/drop_multi.png", pickup(Pickup::Multi)),
        ("images/ball_shade.png", ball_shade()),
        ("images/brick_red.png",    brick((220, 60, 60))),
        ("images/brick_orange.png", brick((220, 140, 40))),
        ("images/brick_yellow.png", brick((200, 200, 50))),
        ("images/brick_green.png",  brick((50,  200, 80))),
        ("images/brick_blue.png",   brick((50,  100, 220))),
        ("images/brick_purple.png", brick((160, 50,  220))),
        ("images/brick_steel.png",         brick_steel(false)),
        ("images/brick_steel_cracked.png", brick_steel(true)),
        ("images/brick_bomb.png", brick_bomb()),
        ("sounds/paddle_hit_0.wav", paddle_hit(0)),
        ("sounds/paddle_hit_1.wav", paddle_hit(1)),
        ("sounds/paddle_hit_2.wav", paddle_hit(2)),
        ("sounds/brick_hit_0.wav", brick_hit(0)),
        ("sounds/brick_hit_1.wav", brick_hit(1)),
        ("sounds/brick_hit_2.wav", brick_hit(2)),
        ("sounds/brick_break_0.wav", brick_break(0)),
        ("sounds/brick_break_1.wav", brick_break(1)),
        ("sounds/brick_break_2.wav", brick_break(2)),
        ("sounds/wall_hit.wav", wall_hit()),
        ("sounds/bomb.wav", bomb_sfx()),
        ("sounds/life_lost.wav",  life_lost()),
        ("sounds/win.wav",        win()),
        ("sounds/pickup_good.wav", pickup_good()),
        ("sounds/pickup_bad.wav",  pickup_bad()),
        ("sounds/pickup_life.wav", pickup_life()),
    ]
}
