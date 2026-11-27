use std::f32::consts::PI;

use blip::image::Image;
use blip::Asset;

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
    img.supersample(|x, y| {
        // Rounded-rectangle distance (negative inside).
        let qx = (x - w as f32 / 2.0).abs() - (w as f32 / 2.0 - r);
        let qy = (y - h as f32 / 2.0).abs() - (h as f32 / 2.0 - r);
        let d = qx.max(0.0).hypot(qy.max(0.0)) + qx.max(qy).min(0.0) - r;
        if d > 0.0 {
            return None;
        }
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
        if d > -1.3 {
            let ink = blip::look::INK;
            c = (ink[0] as f32 / 255.0, ink[1] as f32 / 255.0, ink[2] as f32 / 255.0);
        }
        Some(c)
    });
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
    if d2 >= 1.0 {
        None
    } else {
        Some((x, y, (1.0 - d2).sqrt()))
    }
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
                            let Some((x, y, z)) = sphere_point(px, py, sx, sy) else {
                                continue;
                            };
                            // Into pattern space: Ry(-alpha) * Rx(-beta).
                            let (y1, z1) = (y * cb + z * sb, -y * sb + z * cb);
                            let (x2, z2) = (x * ca - z1 * sa, x * sa + z1 * ca);
                            let t = (x2.atan2(z2) + PI) / (PI / 3.0);
                            let cap = y1.abs() > 0.88;
                            let seam = t.fract();
                            let (mut cr, mut cg, mut cb_) =
                                if cap || t.floor() as i32 % 2 == 0 { (0.97, 0.96, 0.92) } else { (0.9, 0.17, 0.13) };
                            if !cap && !(0.025..=0.975).contains(&seam) {
                                cr *= 0.7;
                                cg *= 0.7;
                                cb_ *= 0.7;
                            }
                            r += cr;
                            g += cg;
                            b += cb_;
                            cov += 1.0;
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
    let light = {
        let (x, y, z) = (-0.5f32, -0.6, 0.62);
        let m = (x * x + y * y + z * z).sqrt();
        (x / m, y / m, z / m)
    };
    let half = {
        let (x, y, z) = (light.0, light.1, light.2 + 1.0);
        let m = (x * x + y * y + z * z).sqrt();
        (x / m, y / m, z / m)
    };
    for py in 0..n {
        for px in 0..n {
            let (mut a_sum, mut c_sum, mut cov) = (0.0f32, 0.0f32, 0.0f32);
            for sy in 0..SS {
                for sx in 0..SS {
                    let Some((x, y, z)) = sphere_point(px, py, sx, sy) else {
                        continue;
                    };
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

/// Bricks are drawn at their size on screen, one image pixel a screen pixel, so they stay crisp.
const BRICK_W: i32 = 44;
const BRICK_H: i32 = 18;

/// A brick in the blip look: a rounded ink rim, a lit top and left edge, a shaded foot and a glint, round a face
/// `fill` paints (x, y inside the brick).
fn block(fill: impl Fn(i32, i32) -> (f32, f32, f32)) -> Image {
    let (w, h) = (BRICK_W, BRICK_H);
    let mut img = Image::new(w as u32, h as u32);
    let ink = blip::look::INK;
    for y in 0..h {
        for x in 0..w {
            let corner = (x == 0 || x == w - 1) && (y == 0 || y == h - 1);
            if corner { continue; }
            if x == 0 || y == 0 || x == w - 1 || y == h - 1 {
                img.set(x, y, ink[0], ink[1], ink[2]);
                continue;
            }
            let (r, g, b) = fill(x, y);
            let k = if y == 1 || x == 1 { 1.35 } else if y >= h - 3 { 0.68 } else if x == w - 2 { 0.82 } else { 1.0 };
            let lift = if y == 1 && x > 1 { 40.0 } else { 0.0 };
            let c = |v: f32| (v * k + lift).clamp(0.0, 255.0) as u8;
            img.set(x, y, c(r), c(g), c(b));
        }
    }
    let cream = blip::look::CREAM;
    for (x, y) in [(3, 3), (4, 3), (5, 3), (3, 4)] {
        img.set(x, y, cream[0], cream[1], cream[2]);
    }
    img
}

fn brick(color: [u8; 3]) -> Vec<u8> {
    let c = (color[0] as f32, color[1] as f32, color[2] as f32);
    block(|_, _| c).encode_png()
}

/// Falling pickups: glossy capsules, drawn at 56x32 and shown at half size.
/// Good ones are smooth and cool/bright with a plain icon; the bad one is
/// red with hazard stripes, so it reads by pattern as well as by colour.
#[derive(Copy, Clone, PartialEq)]
enum Pickup {
    Wide,
    Narrow,
    Slow,
    Life,
    Multi,
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
        ("images/brick_red.png", brick(blip::look::HEART)),
        ("images/brick_orange.png", brick([240, 138, 56])),
        ("images/brick_yellow.png", brick(blip::look::GOLD)),
        ("images/brick_green.png", brick(blip::look::LEAF)),
        ("images/brick_blue.png", brick(blip::look::SEA)),
        ("images/brick_purple.png", brick(blip::look::PLUM)),
        ("images/brick_steel.png", brick_steel(false)),
        ("images/brick_steel_cracked.png", brick_steel(true)),
        ("images/brick_bomb.png", brick_bomb()),
    ]
}

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
        in_bar(p, tail.min(head), tail.max(head), 1.5) || in_tri(p, (tip, 0.0), (head, -4.5), (head, 4.5))
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
    img.supersample(|x, y| {
        // Distance to the capsule's core segment.
        let nx = x.clamp(r + 1.0, w as f32 - r - 1.0);
        let d = (x - nx).hypot(y - cy) - r;
        if d > 0.0 {
            return None;
        }
        let t = ((y - (cy - r)) / (2.0 * r)).clamp(0.0, 1.0);
        let mut c = (top.0 + (bot.0 - top.0) * t, top.1 + (bot.1 - top.1) * t, top.2 + (bot.2 - top.2) * t);
        if kind == Pickup::Narrow && ((x + y) / 7.0).floor() as i32 % 2 == 0 {
            c = (c.0 * 0.28, c.1 * 0.28, c.2 * 0.28);
        }
        // Gloss strip along the top edge, fading toward the ends.
        let inner = (x - cx).abs() / (w as f32 / 2.0 - r);
        if (y - cy + r) > 3.0 && (y - cy + r) < 9.5 && inner < 1.0 {
            let k = 0.4 * (1.0 - inner * inner);
            c = (c.0 + (1.0 - c.0) * k, c.1 + (1.0 - c.1) * k, c.2 + (1.0 - c.2) * k);
        }
        if d > -1.8 {
            c = (c.0 * 0.35, c.1 * 0.35, c.2 * 0.35);
        }
        let p = (x - cx, y - cy);
        let sh = (p.0 - 1.4, p.1 - 1.6);
        if pickup_icon(kind, sh) {
            c = (c.0 * 0.35, c.1 * 0.35, c.2 * 0.35);
        }
        if pickup_icon(kind, p) {
            c = if kind == Pickup::Life { (0.85, 0.08, 0.15) } else { (1.0, 1.0, 1.0) };
        }
        Some(c)
    });
    img.encode_png()
}

/// The bomb brick: a dark casing with a lit fuse-coloured burst on it, so
/// it reads as "this one goes off" among the plain colours.
fn brick_bomb() -> Vec<u8> {
    let (cx, cy) = (BRICK_W as f32 / 2.0, BRICK_H as f32 / 2.0);
    block(|x, y| {
        // an eight-pointed burst: orange, with a yellow heart
        let (dx, dy) = ((x as f32 + 0.5 - cx) / 1.5, y as f32 + 0.5 - cy);
        let d = dx.hypot(dy);
        let reach = 4.5 + 2.0 * (dy.atan2(dx) * 8.0).cos();
        if d < 2.4 { (255.0, 235.0, 120.0) } else if d < reach { (250.0, 140.0, 40.0) } else { (62.0, 54.0, 74.0) }
    })
    .encode_png()
}

/// Two-hit brick; the cracked variant shows its remaining hit with a darker, fractured surface.
fn brick_steel(cracked: bool) -> Vec<u8> {
    let base = if cracked { (112.0, 124.0, 140.0) } else { (156.0, 174.0, 194.0) };
    let mut img = block(|_, _| base);
    let (w, h) = (BRICK_W, BRICK_H);
    // corner rivets sell the "reinforced plate" look
    for (rx, ry) in [(w - 6, 5), (5, h - 6), (w - 6, h - 6)] {
        img.set(rx, ry, 70, 74, 86);
        img.set(rx, ry - 1, 235, 238, 242);
    }
    if cracked {
        let ink = blip::look::INK;
        let mut toggle = 0;
        for x in (6..w - 6).step_by(3) {
            let y = h / 2 + toggle;
            img.set(x, y, ink[0], ink[1], ink[2]);
            img.set(x + 1, y, ink[0], ink[1], ink[2]);
            toggle = if toggle <= 0 { 2 } else { -2 };
        }
    }
    img.encode_png()
}
