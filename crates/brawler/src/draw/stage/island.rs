//! The island: a karate tournament's grounds at noon, yellow banners on the fortress wall, the host on his dais and the
//! entrants in white.

use super::*;

/// THE ISLAND: a karate tournament's grounds at noon, after Enter the Dragon's: hills and palms, a fortress wall
/// hung with yellow banners, the host on a canopied dais, and ranks of entrants in white along the wall.
pub(crate) fn draw_island(blip: &Blip, shake: f32, t: f32, roar: Roar) {
    const HORIZON: f32 = 250.0;
    let c = |r, g, b| BlipColor { r, g, b, a: 1.0 };
    let sky_at = |k: f32| BlipColor { r: 0.46 + 0.36 * k, g: 0.66 + 0.24 * k, b: 0.92 - 0.02 * k, a: 1.0 };
    blip.fill_bands(0.0, shake, WIN_W as f32, HORIZON, 14, sky_at);
    let low = sky_at(1.0);
    blip.fill_rect(0.0, HORIZON + shake, WIN_W as f32, FLOOR_Y - HORIZON, low);
    // Clouds drifting.
    for i in 0..4 {
        let sx = scatter(i, 0x9a1);
        let x = (t * (3.0 + sx * 3.0) + sx * 700.0) % (WIN_W as f32 + 200.0) - 100.0;
        let y = 70.0 + sx * 60.0 + shake;
        for (dx, r) in [(0.0f32, 14.0f32), (16.0, 18.0), (34.0, 13.0)] {
            blip.fill_circle(x + dx, y, r, BlipColor { r: 1.0, g: 1.0, b: 1.0, a: 0.85 });
        }
    }
    // Green hills behind the wall.
    for (base, amp, k, step, salt) in [(HORIZON - 30.0, 70.0f32, 0.45f32, 150.0f32, 0x9a2), (HORIZON - 10.0, 46.0, 0.20, 110.0, 0x9a3)] {
        let col = hazed(c(0.20, 0.42, 0.22), low, k);
        let n = (WIN_W as f32 / step) as usize + 2;
        for i in 0..n {
            let x = i as f32 * step - 40.0;
            let pk = amp * (0.6 + 0.4 * scatter(i, salt));
            for r in 0..10 {
                let f = r as f32 / 10.0;
                let w = step * 1.1 * (1.0 - f * f);
                blip.fill_rect(x + step * 0.6 - w * 0.5, base + shake - pk * f - pk / 10.0, w, pk / 10.0 + 1.0, col);
            }
        }
    }
    // Palms either side, swaying a little.
    for (x, hgt, lean) in [(44.0f32, 120.0f32, 0.18f32), (96.0, 96.0, -0.10), (560.0, 112.0, -0.16), (606.0, 92.0, 0.12)] {
        let trunk = c(0.42, 0.32, 0.20);
        let sway = (t * 0.9 + x).sin() * 2.0;
        let top = V(x + lean * hgt + sway, HORIZON - hgt + shake);
        stroke(blip, V(x, HORIZON + 6.0 + shake), top, 3.0, 2.0, trunk);
        let leaf = c(0.18, 0.46, 0.20);
        for k in 0..6 {
            let a = k as f32 / 6.0 * std::f32::consts::TAU + 0.3;
            let tip = V(top.0 + a.cos() * 26.0, top.1 + a.sin() * 10.0 + 8.0);
            stroke(blip, top, tip, 2.6, 1.0, leaf);
        }
    }
    // The fortress wall, grey stone in courses, crenellated along the top.
    let stone = c(0.58, 0.56, 0.52);
    let top = HORIZON - 64.0 + shake;
    blip.fill_rect(120.0, top, 400.0, HORIZON + 10.0 + shake - top, stone);
    for i in 0..20 { if i % 2 == 0 { blip.fill_rect(120.0 + i as f32 * 20.0, top - 10.0, 20.0, 10.0, stone); } }
    for course in 0..6 {
        let y = top + 4.0 + course as f32 * 12.0;
        blip.fill_rect(120.0, y, 400.0, 1.0, shade(stone, 0.75));
        let off = if course % 2 == 0 { 0.0 } else { 15.0 };
        for i in 0..14 { blip.fill_rect(124.0 + off + i as f32 * 30.0, y, 1.0, 12.0, shade(stone, 0.80)); }
    }
    // The host's dais in the middle, under a red canopy: a dark figure on a carved chair.
    let (dx, dy) = (320.0, HORIZON - 4.0 + shake);
    blip.fill_rect(dx - 46.0, dy - 8.0, 92.0, 12.0, c(0.46, 0.20, 0.14));
    blip.fill_rect(dx - 40.0, dy - 70.0, 80.0, 8.0, c(0.74, 0.12, 0.10));
    for side in [-1.0f32, 1.0] { blip.fill_rect(dx + side * 36.0 - 2.0, dy - 64.0, 4.0, 58.0, c(0.30, 0.16, 0.10)); }
    blip.fill_rect(dx - 12.0, dy - 44.0, 24.0, 36.0, c(0.66, 0.50, 0.18));
    blip.fill_circle(dx, dy - 34.0, 5.0, c(0.80, 0.64, 0.48));
    blip.fill_rect(dx - 7.0, dy - 28.0, 14.0, 20.0, c(0.10, 0.10, 0.12));
    // Yellow banners along the wall, a black device on each, rippling.
    for i in 0..8 {
        let x = 132.0 + i as f32 * 52.0;
        if (x - dx).abs() < 54.0 { continue; }
        let pole = c(0.30, 0.22, 0.14);
        blip.fill_rect(x, top - 40.0, 2.0, 60.0, pole);
        let ripple = (t * 3.0 + i as f32).sin() * 2.0;
        blip.fill_rect(x + 2.0, top - 38.0 + ripple * 0.3, 18.0, 24.0, c(0.98, 0.82, 0.14));
        blip.fill_circle(x + 11.0, top - 26.0 + ripple * 0.3, 4.0, c(0.10, 0.10, 0.10));
    }
    // The entrants in their ranks along the foot of the wall.
    draw_crowd(blip, HORIZON + 10.0 + shake, 24.0, hazed(c(0.86, 0.86, 0.84), low, 0.10), t, 0xe17, 30, roar);
    // The courtyard: sandy ground from the wall down to the fighting floor, darker where it is trodden.
    let sand = c(0.80, 0.70, 0.50);
    blip.fill_rect(0.0, HORIZON + 10.0 + shake, WIN_W as f32, FLOOR_Y - HORIZON - 10.0, sand);
    for i in 0..30usize {
        let x = scatter(i, 0x9a7) * WIN_W as f32;
        let y = HORIZON + 14.0 + scatter(i, 0x9a8) * (FLOOR_Y - HORIZON - 18.0) + shake;
        blip.fill_rect(x, y, 8.0 + 14.0 * scatter(i, 0x9a9), 1.0, shade(sand, 0.86));
    }
}
