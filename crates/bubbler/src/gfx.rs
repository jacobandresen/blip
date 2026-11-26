// Compiled by build.rs, and by the game only for its tests: much goes unused in each.
#![allow(dead_code)]

use blip::image::{coverage, mix_rgb as mix, Image};
use blip::Asset;

// One texture per letter preserves the title's individual bobbing motion.
fn title_letter(ch: char, index: usize) -> Vec<u8> {
    let mut image = Image::new(68, 74);
    let hue = ((0.9 + index as f32 * 0.06) % 1.0) * 6.0;
    let x = 0.45 * (1.0 - ((hue % 2.0) - 1.0).abs());
    let (r, g, b) = match hue as i32 {
        0 => (0.45, x, 0.0),
        1 => (x, 0.45, 0.0),
        2 => (0.0, 0.45, x),
        3 => (0.0, x, 0.45),
        4 => (x, 0.0, 0.45),
        _ => (0.45, 0.0, x),
    };
    let color = [((r + 0.55) * 255.0) as u8, ((g + 0.55) * 255.0) as u8, ((b + 0.55) * 255.0) as u8];
    let mut shape = |rgb: [u8; 3], alpha: f32, inside: &dyn Fn(f32, f32) -> bool| {
        for y in 0..74 {
            for x in 0..68 {
                let covered = coverage(x, y, inside);
                if covered > 0 {
                    image.blend(x, y, [rgb[0], rgb[1], rgb[2], (alpha * covered as f32 / 9.0 * 255.0).round() as u8]);
                }
            }
        }
    };
    shape(color, 0.16, &|x, y| (x - 31.0).hypot(y - 37.0) <= 27.6);
    shape(color, 0.7, &|x, y| ((x - 31.0).hypot(y - 37.0) - 27.6).abs() <= 1.0);
    shape([255, 255, 255], 0.7, &|x, y| {
        let (dx, dy) = (x - 20.2, y - 23.8);
        let angle = -35.0_f32.to_radians();
        let ex = (dx * angle.cos() + dy * angle.sin()) / 7.8;
        let ey = (-dx * angle.sin() + dy * angle.cos()) / 4.2;
        ex * ex + ey * ey <= 1.0
    });
    let text = ch.to_string();
    for (dx, dy) in [(-2, 0), (2, 0), (0, -2), (0, 2)] {
        image.text(&text, 16 + dx, 16 + dy, 6, [38, 5, 63, 230]);
    }
    image.text(&text, 16, 16, 6, [color[0], color[1], color[2], 255]);
    image.encode_png()
}

fn title_panel(image: &mut Image, y: f32, width: f32, height: f32, edge: [u8; 4]) {
    let radius = (height / 2.0).min(14.0);
    for (grow, color) in [(1.6, edge), (0.0, [34, 14, 44, 188])] {
        let (half_w, half_h, r) = (width / 2.0 + grow, height / 2.0 + grow, radius + grow);
        for py in (y as i32 - 2)..(y as i32 + height as i32 + 2) {
            for px in (312.0 - half_w).floor() as i32..(312.0 + half_w).ceil() as i32 {
                let dx = (px as f32 + 0.5 - 312.0).abs() - half_w + r;
                let dy = (py as f32 + 0.5 - y - height / 2.0).abs() - half_h + r;
                let distance = dx.max(0.0).hypot(dy.max(0.0)) + dx.max(dy).min(0.0) - r;
                let coverage = (0.5 - distance).clamp(0.0, 1.0);
                image.blend(px, py, [color[0], color[1], color[2], (color[3] as f32 * coverage) as u8]);
            }
        }
    }
}

fn title_text(image: &mut Image, text: &str, y: i32, color: [u8; 4]) {
    let x = (624 - (text.len() as i32 * 12 - 2)) / 2;
    image.text(text, x + 1, y + 1, 2, [60, 20, 70, 230]);
    image.text(text, x, y, 2, color);
}

