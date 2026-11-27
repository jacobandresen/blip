//! The Queens rooftop: dusk over the river, the city lit up across it, the elevated train, the neighbours on the
//! next roof.

use super::*;

/// QUEENS ROOFTOP: dusk over the river, the city lit up across it, the
/// elevated train going by, and the neighbours watching from the next roof.
/// The brick parapet behind the fighters is the quiet band.
pub(crate) fn draw_rooftop(blip: &Blip, shake: f32, t: f32, roar: Roar) {
    const HORIZON: f32 = 214.0;
    let c = |r: f32, g: f32, b: f32| BlipColor { r, g, b, a: 1.0 };
    let low = c(0.96, 0.60, 0.36);
    draw_sky(blip, shake, HORIZON, c(0.16, 0.14, 0.36), low);
    blip.fill_circle(120.0, HORIZON - 16.0 + shake, 26.0, c(1.0, 0.82, 0.50));

    // The skyline across the river: towers in two rows, windows coming on.
    for (row, (salt, tone)) in [(0xb1u32, 0.50f32), (0xb5, 0.0)].into_iter().enumerate() {
        let body = hazed(c(0.16, 0.13, 0.26), low, tone);
        let n = 20 - row * 4;
        for i in 0..n {
            let w = WIN_W as f32 / n as f32;
            let x = i as f32 * w;
            let h = 28.0 + scatter(i, salt) * (58.0 + 46.0 * row as f32)
                + if scatter(i, salt ^ 9) > 0.86 { 46.0 } else { 0.0 };
            let top = HORIZON - h + shake;
            blip.fill_rect(x, top, w - 2.0, h + 2.0, body);
            // A spire on the tall ones.
            if h > 110.0 { blip.fill_rect(x + w / 2.0 - 1.5, top - 20.0, 3.0, 20.0, body); }
            if row == 0 { continue; }
            let mut wy = top + 6.0;
            let mut k = 0;
            while wy < HORIZON - 6.0 + shake {
                for col in 0..3 {
                    k += 1;
                    if scatter(i * 97 + k, salt ^ 0x33) > 0.55 {
                        blip.fill_rect(x + 4.0 + col as f32 * (w - 10.0) / 3.0, wy, 3.0, 4.0,
                            c(1.0, 0.86, 0.48));
                    }
                }
                wy += 10.0;
            }
        }
    }

    // The steel globe of the old world's fair, and a red sign on its frame.
    let steel = c(0.86, 0.90, 0.96);
    let (gx, gy) = (286.0, HORIZON - 15.0 + shake);
    blip.fill_circle(gx, gy, 14.0, steel);
    blip.fill_circle(gx, gy, 12.2, c(0.22, 0.20, 0.36));
    for k in [-1.0f32, 0.0, 1.0] {
        stroke(blip, V(gx - 12.0, gy + k * 6.0), V(gx + 12.0, gy + k * 6.0), 0.7, 0.7, steel);
        stroke(blip, V(gx + k * 6.0, gy - 12.0), V(gx + k * 6.0, gy + 12.0), 0.7, 0.7, steel);
    }
    stroke(blip, V(gx - 19.0, gy + 7.0), V(gx + 19.0, gy - 9.0), 0.8, 0.8, steel);
    // Neon: it drops out for a blink every couple of seconds.
    let lit = if (t * 0.5).fract() < 0.92 { c(1.0, 0.36, 0.30) } else { c(0.50, 0.14, 0.14) };
    let sy = HORIZON - 118.0 + shake;
    for sx in [182.0f32, 216.0, 250.0] { blip.fill_rect(sx, sy + 18.0, 1.5, 34.0, c(0.10, 0.08, 0.14)); }
    blip.fill_rect(174.0, sy - 4.0, 84.0, 22.0, c(0.10, 0.07, 0.12));
    blip.draw_text("QUEENS", 180.0, sy, 2.0, lit);

    let poster = V(500.0, 112.0 + shake);
    blip.fill_rect(poster.0 - 82.5, poster.1 - 46.0, 165.0, 92.0, c(0.08, 0.06, 0.14));
    FILM_POSTER.with(|art| blip.draw_texture(art, poster.0 - 82.5, poster.1 - 46.0, 165.0, 92.0));
    blip.draw_rect(poster.0 - 82.5, poster.1 - 46.0, 165.0, 92.0, c(0.78, 0.28, 0.42));

    // The river, and the bridge over it: a cantilever in steel, two towers
    // with their spikes and the trusses hung between and beyond them.
    blip.fill_rect(0.0, HORIZON + shake, WIN_W as f32, 14.0, c(0.34, 0.26, 0.42));
    let iron = c(0.20, 0.14, 0.22);
    // Painted steel with the last of the sun on it, or it is lost against
    // the towers behind.
    let span = c(0.80, 0.62, 0.56);
    let deck = HORIZON - 8.0 + shake;
    blip.fill_rect(330.0, deck, 310.0, 4.0, span);
    blip.fill_rect(330.0, deck - 9.0, 310.0, 1.5, span);
    // The top chord: up to each tower, down between them.
    let chord = [(330.0f32, 12.0f32), (400.0, 46.0), (480.0, 18.0), (560.0, 46.0), (640.0, 12.0)];
    for w in chord.windows(2) {
        let (a, b) = (w[0], w[1]);
        stroke(blip, V(a.0, deck - a.1), V(b.0, deck - b.1), 1.3, 1.3, span);
        // The web under it: posts and braces, a panel every sixteen pixels.
        let n = ((b.0 - a.0) / 16.0) as i32;
        for k in 0..n {
            let (u0, u1) = (k as f32 / n as f32, (k + 1) as f32 / n as f32);
            let (x0, x1) = (a.0 + (b.0 - a.0) * u0, a.0 + (b.0 - a.0) * u1);
            let (h0, h1) = (a.1 + (b.1 - a.1) * u0, a.1 + (b.1 - a.1) * u1);
            blip.fill_rect(x0, deck - h0, 1.2, h0, span);
            stroke(blip, V(x0, deck), V(x1, deck - h1), 0.6, 0.6, span);
        }
    }
    for tx in [400.0f32, 560.0] {
        blip.fill_rect(tx - 4.0, deck - 46.0, 8.0, 60.0, span);
        for sp in [-3.0f32, 0.0, 3.0] { blip.fill_rect(tx + sp - 0.75, deck - 54.0, 1.5, 9.0, span); }
    }

    // The elevated train, now and then, along its viaduct.
    let rail = HORIZON + 18.0 + shake;
    blip.fill_rect(0.0, rail, WIN_W as f32, 5.0, iron);
    for i in 0..11 { blip.fill_rect(20.0 + i as f32 * 60.0, rail, 6.0, 22.0, iron); }
    if let Some(x) = cameo_x(t, 5.0) {
        for car in 0..4 {
            let cx = x - car as f32 * 64.0;
            blip.fill_rect(cx, rail - 18.0, 60.0, 18.0, c(0.66, 0.68, 0.72));
            blip.fill_rect(cx, rail - 6.0, 60.0, 2.0, c(0.60, 0.20, 0.50));
            for w in 0..5 { blip.fill_rect(cx + 5.0 + w as f32 * 11.0, rail - 15.0, 8.0, 7.0, c(1.0, 0.90, 0.56)); }
        }
    }

    // The next roof: a water tower, washing on a line, the neighbours.
    let roof = HORIZON + 40.0 + shake;
    let wood = c(0.42, 0.26, 0.18);
    for lx in [66.0f32, 86.0, 106.0] { blip.fill_rect(lx, roof - 34.0, 3.0, 34.0, iron); }
    blip.fill_rect(58.0, roof - 78.0, 60.0, 46.0, wood);
    for b in 0..3 { blip.fill_rect(58.0, roof - 70.0 + b as f32 * 14.0, 60.0, 2.0, shade(wood, 0.6)); }
    stroke(blip, V(60.0, roof - 78.0), V(88.0, roof - 98.0), 1.0, 1.0, shade(wood, 0.7));
    for k in 0..15 {
        let y = roof - 78.0 - k as f32 * 1.4;
        let half = 30.0 - k as f32 * 2.0;
        blip.fill_rect(88.0 - half, y, half * 2.0, 1.6, shade(wood, 0.7));
    }
    blip.fill_rect(470.0, roof - 44.0, 2.0, 44.0, iron);
    blip.fill_rect(600.0, roof - 44.0, 2.0, 44.0, iron);
    blip.fill_rect(470.0, roof - 42.0, 132.0, 1.0, iron);
    for (i, col) in [c(0.90, 0.90, 0.94), c(0.80, 0.26, 0.24), c(0.30, 0.44, 0.78), c(0.94, 0.80, 0.36)]
        .into_iter().enumerate() {
        let sway = (t * 1.8 + i as f32).sin() * 1.5;
        blip.fill_rect(484.0 + i as f32 * 28.0 + sway, roof - 41.0, 16.0, 18.0 + (i % 2) as f32 * 6.0, col);
    }
    draw_crowd(blip, roof + 2.0, 26.0, hazed(c(0.20, 0.14, 0.22), low, 0.10), t, 0xb9d, 26, roar);

    // The parapet: brick, with a stone cap, right across behind the fight.
    let brick = c(0.50, 0.24, 0.20);
    blip.fill_rect(0.0, roof, WIN_W as f32, FLOOR_Y - roof + shake.abs() + 1.0, brick);
    for r in 0..9 {
        let y = roof + 8.0 + r as f32 * 9.0;
        if y > FLOOR_Y + shake { break; }
        blip.fill_rect(0.0, y, WIN_W as f32, 1.0, shade(brick, 0.72));
        for i in 0..18 {
            blip.fill_rect(i as f32 * 38.0 + (r % 2) as f32 * 19.0, y - 8.0, 1.0, 8.0, shade(brick, 0.72));
        }
    }
    blip.fill_rect(0.0, roof - 2.0, WIN_W as f32, 7.0, c(0.74, 0.70, 0.66));
    blip.fill_rect(0.0, roof + 5.0, WIN_W as f32, 2.0, shade(brick, 0.6));

    // A vent and an aerial at either edge of this roof.
    blip.fill_rect(8.0, FLOOR_Y - 40.0 + shake, 34.0, 40.0, c(0.46, 0.48, 0.52));
    blip.fill_rect(4.0, FLOOR_Y - 46.0 + shake, 42.0, 8.0, c(0.58, 0.60, 0.64));
    for v in 0..4 { blip.fill_rect(12.0, FLOOR_Y - 32.0 + v as f32 * 7.0 + shake, 26.0, 2.0, c(0.24, 0.24, 0.28)); }
    blip.fill_rect(612.0, FLOOR_Y - 96.0 + shake, 2.5, 96.0, iron);
    for (y, w) in [(90.0f32, 26.0f32), (78.0, 18.0), (66.0, 12.0)] {
        blip.fill_rect(613.0 - w / 2.0, FLOOR_Y - y + shake, w, 2.0, iron);
    }
}
