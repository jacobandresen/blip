//! The temple: night under its roof, lit by its lanterns, the mountain behind.

use super::*;

/// THE TEMPLE: night under a roof, the mountain behind. Built like the docks;
/// the light comes from the lanterns.
pub(crate) fn draw_temple(blip: &Blip, shake: f32, t: f32, roar: Roar) {
    const HORIZON: f32 = 262.0;
    let sky_at = |k: f32| BlipColor {
        r: 0.05 + 0.13 * k, g: 0.06 + 0.13 * k, b: 0.13 + 0.17 * k, a: 1.0,
    };
    blip.fill_bands(0.0, shake, WIN_W as f32, HORIZON, 14, sky_at);
    let low = sky_at(1.0);
    blip.fill_rect(0.0, HORIZON + shake, WIN_W as f32, FLOOR_Y - HORIZON, low);

    for i in 0..52 {
        let sx = scatter(i, 0x2a);
        let x = sx * WIN_W as f32;
        let y = scatter(i, 0x2b) * 210.0 + 8.0 + shake;
        let tw = 0.5 + 0.5 * ((t * 1.7 + sx * 30.0).sin());
        let r = 0.8 + scatter(i, 0x2c) * 1.2;
        blip.fill_circle(x, y, r,
            BlipColor { r: 0.9, g: 0.95, b: 1.0, a: 0.20 + 0.40 * tw });
    }

    // Moon, with its glow and the cloud that drifts over it.
    let (mx, my) = (143.0, 92.0 + shake);
    for r in [46.0f32, 34.0] {
        blip.fill_circle(mx, my, r, BlipColor { r: 0.75, g: 0.82, b: 0.95, a: 0.10 });
    }
    blip.fill_circle(mx, my, 26.0, BlipColor { r: 0.92, g: 0.94, b: 0.86, a: 1.0 });
    blip.fill_circle(mx - 10.0, my - 7.0, 23.0, sky_at(0.28));
    for i in 0..5 {
        let sx = scatter(i, 0x40);
        let y = 60.0 + sx * 96.0 + shake;
        let w = 110.0 + scatter(i, 0x41) * 170.0;
        let x = (t * (2.0 + sx * 3.0) + sx * 800.0) % (WIN_W as f32 + 300.0) - 150.0;
        blip.fill_rect(x, y, w, 6.0, BlipColor { r: 0.20, g: 0.21, b: 0.31, a: 0.7 });
    }

    // Mountains, in two ranges so there is a distance between them.
    for (base, amp, k, step) in [(HORIZON - 52.0, 66.0f32, 0.66f32, 128.0f32),
                                 (HORIZON - 26.0, 44.0, 0.44, 96.0)] {
        let c = hazed(BlipColor { r: 0.10, g: 0.11, b: 0.20, a: 1.0 }, low, k);
        let n = (WIN_W as f32 / step) as usize + 2;
        for i in 0..n {
            let x = i as f32 * step - 40.0;
            let pk = amp * (0.55 + 0.45 * scatter(i, if k > 0.5 { 0x51 } else { 0x52 }));
            // A peak drawn as a stack of shrinking bars: at this size a
            // triangle and a staircase are the same picture.
            let rows = 9;
            for r in 0..rows {
                let f = r as f32 / rows as f32;
                let w = step * 0.95 * (1.0 - f);
                blip.fill_rect(x + step * 0.75 - w * 0.5, base + shake - pk * f - pk / rows as f32,
                    w, pk / rows as f32 + 1.0, c);
            }
        }
    }

    draw_flyby(blip, 112.0 + shake, t);

    // The roof over the whole stage, and the rafters holding it up.
    let stone = BlipColor { r: 0.19, g: 0.18, b: 0.24, a: 1.0 };
    let lit = BlipColor { r: 0.31, g: 0.29, b: 0.37, a: 1.0 };
    let tile = BlipColor { r: 0.14, g: 0.13, b: 0.19, a: 1.0 };
    blip.fill_rect(0.0, 30.0 + shake, WIN_W as f32, 20.0, tile);
    for i in 0..27 {
        blip.fill_rect(i as f32 * 24.0, 30.0 + shake, 20.0, 20.0, shade(tile, 1.3));
        blip.fill_rect(i as f32 * 24.0, 46.0 + shake, 20.0, 4.0, shade(tile, 0.7));
    }
    blip.fill_rect(0.0, 50.0 + shake, WIN_W as f32, 7.0, lit);
    for i in 0..14 {
        blip.fill_rect(10.0 + i as f32 * 46.0, 57.0 + shake, 12.0, 16.0, stone);
    }

    // Colonnade, and the onlookers standing between the pillars.
    draw_crowd(blip, HORIZON + 6.0 + shake, 32.0,
        BlipColor { r: 0.20, g: 0.20, b: 0.29, a: 1.0 }, t, 0x77a, 30, roar);
    for i in 0..5 {
        let x = 22.0 + i as f32 * 146.0;
        blip.fill_rect(x, 57.0 + shake, 30.0, HORIZON - 45.0, stone);
        blip.fill_rect(x, 57.0 + shake, 7.0, HORIZON - 45.0, lit);
        // Capital and base, which is what stops a pillar being a bar.
        blip.fill_rect(x - 5.0, 57.0 + shake, 40.0, 9.0, lit);
        blip.fill_rect(x - 5.0, HORIZON + 4.0 + shake, 40.0, 10.0, lit);
        blip.fill_rect(x - 3.0, HORIZON + 14.0 + shake, 36.0, 6.0, shade(stone, 0.8));
    }

    // Banners between the pillars, and the lanterns that light them.
    for i in 0..4 {
        let x = 96.0 + i as f32 * 146.0;
        let sway = (t * 1.1 + i as f32).sin() * 3.0;
        blip.fill_rect(x + sway, 73.0 + shake, 24.0, 118.0,
            BlipColor { r: 0.60, g: 0.11, b: 0.16, a: 1.0 });
        blip.fill_rect(x + 8.0 + sway, 95.0 + shake, 8.0, 62.0,
            BlipColor { r: 0.95, g: 0.85, b: 0.5, a: 0.9 });
        blip.fill_rect(x - 2.0 + sway, 73.0 + shake, 28.0, 5.0,
            BlipColor { r: 0.30, g: 0.26, b: 0.20, a: 1.0 });
    }
    for i in 0..5 {
        let x = 22.0 + i as f32 * 146.0 + 15.0;
        let sway = (t * 1.4 + i as f32 * 1.9).sin() * 2.2;
        let y = 82.0 + shake;
        blip.draw_line(x, 66.0 + shake, x + sway, y, BlipColor { r: 0.25, g: 0.22, b: 0.18, a: 1.0 });
        let flick = 0.86 + 0.14 * ((t * 9.0 + i as f32 * 2.1).sin());
        blip.fill_glow_circle(x + sway, y + 9.0, 22.0,
            BlipColor { r: 1.0, g: 0.72, b: 0.32, a: 0.16 * flick });
        blip.fill_circle(x + sway, y + 9.0, 8.0,
            BlipColor { r: 0.95, g: 0.40, b: 0.25, a: 1.0 });
        blip.fill_circle(x + sway, y + 8.0, 5.0,
            BlipColor { r: 1.0, g: 0.85 * flick, b: 0.5, a: 1.0 });
    }

    // The terrace wall under the colonnade, down to the floor.
    let wall = BlipColor { r: 0.15, g: 0.15, b: 0.20, a: 1.0 };
    blip.fill_rect(0.0, HORIZON + 20.0 + shake, WIN_W as f32, FLOOR_Y - HORIZON - 20.0, wall);
    for i in 0..13 {
        let x = i as f32 * 50.0;
        blip.fill_rect(x + 2.0, HORIZON + 24.0 + shake, 46.0, 20.0, shade(wall, 1.25));
        blip.fill_rect(x + 27.0, HORIZON + 46.0 + shake, 46.0, 20.0, shade(wall, 1.12));
    }
    blip.fill_rect(0.0, HORIZON + 20.0 + shake, WIN_W as f32, 3.0, lit);
}
