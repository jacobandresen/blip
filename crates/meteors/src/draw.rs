//! Drawing: the ship, rocks and flowers, the haze and its smears, the saucer, crystals, stars, the play field, and
//! the title and game-over screens.

use super::*;

pub(super) fn draw_ship(blip: &Blip, ship: &Ship, invuln_t: f32, level: i32, color: BlipColor) {
    if invuln_t > 0.0 && (invuln_t * 10.0) as i32 % 2 == 0 { return; }
    let a = ship.angle;
    let fwd = (a.sin(), -a.cos());
    let side = (a.cos(), a.sin());
    let nose = (ship.x + fwd.0 * 14.0, ship.y + fwd.1 * 14.0);
    let left = (ship.x - fwd.0 * 10.0 - side.0 * 9.0, ship.y - fwd.1 * 10.0 - side.1 * 9.0);
    let right = (ship.x - fwd.0 * 10.0 + side.0 * 9.0, ship.y - fwd.1 * 10.0 + side.1 * 9.0);
    let tail = (ship.x - fwd.0 * 4.0, ship.y - fwd.1 * 4.0);
    let hull = [(nose, left), (left, tail), (tail, right), (right, nose)];
    for (back, age) in smear_echoes(blip, level) {
        let (dx, dy) = (ship.vx * back, ship.vy * back);
        for (p, q) in hull {
            smear_line(blip, (p, q), ((p.0 - dx, p.1 - dy), (q.0 - dx, q.1 - dy)), age, color);
        }
    }
    if ship.thrusting {
        let flick = rand_range_f32(0.5, 1.0);
        let flame = (ship.x - fwd.0 * (10.0 + 10.0 * flick), ship.y - fwd.1 * (10.0 + 10.0 * flick));
        let (l, r) = ((tail.0 * 0.4 + left.0 * 0.6, tail.1 * 0.4 + left.1 * 0.6), (tail.0 * 0.4 + right.0 * 0.6, tail.1 * 0.4 + right.1 * 0.6));
        outlined_shape(blip, tail, &[l, flame, r], BLIP_GOLD, 1.5);
    }
    outlined_shape(blip, (ship.x, ship.y), &[nose, left, tail, right], color, 2.0);
    // The canopy.
    blip.fill_circle(ship.x + fwd.0 * 3.0, ship.y + fwd.1 * 3.0, 2.5, BLIP_INK);
    blip.fill_circle(ship.x + fwd.0 * 3.0 - 0.5, ship.y + fwd.1 * 3.0 - 0.5, 1.5, BLIP_SEA);
}

// Line smearing and rock colours increase by wave; flowers appear from wave 3.

pub(super) type Seg = ((f32, f32), (f32, f32));

/// A filled shape in the blip look: an ink rim `rim` thick (round at the corners), the body, and a lit patch
/// toward the top left. `pts` go round `centre` in order.
pub(super) fn outlined_shape(blip: &Blip, centre: (f32, f32), pts: &[(f32, f32)], fill: BlipColor, rim: f32) {
    let n = pts.len();
    for i in 0..n {
        let (p, q) = (pts[i], pts[(i + 1) % n]);
        blip.draw_line_ex(p.0, p.1, q.0, q.1, rim * 2.0, BLIP_INK);
        blip.fill_circle(p.0, p.1, rim, BLIP_INK);
    }
    // The shaded side, then the face shifted toward the light, which leaves a dark crescent at the bottom right.
    blip.fill_fan(centre, pts, blip::shade(fill, 0.68));
    let lit: Vec<(f32, f32)> = pts.iter()
        .map(|&(x, y)| (centre.0 + (x - centre.0) * 0.82 - 1.5, centre.1 + (y - centre.1) * 0.82 - 1.5)).collect();
    blip.fill_fan((centre.0 - 1.5, centre.1 - 1.5), &lit, fill);
}