fn title_info(mode: usize) -> Vec<u8> {
    let mut image = Image::new(624, 92);
    let join = ["P2 PRESS J TO JOIN", "P2 PRESS BUBBLE TO JOIN", "P2 TOUCH TO JOIN"][mode];
    title_text(&mut image, join, 6, [165, 215, 255, 255]);
    let hints = [
        ["MOVE A D   JUMP W   BUBBLE F", "HOLD JUMP TO RIDE BUBBLES"],
        ["PAD MOVES   BUTTONS BUBBLE AND JUMP", "HOLD JUMP TO RIDE BUBBLES"],
        ["TOUCH TO BUBBLE   SLIDE TO RUN", "SWIPE UP TO JUMP"],
    ][mode];
    title_panel(&mut image, 30.0, hints[0].len() as f32 * 12.0 + 34.0, 54.0, [255, 196, 150, 165]);
    title_text(&mut image, hints[0], 40, [255, 244, 222, 242]);
    title_text(&mut image, hints[1], 60, [255, 244, 222, 242]);
    image.encode_png()
}

fn title_start(mode: usize) -> Vec<u8> {
    let mut image = Image::new(624, 40);
    let text = ["P1 PRESS SPACE", "P1 PRESS BUBBLE", "P1 TOUCH TO START"][mode];
    title_panel(&mut image, 4.0, text.len() as f32 * 12.0 + 24.0, 32.0, [170, 240, 190, 230]);
    title_text(&mut image, text, 13, [170, 240, 190, 255]);
    image.encode_png()
}

fn title_sky() -> Vec<u8> {
    let mut image = Image::new(624, 600);
    for y in 0..600 {
        let t = (((y as f32 + 0.5) * 18.0 / 600.0).floor() / 17.0).min(1.0);
        let color = ((18.0 + 30.0 * t) as u8, (12.0 + 8.0 * t) as u8, (52.0 + 32.0 * t) as u8);
        for x in 0..624 {
            image.set(x, y, color.0, color.1, color.2);
        }
    }
    image.encode_png()
}

type Rgb = (u8, u8, u8);

// ---- kawaii sprites --------------------------------------------------------
// Drawn at twice the 32x36 grid (64x72) and shown smoothed, so the toys keep
// soft edges, stitches and fuzz. The look follows Harajuku kawaii: pastel
// bodies, big glossy eyes set low, blush, a tiny mouth, and decora clips and bows.

/// Paint every pixel `inside` covers (3x3 samples a pixel, for smooth edges).
fn fill(img: &mut Image, c: [u8; 4], inside: impl Fn(f32, f32) -> bool) {
    for y in 0..img.h as i32 {
        for x in 0..img.w as i32 {
            let n = coverage(x, y, &inside);
            if n > 0 {
                img.blend(x, y, [c[0], c[1], c[2], (c[3] as f32 * n as f32 / 9.0) as u8]);
            }
        }
    }
}

/// Inside an ellipse turned `rot` radians; `fuzz` ruffles its edge like felt.
fn ell(cx: f32, cy: f32, rx: f32, ry: f32, rot: f32, fuzz: f32) -> impl Fn(f32, f32) -> bool {
    move |x, y| {
        let (dx, dy) = (x - cx, y - cy);
        let (u, v) = (dx * rot.cos() + dy * rot.sin(), -dx * rot.sin() + dy * rot.cos());
        let a = v.atan2(u);
        let k = 1.0 + fuzz * ((a * 17.0).sin() * 0.6 + (a * 29.0 + 1.3).sin() * 0.4);
        (u / (rx * k)).powi(2) + (v / (ry * k)).powi(2) <= 1.0
    }
}

fn rgba(c: Rgb, a: u8) -> [u8; 4] {
    [c.0, c.1, c.2, a]
}

fn mul(c: Rgb, k: f32) -> Rgb {
    let f = |v: u8| (v as f32 * k).min(255.0) as u8;
    (f(c.0), f(c.1), f(c.2))
}

