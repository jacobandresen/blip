//! The pagoda, DRAKE's home: red and gold at dusk, the moon over the mountains, a moon gate, lanterns and the wooden
//! dummy; the school's students bow to the fighters as a round opens.

use super::*;

/// THE PAGODA: DRAKE's home, a nod to the five-storey tower of Game of Death: a pagoda of red and gold at dusk,
/// the moon over the mountains, a courtyard wall with a moon gate, lanterns on a line, an incense burner smoking
/// and a wooden dummy by the gate.
pub(crate) fn draw_pagoda(blip: &Blip, shake: f32, t: f32, roar: Roar) {
    const HORIZON: f32 = 258.0;
    let c = |r, g, b| BlipColor { r, g, b, a: 1.0 };
    // Dusk: indigo overhead, the last of the light red along the mountains.
    let sky_at = |k: f32| BlipColor { r: 0.06 + 0.40 * k * k, g: 0.05 + 0.15 * k * k, b: 0.15 + 0.12 * k, a: 1.0 };
    blip.fill_bands(0.0, shake, WIN_W as f32, HORIZON, 16, sky_at);
    let low = sky_at(1.0);
    blip.fill_rect(0.0, HORIZON + shake, WIN_W as f32, FLOOR_Y - HORIZON, low);
    for i in 0..30 {
        let x = scatter(i, 0x8a) * WIN_W as f32;
        let y = scatter(i, 0x8b) * 120.0 + 6.0 + shake;
        let tw = 0.5 + 0.5 * (t * 1.3 + i as f32).sin();
        blip.fill_rect(x, y, 1.0, 1.0, BlipColor { r: 1.0, g: 0.95, b: 0.85, a: 0.25 + 0.5 * tw });
    }
    // A big low moon, hazed gold.
    let (mx, my) = (508.0, 118.0 + shake);
    for r in [62.0f32, 48.0] { blip.fill_circle(mx, my, r, BlipColor { r: 1.0, g: 0.86, b: 0.62, a: 0.10 }); }
    blip.fill_circle(mx, my, 34.0, c(0.99, 0.90, 0.68));
    blip.fill_circle(mx + 9.0, my - 6.0, 6.0, c(0.93, 0.82, 0.60));
    blip.fill_circle(mx - 12.0, my + 9.0, 4.0, c(0.93, 0.82, 0.60));
    // Two ranges of mountains and the mist lying between them.
    for (base, amp, k, step, salt) in [(HORIZON - 40.0, 78.0f32, 0.62f32, 140.0f32, 0x8c), (HORIZON - 12.0, 52.0, 0.40, 104.0, 0x8d)] {
        let col = hazed(c(0.12, 0.08, 0.18), low, k);
        let n = (WIN_W as f32 / step) as usize + 2;
        for i in 0..n {
            let x = i as f32 * step - 50.0;
            let pk = amp * (0.55 + 0.45 * scatter(i, salt));
            let rows = 10;
            for r in 0..rows {
                let f = r as f32 / rows as f32;
                let w = step * 0.95 * (1.0 - f);
                blip.fill_rect(x + step * 0.75 - w * 0.5, base + shake - pk * f - pk / rows as f32, w, pk / rows as f32 + 1.0, col);
            }
        }
        blip.fill_rect(0.0, base + shake - 6.0, WIN_W as f32, 5.0, BlipColor { a: 0.35, ..low });
    }
    // Pines either side, layered tiers of needles.
    for (x, hgt) in [(40.0f32, 120.0f32), (92.0, 96.0), (566.0, 112.0), (612.0, 90.0)] {
        let pine = c(0.07, 0.08, 0.10);
        blip.fill_rect(x - 2.0, HORIZON - hgt * 0.3 + shake, 4.0, hgt * 0.3, pine);
        for tier in 0..5 {
            let k = tier as f32 / 5.0;
            let w = 46.0 * (1.0 - k * 0.75);
            blip.fill_rect(x - w / 2.0, HORIZON - hgt * (0.25 + 0.15 * tier as f32) + shake, w, 12.0, pine);
        }
    }

    // The pagoda: five storeys, each narrower, red walls lit from inside, black eaves turned up at the ends
    // and edged in gold, a spire of rings on top.
    let (cx, base) = (320.0, HORIZON + 8.0 + shake);
    let wall = c(0.50, 0.10, 0.10);
    let eave = c(0.10, 0.08, 0.10);
    let gold = c(0.92, 0.70, 0.28);
    let mut y = base;
    for storey in 0..5 {
        let w = 132.0 - storey as f32 * 20.0;
        let tall = 30.0 - storey as f32 * 2.0;
        blip.fill_rect(cx - w / 2.0, y - tall, w, tall, wall);
        blip.fill_rect(cx - w / 2.0, y - tall, 5.0, tall, shade(wall, 0.7));
        blip.fill_rect(cx + w / 2.0 - 5.0, y - tall, 5.0, tall, shade(wall, 0.7));
        // Windows lit amber, one flickering now and then.
        let n = (w / 20.0) as usize;
        for k in 0..n {
            let flick = if (t * 0.7 + (storey * 7 + k) as f32).sin() > 0.95 { 0.6 } else { 1.0 };
            let (wx, wy, wh) = (cx - w / 2.0 + 10.0 + k as f32 * 20.0, y - tall + 8.0, tall - 14.0);
            blip.fill_rect(wx, wy, 8.0, wh, BlipColor { r: 1.0 * flick, g: 0.72 * flick, b: 0.34 * flick, a: 1.0 });
            // On each floor one window shows its master at practice: a shadow on the paper, stepping and striking.
            if k == (storey * 3 + 1) % n.max(1) {
                let step = (t * (1.6 + 0.3 * storey as f32) + storey as f32).sin();
                let sx = wx + 4.0 + 1.5 * step;
                let shadow = BlipColor { r: 0.30, g: 0.12, b: 0.08, a: 0.85 };
                blip.fill_circle(sx, wy + 2.5, 1.6, shadow);
                blip.fill_rect(sx - 1.5, wy + 4.0, 3.0, wh * 0.45, shadow);
                blip.fill_rect(sx - 1.5, wy + 4.0 + wh * 0.45, 1.2, wh * 0.45 - 2.0, shadow);
                blip.fill_rect(sx + 0.3, wy + 4.0 + wh * 0.45, 1.2, wh * 0.45 - 2.0, shadow);
                // An arm thrown out now and then.
                if step > 0.5 { blip.fill_rect(sx + 1.0, wy + 5.0, 3.0, 1.2, shadow); }
            }
        }
        // The eave: a wide roof slab, its ends swept up.
        let ew = w + 34.0;
        let ey = y - tall - 6.0;
        blip.fill_rect(cx - ew / 2.0, ey, ew, 7.0, eave);
        for side in [-1.0f32, 1.0] {
            let tip = V(cx + side * (ew / 2.0 + 8.0), ey - 7.0);
            stroke(blip, V(cx + side * (ew / 2.0 - 10.0), ey + 3.0), tip, 3.5, 1.5, eave);
            blip.fill_circle(tip.0, tip.1, 1.6, gold);
        }
        blip.fill_rect(cx - ew / 2.0, ey + 6.0, ew, 1.0, gold);
        y = ey;
    }
    // The spire.
    blip.fill_rect(cx - 1.5, y - 30.0, 3.0, 30.0, eave);
    for r in 0..5 { blip.fill_rect(cx - 5.0 + r as f32 * 0.6, y - 8.0 - r as f32 * 5.0, 10.0 - r as f32 * 1.2, 2.0, gold); }
    blip.fill_circle(cx, y - 33.0, 3.0, gold);
    // The students watching from the courtyard steps.
    draw_crowd(blip, HORIZON + 10.0 + shake, 26.0, hazed(c(0.12, 0.10, 0.14), low, 0.12), t, 0xd1e, 26, roar);

    // The courtyard wall: plaster under a tiled cap, the moon gate cut through it on the left.
    let plaster = c(0.62, 0.52, 0.44);
    let top = HORIZON + 16.0 + shake;
    blip.fill_rect(0.0, top, WIN_W as f32, FLOOR_Y + shake - top, plaster);
    blip.fill_rect(0.0, top - 6.0, WIN_W as f32, 7.0, c(0.16, 0.12, 0.14));
    for i in 0..33 { blip.fill_rect(i as f32 * 20.0, top - 6.0, 16.0, 3.0, c(0.26, 0.20, 0.22)); }
    blip.fill_rect(0.0, FLOOR_Y + shake - 8.0, WIN_W as f32, 8.0, shade(plaster, 0.7));
    let gate = V(170.0, top + 14.0);
    blip.fill_circle(gate.0, gate.1, 18.0, c(0.22, 0.14, 0.16));
    blip.fill_circle(gate.0, gate.1, 15.0, sky_at(0.9));
    blip.fill_rect(gate.0 - 15.0, gate.1, 30.0, FLOOR_Y + shake - gate.1, sky_at(0.9));
    // Lanterns on a line across the top of the stage, swaying out of step.
    // (Below the health bars, which cover the top 60 px.)
    let line = 104.0 + shake;
    blip.draw_line(0.0, line, WIN_W as f32, line + 6.0, c(0.15, 0.10, 0.10));
    for i in 0..7 {
        let x = 46.0 + i as f32 * 92.0;
        // A big blow sets the crowd off and the line swings with it, dying away over a second.
        let since = t - roar.at;
        let jolt = if since >= 0.0 { roar.loud * 20.0 * (-since * 2.5).exp() } else { 0.0 };
        let sway = (t * (1.2 + jolt * 0.3) + i as f32 * 1.7).sin() * (2.5 + jolt);
        let y = line + 3.0 + (x / WIN_W as f32) * 6.0;
        let flick = 0.86 + 0.14 * (t * 8.0 + i as f32 * 2.3).sin();
        blip.fill_glow_circle(x + sway, y + 12.0, 20.0, BlipColor { r: 1.0, g: 0.62, b: 0.28, a: 0.18 * flick });
        blip.fill_rect(x + sway - 6.0, y + 4.0, 12.0, 16.0, c(0.82, 0.14, 0.12));
        blip.fill_rect(x + sway - 4.0, y + 6.0, 8.0, 12.0, BlipColor { r: 1.0, g: 0.62 * flick, b: 0.30, a: 1.0 });
        blip.fill_rect(x + sway - 7.0, y + 3.0, 14.0, 2.0, gold);
        blip.fill_rect(x + sway - 7.0, y + 19.0, 14.0, 2.0, gold);
    }
    // The bronze incense burner on its legs, smoke curling up and away.
    let (bx, by) = (500.0, FLOOR_Y + shake - 10.0);
    let bronze = c(0.42, 0.30, 0.16);
    blip.fill_rect(bx - 16.0, by - 22.0, 32.0, 18.0, bronze);
    blip.fill_rect(bx - 19.0, by - 25.0, 38.0, 4.0, shade(bronze, 1.3));
    for lx in [-13.0f32, 11.0] { blip.fill_rect(bx + lx, by - 4.0, 3.0, 8.0, shade(bronze, 0.7)); }
    for k in 0..6 {
        let age = (t * 0.25 + k as f32 / 6.0).fract();
        let sx = bx + (age * 9.0).sin() * 6.0 + age * 14.0;
        blip.fill_circle(sx, by - 28.0 - age * 70.0, 2.0 + age * 6.0, BlipColor { r: 0.80, g: 0.76, b: 0.82, a: 0.35 * (1.0 - age) });
    }
    // The wooden dummy by the gate: a post, three arms and a leg.
    let (dx, dy) = (118.0, FLOOR_Y + shake - 6.0);
    let wood = c(0.40, 0.24, 0.12);
    blip.fill_rect(dx - 5.0, dy - 52.0, 10.0, 52.0, wood);
    blip.fill_rect(dx - 5.0, dy - 52.0, 3.0, 52.0, shade(wood, 1.25));
    for (ay, ang) in [(dy - 44.0, -0.25f32), (dy - 44.0, 0.25), (dy - 34.0, 0.0)] {
        let (ex, ey) = (dx + 14.0 * ang.cos().signum() * 1.0, ay + 14.0 * ang.sin());
        stroke(blip, V(dx, ay), V(ex + 4.0, ey), 1.6, 1.4, shade(wood, 0.85));
    }
    stroke(blip, V(dx, dy - 18.0), V(dx + 12.0, dy - 10.0), 2.0, 1.6, shade(wood, 0.85));
}