/// The echoes a moving line leaves, oldest first: how many seconds back each
/// one stands, and how bright it is for its age. Both grow with the wave.
pub(super) fn smear_echoes(blip: &Blip, level: i32) -> impl Iterator<Item = (f32, f32)> {
    // A device already short of frame time gets four at most.
    let n = (1 + level).min(if blip.low_power() { 4 } else { SMEAR_ECHOES });
    let reach = (SMEAR_REACH * (1 + level) as f32).min(SMEAR_REACH_MAX);
    (1..=n).rev().map(move |e| (reach * e as f32 / n as f32, 1.0 - (e - 1) as f32 / n as f32))
}

/// One echo of a line: where it was `then`, lit only by how far it has come
/// since. What moves fast streaks; what stands still stays a sharp line.
pub(super) fn smear_line(blip: &Blip, now: Seg, then: Seg, age: f32, c: BlipColor) {
    let dist = |a: (f32, f32), b: (f32, f32)| ((b.0 - a.0).powi(2) + (b.1 - a.1).powi(2)).sqrt();
    let moved = (dist(now.0, then.0) + dist(now.1, then.1)) * 0.5;
    if moved < SMEAR_STILL { return; }
    let lit = ((moved - SMEAR_STILL) / SMEAR_FULL).min(1.0) * age;
    blip.draw_line_ex(then.0.0, then.0.1, then.1.0, then.1.1, 1.0, BlipColor { a: SMEAR_ALPHA * lit, ..c });
}

/// A rock's own fixed random number, 0..1, from its shape.
pub(super) fn rock_seed(a: &Asteroid, k: usize) -> f32 { (a.jag[k] * 7.13).fract() }

/// A rock's colour: wave 1's purple, fanning out round the rainbow wave by
/// wave, and drifting from wave 4.
pub(super) fn rock_color(a: &Asteroid, level: i32, t: f32) -> BlipColor {
    if level <= 1 { return BLIP_PLUM; }
    let spread = (0.18 * (level - 1) as f32).min(1.0);
    let drift = if level >= 4 { t * 0.04 * (level - 3) as f32 } else { 0.0 };
    hsv(0.76 + (rock_seed(a, 3) - 0.5) * spread + drift, 0.5, 0.82, 1.0)
}

/// From wave 3 a growing share of the rocks are flowers: a quarter, half,
/// three quarters, then nine in ten.
pub(super) fn is_flower(a: &Asteroid, level: i32) -> bool {
    rock_seed(a, 5) < ((level - 2) as f32 * 0.25).clamp(0.0, 0.9)
}

/// A rock's outline round its own centre, as it is turned now: a flower's
/// petals (the same size as the rock it stands for) or the jagged ring.
pub(super) fn outline(a: &Asteroid, flower: bool) -> Vec<Seg> {
    let r = a.size.radius();
    if flower {
        let petals = 5 + (rock_seed(a, 1) * 3.0) as usize;
        (0..petals * 10).map(|i| {
            let (uy, ux) = (a.rot + (i / 10) as f32 / petals as f32 * PI * 2.0).sin_cos();
            let pt = |k: usize| {
                let (ty, tx) = (k as f32 / 10.0 * PI * 2.0).sin_cos();
                let (ex, ey) = (r * (0.55 + 0.45 * tx), r * 0.22 * ty);
                (ux * ex - uy * ey, uy * ex + ux * ey)
            };
            (pt(i % 10), pt(i % 10 + 1))
        }).collect()
    } else {
        let n = a.jag.len();
        let pt = |k: usize| {
            let (s, c) = (a.rot + (k % n) as f32 / n as f32 * PI * 2.0).sin_cos();
            (c * r * a.jag[k % n], s * r * a.jag[k % n])
        };
        (0..n).map(|i| (pt(i), pt(i + 1))).collect()
    }
}