/// A smooth, squeezable silicone ball (a squishy toy), lit like a sphere from
/// the top left, with a thin darker rim and a soft shine.
fn ball(img: &mut Image, cx: f32, cy: f32, rx: f32, ry: f32, body: Rgb) {
    fill(img, rgba(mul(body, 0.62), 255), ell(cx, cy + 0.6, rx + 1.4, ry + 1.4, 0.0, 0.0));
    for y in 0..img.h as i32 {
        for x in 0..img.w as i32 {
            let (u, v) = ((x as f32 + 0.5 - cx) / rx, (y as f32 + 0.5 - cy) / ry);
            let d = u * u + v * v;
            if d > 1.0 {
                continue;
            }
            let cover = ((1.0 - d.sqrt()) * rx.min(ry)).clamp(0.0, 1.0);
            let z = (1.0 - d).sqrt();
            let lit = (-0.42 * u - 0.5 * v + 0.76 * z).max(0.0);
            let c = mul(body, 0.74 + 0.36 * lit);
            img.blend(x, y, rgba(c, (cover * 255.0) as u8));
        }
    }
    fill(img, [255, 255, 255, 70], ell(cx - rx * 0.36, cy - ry * 0.46, rx * 0.34, ry * 0.2, -0.5, 0.0));
}

/// A squishy's big eye: a ring of `iris` colour round a dark pupil, shines on top.
/// Friendly, not staring: a dark glossy oval, a little taller than wide, with
/// only a hint of `iris` colour low down, a big shine and a small one.
fn iris_eye(img: &mut Image, x: f32, y: f32, r: f32, iris: Rgb) {
    let (rx, ry) = (r * 0.78, r);
    fill(img, [50, 30, 58, 255], ell(x, y, rx, ry, 0.0, 0.0));
    fill(img, rgba(mix((50, 30, 58), iris, 0.55), 255), |px, py| ell(x, y, rx, ry, 0.0, 0.0)(px, py) && py > y + ry * 0.35);
    img.circle(x - rx * 0.3, y - ry * 0.38, rx * 0.48, [255, 255, 255, 255]);
    img.circle(x + rx * 0.32, y + ry * 0.28, rx * 0.2, [255, 255, 255, 235]);
}

/// A big glossy kawaii eye: dark, a large shine up top and a small one below.
fn eye(img: &mut Image, x: f32, y: f32, rx: f32, ry: f32) {
    fill(img, [45, 25, 55, 255], ell(x, y, rx, ry, 0.0, 0.0));
    fill(img, [120, 70, 140, 255], |px, py| ell(x, y, rx, ry, 0.0, 0.0)(px, py) && py > y + ry * 0.35);
    img.circle(x - rx * 0.32, y - ry * 0.38, rx * 0.42, [255, 255, 255, 255]);
    img.circle(x + rx * 0.35, y + ry * 0.3, rx * 0.2, [255, 255, 255, 230]);
}

fn blush(img: &mut Image, x: f32, y: f32, w: f32) {
    fill(img, [255, 130, 170, 150], ell(x, y, w, w * 0.55, 0.0, 0.0));
}

/// The cat-like ω mouth.
fn omega(img: &mut Image, x: f32, y: f32, s: f32, c: [u8; 4]) {
    let p = [(-2.0, -0.6), (-1.0, 0.8), (0.0, -0.4), (1.0, 0.8), (2.0, -0.6)];
    for w in p.windows(2) {
        img.line((x + w[0].0 * s, y + w[0].1 * s), (x + w[1].0 * s, y + w[1].1 * s), 1.4, c);
    }
}

