//! The river village: a jetty on a slow green river, a hut on stilts, the jungle, and something long and green in the
//! big tree.

use super::*;

/// THE RIVER VILLAGE: a jetty on a slow green river, a hut on stilts, the
/// jungle behind and something long and green in the big tree.
pub(crate) fn draw_village(blip: &Blip, shake: f32, t: f32, roar: Roar) {
    const HORIZON: f32 = 250.0;
    let c = |r: f32, g: f32, b: f32| BlipColor { r, g, b, a: 1.0 };
    let low = c(0.94, 0.93, 0.70);
    draw_sky(blip, shake, HORIZON, c(0.50, 0.78, 0.76), low);

    // Jungle in two ranges of rounded crowns, and palms above them.
    for (base, r0, salt, k, n) in [(HORIZON - 26.0, 22.0f32, 0xc1u32, 0.50f32, 22usize),
                                   (HORIZON - 6.0, 28.0, 0xc2, 0.22, 16)] {
        let leaf = hazed(c(0.12, 0.32, 0.18), low, k);
        for i in 0..n {
            let x = (i as f32 + scatter(i, salt)) * WIN_W as f32 / (n - 1) as f32;
            blip.fill_circle(x, base + shake, r0 + scatter(i, salt ^ 9) * 16.0, leaf);
        }
        blip.fill_rect(0.0, base + shake, WIN_W as f32, HORIZON - base + 8.0, leaf);
    }
    let palm = hazed(c(0.10, 0.26, 0.16), low, 0.30);
    for (x, top) in [(232.0f32, 120.0f32), (402.0, 96.0), (470.0, 138.0)] {
        let sway = (t * 0.8 + x).sin() * 3.0;
        blip.draw_line_ex(x, HORIZON - 20.0 + shake, x + sway, top + shake, 4.0, palm);
        for a in [-2.6f32, -2.0, -1.2, -0.5, 0.1] {
            stroke(blip, V(x + sway, top + shake), V(x + sway + a.cos() * 34.0, top - 6.0 + a.sin().abs() * 20.0 + shake),
                3.0, 1.0, palm);
        }
    }

    // The far bank and the villagers on it; the river, slow and green.
    draw_crowd(blip, HORIZON + 8.0 + shake, 28.0, hazed(c(0.24, 0.18, 0.12), low, 0.12), t, 0xc5d, 28, roar);
    blip.fill_rect(0.0, HORIZON + 4.0 + shake, WIN_W as f32, 10.0, c(0.46, 0.36, 0.22));
    let river = c(0.30, 0.42, 0.32);
    blip.fill_rect(0.0, HORIZON + 14.0 + shake, WIN_W as f32, FLOOR_Y - HORIZON - 14.0, river);
    for i in 0..20 {
        let sx = scatter(i, 0xc7);
        let y = HORIZON + 20.0 + sx * 50.0 + shake;
        let x = ((i as f32 * 67.0) + t * (5.0 + sx * 7.0)) % (WIN_W as f32 + 60.0) - 30.0;
        blip.fill_rect(x, y, 16.0 + sx * 22.0, 1.5, BlipColor { r: 0.80, g: 0.90, b: 0.70, a: 0.20 });
    }

    // A hut on stilts, upstream.
    let (hx, hy) = (36.0, HORIZON - 30.0 + shake);
    for sx in [8.0f32, 44.0, 84.0, 118.0] { blip.draw_line_ex(hx + sx, hy + 44.0, hx + sx, HORIZON + 30.0 + shake, 3.0, c(0.30, 0.20, 0.12)); }
    blip.fill_rect(hx, hy, 126.0, 46.0, c(0.52, 0.36, 0.20));
    blip.fill_rect(hx + 50.0, hy + 14.0, 24.0, 32.0, c(0.16, 0.10, 0.08));
    for r in 0..6 {
        let f = r as f32 / 6.0;
        blip.fill_rect(hx - 14.0 + f * 60.0, hy - 8.0 - r as f32 * 8.0, 154.0 - f * 120.0, 9.0,
            shade(c(0.72, 0.58, 0.30), 1.0 - 0.06 * (r % 2) as f32));
    }

    // The big tree at the water's edge, its vines, and the snake asleep on a
    // branch: only its tail moves.
    let bark = c(0.28, 0.19, 0.12);
    blip.fill_rect(566.0, shake, 50.0, FLOOR_Y - 30.0, bark);
    blip.fill_rect(566.0, shake, 10.0, FLOOR_Y - 30.0, blend(bark, BLIP_WHITE, 0.12));
    stroke(blip, V(580.0, 108.0 + shake), V(470.0, 84.0 + shake), 10.0, 5.0, bark);
    for (x, y, r) in [(560.0f32, 26.0f32, 46.0f32), (610.0, 50.0, 40.0), (500.0, 40.0, 34.0), (640.0, 10.0, 40.0)] {
        blip.fill_circle(x, y + shake, r, c(0.14, 0.36, 0.20));
    }
    for (x, len) in [(520.0f32, 70.0f32), (548.0, 104.0), (626.0, 90.0)] {
        blip.draw_line(x, 60.0 + shake, x + (t * 0.9 + x).sin() * 3.0, 60.0 + len + shake, c(0.20, 0.44, 0.24));
    }
    let scale = c(0.36, 0.62, 0.22);
    let mut at = V(540.0, 92.0 + shake);
    for i in 0..9 {
        let next = V(at.0 - 9.0, 86.0 + shake + if i % 2 == 0 { 9.0 } else { -3.0 });
        stroke(blip, at, next, 4.0, 4.0, scale);
        at = next;
    }
    stroke(blip, at, V(at.0 - 4.0, at.1 + 18.0 + (t * 1.6).sin() * 4.0), 3.0, 1.0, scale);
    blip.fill_circle(544.0, 90.0 + shake, 5.5, scale);
    blip.fill_circle(546.0, 88.0 + shake, 1.3, c(0.96, 0.90, 0.30));

    // The jetty's edge: posts and a rope.
    let post = c(0.40, 0.28, 0.16);
    blip.fill_rect(0.0, FLOOR_Y - 30.0 + shake, WIN_W as f32, 30.0, post);
    blip.fill_rect(0.0, FLOOR_Y - 32.0 + shake, WIN_W as f32, 3.0, blend(post, BLIP_WHITE, 0.22));
    for i in 0..22 { blip.draw_line(i as f32 * 30.0, FLOOR_Y - 29.0 + shake, i as f32 * 30.0, FLOOR_Y + shake, shade(post, 0.72)); }
    blip.fill_rect(0.0, FLOOR_Y - 16.0 + shake, WIN_W as f32, 2.0, c(0.74, 0.66, 0.46));
}