/// A mushroom in thin lines, its cap `w` to each side, leaning by `lean`.
pub(super) fn draw_mushroom(blip: &Blip, x: f32, y: f32, w: f32, lean: f32, c: BlipColor) {
    let (s, k) = lean.sin_cos();
    let at = |px: f32, py: f32| (x + (px * k - py * s) * w, y + (px * s + py * k) * w);
    let line = |p: (f32, f32), q: (f32, f32)| blip.draw_line_ex(p.0, p.1, q.0, q.1, 1.5, c);
    // part of an ellipse, from angle `a0` to `a1`, in `n` lines
    let arc = |cx: f32, cy: f32, rx: f32, ry: f32, a0: f32, a1: f32, n: usize| {
        let pt = |j: usize| {
            let (sn, cs) = (a0 + (a1 - a0) * j as f32 / n as f32).sin_cos();
            at(cx + cs * rx, cy + sn * ry)
        };
        for j in 0..n { line(pt(j), pt(j + 1)); }
    };
    arc(0.0, -0.2, 1.0, 0.85, PI, PI * 2.0, 12); // the cap
    arc(0.0, -0.2, 1.0, 0.18, 0.0, PI, 8);       // its underside
    line(at(-0.26, -0.03), at(-0.36, 0.9));      // the stalk
    line(at(0.26, -0.03), at(0.36, 0.9));
    arc(0.0, 0.9, 0.36, 0.1, 0.0, PI, 5);
    for (sx, sy, sr) in [(-0.45, -0.58, 0.16), (0.1, -0.75, 0.2), (0.55, -0.45, 0.12)] {
        arc(sx, sy, sr, sr * 0.8, 0.0, PI * 2.0, 8);
    }
}

/// A thread from `p` to `q`, shading from `c0` to `c1`: a faint bowed line
/// with a brighter pulse `pulse` of the way along it.
#[allow(clippy::too_many_arguments)]
pub(super) fn draw_thread(blip: &Blip, p: (f32, f32), q: (f32, f32), bow: f32, pulse: f32, c0: BlipColor, c1: BlipColor, a: f32) {
    const N: usize = 14;
    let (dx, dy) = (q.0 - p.0, q.1 - p.1);
    let at = |u: f32| {
        let b = 4.0 * u * (1.0 - u) * bow;
        (p.0 + dx * u - dy * b, p.1 + dy * u + dx * b)
    };
    for j in 0..N {
        let u = j as f32 / N as f32;
        let (m, n) = (at(u), at(u + 1.0 / N as f32));
        let lit = 1.0 + 2.0 * (1.0 - ((u - pulse).abs() * 6.0).min(1.0));
        let mix = |x: f32, y: f32| x + (y - x) * u;
        blip.draw_line(m.0, m.1, n.0, n.1,
            BlipColor { r: mix(c0.r, c1.r), g: mix(c0.g, c1.g), b: mix(c0.b, c1.b), a: a * lit });
    }
}