/// A ribbon bow: two rounded loops and a knot, tipped `rot` radians.
fn bow(img: &mut Image, x: f32, y: f32, s: f32, rot: f32, c: Rgb) {
    for side in [-1.0f32, 1.0] {
        let (dx, dy) = (rot.cos() * side * s * 0.9, rot.sin() * side * s * 0.9);
        fill(img, rgba(mul(c, 0.7), 255), ell(x + dx, y + dy + 0.6, s * 0.95, s * 0.7, rot + side * 0.35, 0.0));
        fill(img, rgba(c, 255), ell(x + dx, y + dy, s * 0.85, s * 0.6, rot + side * 0.35, 0.0));
        img.circle(x + dx * 1.15 - s * 0.15, y + dy - s * 0.2, s * 0.18, [255, 255, 255, 170]);
    }
    fill(img, rgba(mul(c, 0.8), 255), ell(x, y, s * 0.4, s * 0.45, 0.0, 0.0));
}

/// A five-pointed star of outer radius `r`.
fn star(img: &mut Image, x: f32, y: f32, r: f32, c: [u8; 4]) {
    fill(img, c, move |px, py| {
        let (dx, dy) = (px - x, py - y);
        let a = dy.atan2(dx) + std::f32::consts::FRAC_PI_2;
        let seg = std::f32::consts::TAU / 5.0;
        let t = ((a.rem_euclid(seg)) / seg - 0.5).abs() * 2.0;
        dx.hypot(dy) <= r * (0.45 + 0.55 * t.powf(1.6))
    });
}

/// A heart of half-width `s`.
fn heart(img: &mut Image, x: f32, y: f32, s: f32, c: [u8; 4]) {
    fill(img, c, move |px, py| {
        let (u, v) = ((px - x) / s, -(py - y) / s + 0.25);
        (u * u + v * v - 1.0).powi(3) - u * u * v * v * v <= 0.0
    });
}