/// Green and purple patches behind the field, a mushroom at the heart of
/// each, coming and going on clocks of their own; by HAZE_FULL_WAVE little
/// black is left. From wave 3 threads join the mushrooms, later the rocks.
pub(super) fn draw_haze(blip: &Blip, blot: &Texture2D, rocks: &[Asteroid], level: i32, t: f32) {
    let full = ((level - 1) as f32 / (HAZE_FULL_WAVE - 1) as f32).clamp(0.0, 1.0);
    let patch = |i: usize| {
        let k = i as f32;
        let x = PLAY_W as f32 * (0.5 + 0.45 * (t * (0.05 + 0.011 * k) + k * 2.4).sin());
        let y = PLAY_Y0 + PLAY_H as f32 * (0.5 + 0.45 * (t * (0.04 + 0.013 * k) + k * 1.7).cos());
        let r = 200.0 + 70.0 * (k * 1.3).sin();
        // squared, so on the early waves each is gone a while between showings
        let fade = (0.5 + 0.5 * (t * (0.21 + 0.03 * k) + k * 1.9).sin()).powi(2);
        (x, y, r, 0.7 * full + (1.0 - 0.7 * full) * fade, if i.is_multiple_of(2) { BLIP_LEAF } else { BLIP_PLUM })
    };
    for i in 0..HAZES {
        let (x, y, r, fade, c) = patch(i);
        let r = r * (1.0 + 0.6 * full);
        // purple is the darker of the two to the eye, so it is laid on thicker
        let a = (HAZE_ALPHA + (HAZE_ALPHA_MAX - HAZE_ALPHA) * full) * if i % 2 == 0 { 1.0 } else { 1.5 };
        blip.draw_texture_tinted(blot, x - r, y - r, r * 2.0, r * 2.0, BlipColor { a: a * fade, ..c });
    }
    if level >= THREAD_WAVE {
        let grown = ((level - THREAD_WAVE) as f32 / THREAD_WAVES as f32).min(1.0);
        let a = THREAD_ALPHA + (THREAD_ALPHA_MAX - THREAD_ALPHA) * grown;
        // from the foot of the stalk
        let foot = |i: usize| { let (x, y, r, fade, c) = patch(i); ((x, y + r * 0.13), fade, c) };
        for i in 0..HAZES {
            let k = i as f32;
            let (p, fade, c) = foot(i);
            // each to the next, and two waves on to one across the ring as well
            for (n, step) in [1, 3].into_iter().enumerate().take(if level >= THREAD_WAVE + 2 { 2 } else { 1 }) {
                let (q, fade2, c2) = foot((i + step) % HAZES);
                let bow = 0.12 * (t * 0.3 + k + n as f32 * 2.0).sin();
                draw_thread(blip, p, q, bow, (t * 0.15 + k * 0.37 + n as f32 * 0.5).fract(), c, c2, a * fade.min(fade2));
            }
            if level < THREAD_ROCK_WAVE { continue; }
            let near = pool_iter(rocks)
                .map(|rock| (rock, (rock.x - p.0).hypot(rock.y - p.1)))
                .filter(|&(_, d)| d < THREAD_REACH)
                .min_by(|m, n| m.1.total_cmp(&n.1));
            if let Some((rock, d)) = near {
                // thinning out as the rock gets away
                let hold = (1.0 - d / THREAD_REACH) * 0.6;
                draw_thread(blip, p, (rock.x, rock.y), 0.1 * (t * 0.4 + k).sin(), (t * 0.3 + k * 0.61).fract(),
                    c, rock_color(rock, level, t), a * fade * hold);
            }
        }
    }
    for i in 0..HAZES {
        let (x, y, r, fade, c) = patch(i);
        let a = SHROOM_ALPHA + (SHROOM_ALPHA_MAX - SHROOM_ALPHA) * full;
        let lean = 0.12 * (t * 0.5 + i as f32).sin();
        draw_mushroom(blip, x, y, r * 0.14, lean, BlipColor { a: a * fade, ..c });
    }
}