/// A squishy toy, after the Kawaii Kuties: a round silicone ball with nub ears,
/// stubby arms and feet, huge eyes and a face or belly patch, turned a little
/// to the right (mirrored it faces left). `kind` 0 an owl (the walker), 1 a
/// shiba dog (the hopper), 2 a panda (the floater).
fn plush(body: Rgb, angry_body: Rgb, kind: usize, angry: bool) -> Vec<u8> {
    let body = if angry { angry_body } else { body };
    let mut img = Image::new(64, 72);
    let cream = (255, 240, 205);
    let ink = [45, 28, 50, 255];
    // ears behind the ball
    match kind {
        0 => {
            // feather tufts
            fill(&mut img, rgba(mul(body, 0.62), 255), |x, y| y > 8.0 && y < 24.0 && ((x - 13.0).abs() < (y - 7.0) * 0.55 || (x - 51.0).abs() < (y - 7.0) * 0.55));
            fill(&mut img, rgba(mul(body, 0.95), 255), |x, y| y > 10.0 && y < 24.0 && ((x - 13.5).abs() < (y - 9.5) * 0.45 || (x - 50.5).abs() < (y - 9.5) * 0.45));
        }
        1 => {
            for x in [16.0, 48.0] {
                fill(&mut img, rgba(mul(body, 0.62), 255), move |px, py| py > 6.0 && py < 22.0 && (px - x).abs() < (py - 5.0) * 0.6);
                fill(&mut img, rgba(body, 255), move |px, py| py > 8.0 && py < 22.0 && (px - x).abs() < (py - 7.5) * 0.5);
                fill(&mut img, rgba(cream, 255), move |px, py| py > 12.0 && py < 22.0 && (px - x).abs() < (py - 11.5) * 0.3);
            }
        }
        _ => {
            for x in [15.0, 49.0] {
                ball(&mut img, x, 15.0, 6.5, 6.5, (70, 62, 80));
            }
        }
    }
    // stubby feet and arms
    let limb = if kind == 2 { (70, 62, 80) } else { mul(body, 0.92) };
    ball(&mut img, 22.0, 62.0, 6.0, 4.0, limb);
    ball(&mut img, 42.0, 62.0, 6.0, 4.0, limb);
    ball(&mut img, 32.0, 37.0, 25.0, 25.0, body);
    // the arms are drawn live by the game (art.rs), so they can wave
    match kind {
        0 => {
            // scalloped feather belly and a little beak
            fill(&mut img, rgba(cream, 255), ell(34.0, 51.0, 13.0, 9.0, 0.0, 0.0));
            for row in 0..3 {
                for i in 0..4 - (row == 2) as i32 {
                    let (x, y) = (26.0 + i as f32 * 5.5 + row as f32 * 2.5, 47.0 + row as f32 * 4.0);
                    for k in 0..4 {
                        let a = std::f32::consts::PI * (0.15 + k as f32 * 0.233);
                        let b = a + 0.6;
                        img.line((x + a.cos() * 2.2, y + a.sin() * 2.2), (x + b.cos() * 2.2, y + b.sin() * 2.2), 1.1, [200, 160, 90, 255]);
                    }
                }
            }
            // soft pale face discs round the eyes, an owl's look without the stare
            fill(&mut img, rgba(mix(body, (255, 255, 255), 0.55), 255), ell(24.0, 31.0, 8.5, 8.0, 0.0, 0.0));
            fill(&mut img, rgba(mix(body, (255, 255, 255), 0.55), 255), ell(44.0, 31.0, 8.5, 8.0, 0.0, 0.0));
            iris_eye(&mut img, 25.0, 32.0, 5.0, (240, 190, 80));
            iris_eye(&mut img, 43.0, 32.0, 5.0, (240, 190, 80));
            fill(&mut img, [235, 170, 60, 255], |x, y| y > 36.0 && y < 42.0 && (x - 34.0).abs() < (42.0 - y) * 0.5);
        }
        1 => {
            // white muzzle, green eyes, tongue out
            fill(&mut img, rgba(cream, 255), ell(35.0, 44.0, 15.0, 11.0, 0.0, 0.0));
            fill(&mut img, rgba(cream, 255), ell(26.0, 34.0, 5.0, 4.0, 0.0, 0.0));
            fill(&mut img, rgba(cream, 255), ell(44.0, 34.0, 5.0, 4.0, 0.0, 0.0));
            iris_eye(&mut img, 26.0, 32.0, 4.8, (120, 190, 90));
            iris_eye(&mut img, 44.0, 32.0, 4.8, (120, 190, 90));
            fill(&mut img, ink, ell(35.0, 39.5, 2.6, 1.8, 0.0, 0.0));
            fill(&mut img, [255, 120, 150, 255], ell(35.0, 46.5, 3.0, 3.6, 0.0, 0.0));
            omega(&mut img, 35.0, 42.5, 1.3, ink);
        }
        _ => {
            // eye patches tipped outward, blue eyes, a small nose
            fill(&mut img, [70, 62, 80, 255], ell(24.0, 33.0, 7.5, 9.0, 0.5, 0.0));
            fill(&mut img, [70, 62, 80, 255], ell(44.0, 33.0, 7.5, 9.0, -0.5, 0.0));
            iris_eye(&mut img, 25.0, 33.0, 4.6, (110, 150, 230));
            iris_eye(&mut img, 43.0, 33.0, 4.6, (110, 150, 230));
            fill(&mut img, ink, ell(34.0, 41.0, 2.4, 1.6, 0.0, 0.0));
            omega(&mut img, 34.0, 44.5, 1.3, ink);
        }
    }
    if angry {
        // cross, but cute: a pout, not a glare (small soft brows, big red cheeks, a puff of steam)
        img.line((20.0, 24.0), (27.0, 25.5), 1.6, [140, 60, 90, 255]);
        img.line((48.0, 24.0), (41.0, 25.5), 1.6, [140, 60, 90, 255]);
        fill(&mut img, [255, 255, 255, 200], ell(55.0, 14.0, 3.0, 2.2, 0.0, 0.0));
        fill(&mut img, [255, 255, 255, 170], ell(59.0, 10.0, 2.0, 1.6, 0.0, 0.0));
    }
    blush(&mut img, 17.0, 42.0, if angry { 5.0 } else { 3.6 });
    blush(&mut img, 51.0, 42.0, if angry { 5.0 } else { 3.6 });
    img.encode_png()
}