pub(super) fn draw_rock(blip: &Blip, a: &Asteroid, level: i32, t: f32) {
    let c = rock_color(a, level, t);
    let flower = is_flower(a, level);
    let lines = outline(a, flower);
    let at = |p: (f32, f32)| (a.x + p.0, a.y + p.1);
    for (back, age) in smear_echoes(blip, level) {
        // Where the rock was: drifted back, and turned back about its centre.
        let (ox, oy) = (a.x - a.vx * back, a.y - a.vy * back);
        let (s, k) = (-a.spin * back).sin_cos();
        let was = |p: (f32, f32)| (ox + p.0 * k - p.1 * s, oy + p.0 * s + p.1 * k);
        // A flower's echoes are drawn at half the outline's detail: two
        // lines as one, which halves the busiest thing on a late wave.
        for pair in lines.chunks(if flower { 2 } else { 1 }) {
            let (p, q) = (pair[0].0, pair[pair.len() - 1].1);
            smear_line(blip, (at(p), at(q)), (was(p), was(q)), age, c);
        }
    }
    if !flower {
        // A solid lump: ink rim, stone body, a lit face and two craters.
        let pts: Vec<(f32, f32)> = lines.iter().map(|&(p, _)| at(p)).collect();
        outlined_shape(blip, (a.x, a.y), &pts, c, 2.0);
        let r = a.size.radius();
        for (k, (dx, dy)) in [(0.3f32, 0.25f32), (-0.2, -0.35)].into_iter().enumerate() {
            let (s, co) = (a.rot + k as f32).sin_cos();
            let (x, y) = (a.x + (dx * co - dy * s) * r, a.y + (dx * s + dy * co) * r);
            blip.fill_circle(x, y, r * 0.16, blip::shade(c, 0.7));
        }
    } else {
        for &(p, q) in &lines {
            let (p, q) = (at(p), at(q));
            blip.draw_line_ex(p.0, p.1, q.0, q.1, 4.0, BLIP_INK);
        }
        for &(p, q) in &lines {
            let (p, q) = (at(p), at(q));
            blip.draw_line_ex(p.0, p.1, q.0, q.1, 2.0, c);
        }
        let heart = hsv(0.14 + rock_seed(a, 2) * 0.1, 0.7, 1.0, 1.0);
        let hr = a.size.radius() * 0.2;
        blip.fill_circle(a.x, a.y, hr + 2.0, BLIP_INK);
        blip.fill_circle(a.x, a.y, hr, heart);
    }
}

pub(super) fn draw_saucer(blip: &Blip, s: &Saucer, level: i32) {
    let r = saucer_radius(s.big);
    // A shot is coming: the core lights up and swells just before it fires.
    if s.fire_t < SAUCER_TELL {
        let k = 1.0 - s.fire_t / SAUCER_TELL;
        blip.fill_glow_circle(s.x, s.y, 2.0 + 4.0 * k, BLIP_HEART);
    }
    let w = r * 2.6;
    let dw = r * 1.1;
    let h = r * 0.5;
    let hull = |x: f32, y: f32| [
        (x - w / 2.0, y),
        (x - dw / 2.0, y - h),
        (x + dw / 2.0, y - h),
        (x + w / 2.0, y),
        (x + dw / 2.0, y + h),
        (x - dw / 2.0, y + h),
    ];
    let pts = hull(s.x, s.y);
    for (back, age) in smear_echoes(blip, level) {
        let was = hull(s.x - s.vx * back, s.y - (s.wave_t * 2.0).sin() * 24.0 * back);
        for i in 0..pts.len() {
            let j = (i + 1) % pts.len();
            smear_line(blip, (pts[i], pts[j]), (was[i], was[j]), age, BLIP_HEART);
        }
    }
    blip.fill_circle(s.x, s.y - h, dw * 0.45 + 2.0, BLIP_INK);
    blip.fill_circle(s.x, s.y - h, dw * 0.45, BLIP_SEA);
    outlined_shape(blip, (s.x, s.y), &pts, BLIP_HEART, 2.0);
    for k in [-1.0f32, 0.0, 1.0] {
        blip.fill_rect(s.x + k * w * 0.25 - 1.0, s.y - 1.0, 2.0, 2.0, BLIP_GOLD);
    }
}

/// A crystal: a turning diamond with a bright heart, blinking as it fades.
pub(super) fn draw_crystal(blip: &Blip, c: &Crystal, t: f32) {
    if c.ttl < CRYSTAL_WARN && (c.ttl * 8.0) as i32 % 2 == 0 { return; }
    let k = (t * 2.0).cos();
    let pts: Vec<(f32, f32)> = [(0.0, -1.3), (1.0, 0.0), (0.0, 1.3), (-1.0, 0.0)].iter()
        .map(|&(x, y)| (c.x + (x * k * 0.6 + 0.4 * x) * CRYSTAL_R, c.y + y * CRYSTAL_R)).collect();
    outlined_shape(blip, (c.x, c.y), &pts, BLIP_SEA, 1.5);
    blip.fill_circle(c.x - 1.0, c.y - 2.0, 1.5 + (t * 6.0).sin().abs() * 1.0, BLIP_CREAM);
}

/// Three layers of stars drifting at different speeds, the near ones brighter, each twinkling: depth and motion
/// behind an otherwise black wave 1. Placed by a hash, so the game's random numbers are untouched.
pub(super) fn draw_stars(blip: &Blip, t: f32) {
    for i in 0..110u32 {
        let h = i.wrapping_mul(0x9E37_79B9).rotate_left(7).wrapping_mul(0x85EB_CA6B);
        let layer = (h % 3) as f32;
        let speed = 3.0 + layer * 5.0;
        let x = wrap((h >> 4) as f32 % PLAY_W as f32 + t * speed, 0.0, PLAY_W as f32);
        let y = wrap((h >> 14) as f32 % PLAY_H as f32 + t * speed * 0.4, 0.0, PLAY_H as f32) + PLAY_Y0;
        let tw = 0.6 + 0.4 * (t * (1.0 + layer) + i as f32 * 1.7).sin();
        let a = (0.28 + layer * 0.2) * tw;
        let s = if layer == 2.0 { 2.0 } else { 1.0 };
        blip.fill_rect(x, y, s, s, BlipColor { a, ..BLIP_CREAM });
    }
}

pub(super) fn draw_play(blip: &Blip, g: &Game, blot: &Texture2D) {
    let t = blip::now();
    blip.clear(BLIP_NIGHT);
    draw_stars(blip, t);
    draw_haze(blip, blot, &g.asteroids, g.sess.level, t);
    for a in pool_iter(&g.asteroids) { draw_rock(blip, a, g.sess.level, t); }
    for d in pool_iter(&g.debris) {
        let h = d.len * 0.5;
        let col = BlipColor { a: d.ttl / d.ttl0, ..d.color };
        let shard = |back: f32| {
            let (x, y) = (d.x - d.vx * back, d.y - d.vy * back);
            let (s, c) = (d.rot - d.spin * back).sin_cos();
            ((x - c * h, y - s * h), (x + c * h, y + s * h))
        };
        let (p, q) = shard(0.0);
        // no echo from before the burst
        for (back, age) in smear_echoes(blip, g.sess.level).filter(|e| e.0 < d.ttl0 - d.ttl) {
            smear_line(blip, (p, q), shard(back), age, col);
        }
        blip.draw_line_ex(p.0, p.1, q.0, q.1, 2.0, col);
    }
    if g.saucer.active { draw_saucer(blip, &g.saucer, g.sess.level); }
    for c in &g.crystals { draw_crystal(blip, c, t); }
    g.fx.draw(blip);
    if g.wave_t < WAVE_BANNER_SECS && g.state == State::Play {
        let a = (1.0 - g.wave_t / WAVE_BANNER_SECS).min(0.5) * 2.0;
        blip.draw_message(&format!("WAVE {}", g.sess.level), (WIN_W / 2) as f32, PLAY_Y0 + 134.0, 3.0, BLIP_GOLD, a);
    }
    for b in pool_iter(&g.bullets) {
        // from wave 3 the ship's shots run through the rainbow
        let c = if !b.from_player { BLIP_HEART }
            else if g.sess.level >= 3 { hsv(t * 0.8 + b.x * 0.004, 0.6, 1.0, 1.0) } else { BLIP_GOLD };
        // a shot's echoes are dots, back to the muzzle and no further
        for (back, age) in smear_echoes(blip, g.sess.level).filter(|e| e.0 < BULLET_TTL - b.ttl) {
            let (ex, ey) = (b.x - b.vx * back, b.y - b.vy * back);
            // a shot's own echoes follow it round the wrap
            let (ex, ey) = if b.from_player {
                (wrap(ex, 0.0, PLAY_W as f32), wrap(ey, PLAY_Y0, PLAY_Y0 + PLAY_H as f32))
            } else { (ex, ey) };
            blip.fill_circle(ex, ey, 1.0, BlipColor { a: SMEAR_ALPHA * age, ..c });
        }
        if b.from_player {
            blip.fill_circle(b.x, b.y, 3.5, BLIP_INK);
            blip.fill_circle(b.x, b.y, 2.0, c);
        } else {
            // The saucer's shot is the one thing here that kills from afar:
            // bigger, with a cream heart, so it reads over the haze.
            blip.fill_circle(b.x, b.y, 5.0, BLIP_INK);
            blip.fill_circle(b.x, b.y, 3.5, BLIP_HEART);
            blip.fill_circle(b.x, b.y, 1.5, BLIP_CREAM);
        }
    }
    if g.ship_alive {
        draw_ship(blip, &g.ship, g.invuln_t, g.sess.level, BLIP_CREAM);
    }
    blip.draw_hud(g.sess.score, g.sess.lives);
    blip.draw_hud_multiple(g.sess.score, g.streak.multiple());
    // x=12 clears the shell's 3px canvas clip, which eats more of this wider canvas's units.
    let lvl = format!("LEVEL {}", g.sess.level);
    blip.draw_text_outlined(&lvl, 12.0, WIN_H as f32 - 22.0, 2.0, BLIP_CREAM, BLIP_INK);
}