/// A dragon (`player` 0 or 1): round and pastel, cream spikes, big glossy
/// eyes, blush, and decora hair clips: a pink bow and a mint clip for player
/// one, a yellow star and a pink heart for player two.
fn dragon(body: Rgb, player: usize) -> Vec<u8> {
    let mut img = Image::new(64, 72);
    let dark = mul(body, 0.62);
    let cream = (255, 242, 214);
    for x in [18.0, 46.0] {
        fill(&mut img, rgba(dark, 255), ell(x, 60.0, 10.0, 6.0, 0.0, 0.0));
    }
    for (x, y) in [(14.0, 26.0), (20.0, 18.0), (28.0, 14.0)] {
        fill(&mut img, rgba(mul(cream, 0.85), 255), ell(x, y + 1.0, 6.5, 6.5, 0.0, 0.0));
        fill(&mut img, rgba(cream, 255), ell(x, y, 5.8, 5.8, 0.0, 0.0));
    }
    fill(&mut img, rgba(dark, 255), ell(32.0, 40.0, 24.0, 22.0, 0.0, 0.0));
    fill(&mut img, rgba(body, 255), ell(32.0, 36.0, 22.0, 20.0, 0.0, 0.0));
    fill(&mut img, rgba(mix(body, (255, 255, 255), 0.4), 130), ell(24.0, 26.0, 9.0, 6.0, -0.5, 0.0));
    fill(&mut img, rgba(cream, 255), ell(40.0, 44.0, 14.0, 12.0, 0.0, 0.0));
    eye(&mut img, 27.0, 31.0, 4.6, 6.0);
    eye(&mut img, 41.0, 31.0, 4.6, 6.0);
    blush(&mut img, 20.0, 40.0, 3.6);
    blush(&mut img, 48.0, 40.0, 3.6);
    omega(&mut img, 46.0, 45.0, 1.5, [110, 45, 70, 255]);
    if player == 0 {
        bow(&mut img, 34.0, 13.0, 6.5, 0.25, (255, 95, 160));
        fill(&mut img, [120, 230, 200, 255], ell(45.0, 19.0, 4.5, 1.8, 0.6, 0.0));
    } else {
        star(&mut img, 35.0, 13.0, 7.5, [255, 210, 70, 255]);
        star(&mut img, 35.0, 13.0, 3.6, [255, 240, 160, 255]);
        heart(&mut img, 46.0, 19.0, 3.4, [255, 110, 160, 255]);
    }
    img.encode_png()
}


/// Each round's brick: its face, its lit edge and its shadow.
const TILES: [(Rgb, Rgb, Rgb); 8] = [
    ((240, 120, 180), (255, 190, 225), (160, 60, 120)),
    ((80, 200, 230), (170, 240, 255), (30, 110, 150)),
    ((250, 180, 60), (255, 230, 150), (170, 100, 20)),
    ((110, 220, 110), (190, 255, 180), (40, 130, 60)),
    ((190, 110, 250), (230, 190, 255), (110, 50, 170)),
    ((255, 120, 120), (255, 196, 186), (170, 56, 70)),
    ((226, 222, 84), (252, 250, 176), (136, 130, 34)),
    ((150, 240, 210), (222, 255, 240), (60, 150, 130)),
];

/// A round's block as a sofa cushion (64x64, shown smoothed): puffed up in
/// the middle and sinking at its rounded edges, piped round the rim, with a
/// tufted button in the middle and soft creases to its corners. The walls are
/// the same cushions in a deeper shade, like the sofa's back.
/// `piece`: 0 a lone cushion, 1 a run's left end, 2 its middle, 3 its right end
/// (a platform's tiles join into one long cushion).
fn tile(round: usize, border: bool, piece: usize) -> Vec<u8> {
    let (block, light, _) = TILES[round];
    let base = if border { mul(mix(block, (255, 255, 255), 0.1), 0.62) } else { mix(block, (255, 255, 255), 0.22) };
    let pipe = if border { mul(light, 0.7) } else { mix(light, (255, 255, 255), 0.35) };
    let mut img = Image::new(64, 64);
    let (c, half, r) = (32.0f32, 31.0f32, 13.0f32);
    // an open side runs on into the next tile: its edge is pushed far away
    let far = 1000.0;
    let (cx, half_x) = match piece {
        1 => (1.0 + far, far),
        2 => (32.0, far),
        3 => (63.0 - far, far),
        _ => (c, half),
    };
    for y in 0..64 {
        for x in 0..64 {
            let (px, py) = (x as f32 + 0.5 - c, y as f32 + 0.5 - c);
            // signed distance to the rounded box: negative inside
            let (qx, qy) = ((x as f32 + 0.5 - cx).abs() - half_x + r, py.abs() - half + r);
            let d = qx.max(0.0).hypot(qy.max(0.0)) + qx.max(qy).min(0.0) - r;
            let cover = (0.5 - d).clamp(0.0, 1.0);
            if cover <= 0.0 {
                continue;
            }
            let depth = (-d).max(0.0);
            // the puff: dark where it sinks to the seam, full in the middle, lit from the top left
            let puff = 0.72 + 0.34 * (depth / 14.0).min(1.0).sqrt();
            let lit = 1.0 - py / 64.0 * 0.16;
            // creases from the tuft to the corners, fading out toward the rim
            let diag = (px.abs() - py.abs()).abs();
            let crease = if diag < 2.2 && px.hypot(py) > 5.0 && depth > 7.0 { 0.9 + 0.1 * diag / 2.2 } else { 1.0 };
            // a faint woven texture
            let weave = if (x + y) % 2 == 0 { 1.02 } else { 0.985 };
            let mut col = mul(base, (puff * lit * crease * weave).min(1.25));
            if depth < 2.2 && depth > 0.6 {
                col = mix(col, pipe, 0.75);
            }
            img.blend(x, y, rgba(col, (cover * 255.0) as u8));
        }
    }
    // the tufted button, sunk into a dimple
    fill(&mut img, rgba(mul(base, 0.7), 200), ell(32.0, 32.5, 6.0, 5.5, 0.0, 0.0));
    fill(&mut img, rgba(mul(pipe, 0.95), 255), ell(32.0, 32.0, 3.4, 3.4, 0.0, 0.0));
    img.circle(31.0, 31.0, 1.2, [255, 255, 255, 170]);
    // a soft sheen on the top
    fill(&mut img, [255, 255, 255, if border { 18 } else { 40 }], ell(24.0, 15.0, 14.0, 5.0, -0.2, 0.0));
    img.encode_png()
}