pub(super) fn draw_title(blip: &Blip, art: &[Texture2D; 3]) {
    blip.clear(BLIP_NIGHT);
    draw_stars(blip, blip::now());
    blip.draw_logo("METEORS", (WIN_W / 2) as f32, (WIN_H / 4) as f32, 6.0, BLIP_PLUM);
    // METEORS is sz=6 (42px tall) — clear its bottom by a real margin.
    blip.draw_attract_bar();
    // No touch surface here: a phone plays on the pad.
    let keys = web::controls() == web::Controls::Keys;
    blip.draw_start_prompt((WIN_H / 2) as f32);
    let (l1, l2) = if keys { ("ARROWS/WASD ROTATE+THRUST", "SPACE FIRE  ·  Z HYPERSPACE") }
        else { ("PAD TURNS   UP THRUSTS", "BUTTONS FIRE AND HYPERSPACE") };
    blip.draw_panel(WIN_W as f32 * 0.12, (WIN_H * 2 / 3) as f32 - 14.0, WIN_W as f32 * 0.76, 66.0, 1.0);
    blip.draw_centered(l1, (WIN_H * 2 / 3) as f32, 2.0, BLIP_CREAM);
    blip.draw_centered(l2, (WIN_H * 2 / 3) as f32 + 24.0, 2.0, BLIP_CREAM);
    let t = blip::now();
    blip.draw_texture_cell(&art[0], 0, 1, 0, 1, 72.0, 558.0, 96.0, 96.0, t * 0.16);
    blip.draw_texture_cell(&art[1], 0, 1, 0, 1, 528.0, 572.0, 64.0, 64.0, -t * 0.22);
    blip.draw_texture_cell(&art[2], 0, 1, 0, 1,
        316.0 + (t * 0.5).sin() * 12.0, 584.0, 48.0, 48.0, (t * 0.35).sin() * 0.12);
}

pub(super) fn draw_over(blip: &Blip, score: i32, hi: i32, waiting: bool) {
    blip.clear(BLIP_NIGHT);
    draw_stars(blip, blip::now());
    blip.draw_game_over(score, hi, BLIP_HEART, BLIP_SEA, BLIP_GOLD, !waiting);
}