/// The game's sprites.
pub fn generate() -> Vec<Asset> {
    let mut assets = vec![
        ("images/TITLE_SKY.png", title_sky()),
        ("images/TITLE_INFO_KEYS.png", title_info(0)),
        ("images/TITLE_INFO_PAD.png", title_info(1)),
        ("images/TITLE_INFO_TOUCH.png", title_info(2)),
        ("images/TITLE_START_KEYS.png", title_start(0)),
        ("images/TITLE_START_PAD.png", title_start(1)),
        ("images/TITLE_START_TOUCH.png", title_start(2)),
        ("images/TITLE_LETTER_1.png", title_letter('B', 0)),
        ("images/TITLE_LETTER_2.png", title_letter('U', 1)),
        ("images/TITLE_LETTER_3.png", title_letter('B', 2)),
        ("images/TITLE_LETTER_4.png", title_letter('B', 3)),
        ("images/TITLE_LETTER_5.png", title_letter('L', 4)),
        ("images/TITLE_LETTER_6.png", title_letter('E', 5)),
        ("images/TITLE_LETTER_7.png", title_letter('R', 6)),
        ("images/PLAYER_GREEN.png", dragon((120, 220, 150), 0)),
        ("images/PLAYER_BLUE.png", dragon((130, 190, 255), 1)),
        // the squishies: a blue owl (pink when cross), an orange shiba (red), a white panda (pink)
        ("images/ROBOT.png", plush((150, 190, 215), (245, 110, 175), 0, false)),
        ("images/ROBOT_ANGRY.png", plush((150, 190, 215), (245, 110, 175), 0, true)),
        ("images/BUNNY.png", plush((245, 165, 85), (240, 105, 85), 1, false)),
        ("images/BUNNY_ANGRY.png", plush((245, 165, 85), (240, 105, 85), 1, true)),
        ("images/GHOST.png", plush((246, 244, 250), (255, 200, 222), 2, false)),
        ("images/GHOST_ANGRY.png", plush((246, 244, 250), (255, 200, 222), 2, true)),
    ];
    let names = [
        ("images/TILE_1.png", "images/TILE_1_L.png", "images/TILE_1_M.png", "images/TILE_1_R.png", "images/BORDER_1.png"),
        ("images/TILE_2.png", "images/TILE_2_L.png", "images/TILE_2_M.png", "images/TILE_2_R.png", "images/BORDER_2.png"),
        ("images/TILE_3.png", "images/TILE_3_L.png", "images/TILE_3_M.png", "images/TILE_3_R.png", "images/BORDER_3.png"),
        ("images/TILE_4.png", "images/TILE_4_L.png", "images/TILE_4_M.png", "images/TILE_4_R.png", "images/BORDER_4.png"),
        ("images/TILE_5.png", "images/TILE_5_L.png", "images/TILE_5_M.png", "images/TILE_5_R.png", "images/BORDER_5.png"),
        ("images/TILE_6.png", "images/TILE_6_L.png", "images/TILE_6_M.png", "images/TILE_6_R.png", "images/BORDER_6.png"),
        ("images/TILE_7.png", "images/TILE_7_L.png", "images/TILE_7_M.png", "images/TILE_7_R.png", "images/BORDER_7.png"),
        ("images/TILE_8.png", "images/TILE_8_L.png", "images/TILE_8_M.png", "images/TILE_8_R.png", "images/BORDER_8.png"),
    ];
    for (round, (single, left, mid, right, border)) in names.into_iter().enumerate() {
        for (piece, name) in [single, left, mid, right].into_iter().enumerate() {
            assets.push((name, tile(round, false, piece)));
        }
        assets.push((border, tile(round, true, 0)));
    }
    assets
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sprites_and_round_tiles_have_their_intended_png_sizes() {
        let assets = generate();
        assert_eq!(assets.len(), 62);
        for (path, bytes) in assets {
            assert!(path.starts_with("images/"));
            let img = Image::from_png(&bytes);
            let is_tile = path.contains("TILE_") || path.contains("BORDER_");
            let expected_size = if path.contains("TITLE_LETTER_") {
                (68, 74)
            } else if path.contains("TITLE_INFO_") {
                (624, 92)
            } else if path.contains("TITLE_START_") {
                (624, 40)
            } else if path.contains("TITLE_SKY") {
                (624, 600)
            } else if is_tile {
                (64, 64)
            } else {
                (64, 72)
            };
            assert_eq!((img.w, img.h), expected_size, "{path}");
            let mut alpha = img.px.chunks_exact(4).map(|px| px[3]);
            assert!(alpha.clone().any(|a| a > 0), "{path}");
            if path.contains("TITLE_SKY") {
                assert!(alpha.all(|a| a == 255), "{path}");
            } else {
                assert!(alpha.any(|a| a == 0), "{path}");
            }
        }
    }
}
