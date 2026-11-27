//! The docks: the harbour at sunset, from the sky to the quay, hazier with distance.

use super::*;

/// Docks layers run from sky to quay with distance haze; keep the fighter band low-detail.
pub(crate) fn draw_dock(blip: &Blip, shake: f32, t: f32, roar: Roar) {
    const HORIZON: f32 = 250.0;
    let sky_at = |k: f32| BlipColor {
        r: 0.99 - 0.76 * k, g: 0.52 - 0.40 * k, b: 0.34 + 0.06 * k, a: 1.0,
    };
    blip.fill_bands(0.0, shake, WIN_W as f32, HORIZON, 16, |k| sky_at(1.0 - k));
    let low = sky_at(0.0);
    // Everything below the gradient, before the near layers cover it.
    // Without this the strip between the horizon and the quay is
    // whatever the frame was cleared to, which is black.
    blip.fill_rect(0.0, HORIZON + shake, WIN_W as f32, FLOOR_Y - HORIZON, low);

    // Sun, with the haze around it that a sun near the horizon has.
    let (sun_x, sun_y) = (486.0, 196.0 + shake);
    for r in [74.0f32, 58.0, 46.0] {
        blip.fill_circle(sun_x, sun_y, r,
            BlipColor { r: 1.0, g: 0.78, b: 0.42, a: 0.16 });
    }
    blip.fill_circle(sun_x, sun_y, 38.0, BlipColor { r: 1.0, g: 0.88, b: 0.55, a: 1.0 });

    // Cloud, in flat lit bars. Anything softer at this palette turns to
    // mud against the gradient behind it.
    for i in 0..7 {
        let sx = scatter(i, 0x10c);
        let y = 74.0 + sx * 120.0 + shake;
        let w = 90.0 + scatter(i, 0x20c) * 150.0;
        let x = (t * (3.0 + sx * 4.0) + sx * 900.0) % (WIN_W as f32 + 260.0) - 130.0;
        let lit = 1.0 - ((y - shake) / 200.0).clamp(0.0, 1.0);
        let c = blend(BlipColor { r: 0.62, g: 0.34, b: 0.42, a: 0.85 },
                      BlipColor { r: 1.0, g: 0.80, b: 0.58, a: 0.85 }, lit);
        blip.fill_rect(x, y, w, 7.0, c);
        blip.fill_rect(x + 22.0, y + 7.0, w * 0.6, 5.0, shade(c, 0.9));
    }

    // Far skyline: warehouses and gantries, well hazed.
    let far = hazed(BlipColor { r: 0.26, g: 0.16, b: 0.22, a: 1.0 }, low, 0.55);
    for i in 0..9 {
        let sx = scatter(i, 0x31);
        let w = 34.0 + sx * 56.0;
        let x = i as f32 * 74.0 - 20.0;
        let hh = 20.0 + scatter(i, 0x32) * 34.0;
        blip.fill_rect(x, HORIZON - hh + shake, w, hh, far);
        // Sawtooth warehouse roofs.
        for j in 0..(w as usize / 14) {
            let rx = x + j as f32 * 14.0;
            blip.fill_rect(rx, HORIZON - hh - 5.0 + shake, 8.0, 5.0, far);
        }
        // The sun catches the edge that faces it, and a few windows are lit already.
        let toward_sun = if x + w * 0.5 < sun_x { x + w - 2.0 } else { x };
        blip.fill_rect(toward_sun, HORIZON - hh + shake, 2.0, hh, blend(far, BlipColor { r: 1.0, g: 0.74, b: 0.45, a: 1.0 }, 0.35));
        for row in 0..((hh - 8.0) / 9.0) as usize {
            for col in 0..((w - 8.0) / 9.0) as usize {
                if scatter(i * 97 + row * 13 + col, 0x33) > 0.72 {
                    blip.fill_rect(x + 5.0 + col as f32 * 9.0, HORIZON - hh + 6.0 + row as f32 * 9.0 + shake, 3.0, 3.0,
                        hazed(BlipColor { r: 1.0, g: 0.80, b: 0.42, a: 1.0 }, low, 0.35));
                }
            }
        }
    }
    // Gantry cranes: a tower, a jib out over the water, and the cable
    // under it. Three legs and a bar read as a pylon; the jib is what
    // makes it a crane.
    let crane = hazed(BlipColor { r: 0.17, g: 0.10, b: 0.15, a: 1.0 }, low, 0.30);
    for (n, (x, hgt, dir)) in [(96.0f32, 118.0f32, 1.0f32), (238.0, 92.0, 1.0), (566.0, 106.0, -1.0)].into_iter().enumerate() {
        let base = HORIZON + 4.0 + shake;
        let top = base - hgt;
        // A lattice tower, not a pole: two chords and the bracing zigzagging between them.
        blip.fill_rect(x - 4.0, top, 1.5, hgt, crane);
        blip.fill_rect(x + 2.5, top, 1.5, hgt, crane);
        let mut y = top;
        let mut flip = false;
        while y < base - 8.0 {
            let (a, b) = if flip { (x - 3.5, x + 3.5) } else { (x + 3.5, x - 3.5) };
            blip.draw_line(a, y, b, y + 7.0, crane);
            y += 7.0;
            flip = !flip;
        }
        // A warning lamp on top, blinking.
        if (t * 1.3 + n as f32 * 0.7).fract() < 0.45 {
            blip.fill_circle(x, top - 2.0, 1.8, BlipColor { r: 1.0, g: 0.20, b: 0.12, a: 1.0 });
            blip.fill_glow_circle(x, top - 2.0, 6.0, BlipColor { r: 1.0, g: 0.25, b: 0.15, a: 0.35 });
        }
        blip.fill_rect(x - 16.0, base - 8.0, 32.0, 8.0, crane);
        blip.draw_line(x, base, x - dir * 16.0, top + 14.0, crane);
        // Jib and the hook hanging off it.
        // The jib, a truss too.
        let jx = x.min(x + dir * 62.0);
        blip.fill_rect(jx, top, 62.0, 1.5, crane);
        blip.fill_rect(jx, top + 4.0, 62.0, 1.5, crane);
        for k in 0..9 {
            let a = jx + k as f32 * 7.0;
            blip.draw_line(a, top + 1.0, a + 7.0, top + 4.5, crane);
        }
        blip.draw_line(x + dir * 54.0, top + 5.0, x + dir * 54.0, top + 30.0, crane);
        blip.fill_rect(x + dir * 50.0, top + 30.0, 9.0, 6.0, crane);
        blip.draw_line(x, top, x + dir * 24.0, top - 16.0, crane);
        blip.draw_line(x + dir * 24.0, top - 16.0, x + dir * 60.0, top, crane);
    }

    // A moored ship, because a dock with nothing tied up at it is a
    // sea wall.
    let hull = hazed(BlipColor { r: 0.22, g: 0.24, b: 0.34, a: 1.0 }, low, 0.22);
    blip.fill_rect(300.0, HORIZON - 26.0 + shake, 186.0, 26.0, hull);
    blip.fill_rect(300.0, HORIZON - 30.0 + shake, 186.0, 5.0,
        blend(hull, BLIP_WHITE, 0.25));
    blip.fill_rect(352.0, HORIZON - 54.0 + shake, 58.0, 24.0, shade(hull, 1.15));
    blip.fill_rect(392.0, HORIZON - 72.0 + shake, 7.0, 20.0, shade(hull, 0.8));
    // Containers stacked two high, ribbed.
    for i in 0..6 {
        for tier in 0..2 {
            let c = [BlipColor { r: 0.55, g: 0.30, b: 0.22, a: 1.0 }, BlipColor { r: 0.26, g: 0.42, b: 0.40, a: 1.0 },
                     BlipColor { r: 0.62, g: 0.52, b: 0.24, a: 1.0 }][(i + tier * 2) % 3];
            if tier == 1 && i % 3 == 2 { continue; }
            let (cx, cy) = (414.0 + i as f32 * 12.0, HORIZON - 42.0 - tier as f32 * 12.0 + shake);
            let c = hazed(c, low, 0.3);
            blip.fill_rect(cx, cy, 11.0, 12.0, c);
            for r in 0..3 { blip.fill_rect(cx + 2.0 + r as f32 * 3.0, cy + 1.0, 1.0, 10.0, shade(c, 0.8)); }
        }
    }
    // Portholes along the hull, a red waterline, a rail along the deck.
    for i in 0..12 {
        blip.fill_circle(312.0 + i as f32 * 14.0, HORIZON - 17.0 + shake, 1.6, hazed(BlipColor { r: 1.0, g: 0.82, b: 0.5, a: 1.0 }, low, 0.4));
    }
    blip.fill_rect(300.0, HORIZON - 4.0 + shake, 186.0, 4.0, hazed(BlipColor { r: 0.58, g: 0.16, b: 0.14, a: 1.0 }, low, 0.3));
    for i in 0..24 { blip.fill_rect(302.0 + i as f32 * 8.0, HORIZON - 35.0 + shake, 1.0, 5.0, shade(hull, 0.8)); }
    blip.fill_rect(300.0, HORIZON - 35.0 + shake, 186.0, 1.0, shade(hull, 0.8));
    // Smoke off the funnel, drifting away from the sun and thinning.
    for k in 0..6 {
        let age = (t * 0.35 + k as f32 / 6.0).fract();
        let (px, py) = (395.0 - age * 70.0, HORIZON - 76.0 - age * 34.0 + shake);
        blip.fill_circle(px, py, 3.0 + age * 9.0, BlipColor { r: 0.55, g: 0.42, b: 0.46, a: 0.45 * (1.0 - age) });
    }

    // The far wharf, and the dockers watching from it.
    let wharf = hazed(BlipColor { r: 0.30, g: 0.20, b: 0.20, a: 1.0 }, low, 0.18);
    draw_crowd(blip, HORIZON + 8.0 + shake, 29.0,
        hazed(BlipColor { r: 0.15, g: 0.09, b: 0.13, a: 1.0 }, low, 0.16),
        t, 0x9e3, 34, roar);
    blip.fill_rect(0.0, HORIZON + 4.0 + shake, WIN_W as f32, 10.0, wharf);
    draw_chase(blip, HORIZON + 9.0 + shake, t, wharf);

    // Water: flat, dark, and the quietest thing on screen, because it
    // is what sits directly behind the fighters.
    let sea = BlipColor { r: 0.17, g: 0.22, b: 0.36, a: 1.0 };
    blip.fill_rect(0.0, HORIZON + 14.0 + shake, WIN_W as f32, 72.0, sea);
    // The sun's road on the water, and ripples crossing it.
    for i in 0..13 {
        let k = i as f32 / 12.0;
        let y = HORIZON + 18.0 + k * 62.0 + shake;
        let w = 16.0 + k * 54.0 + (t * 2.0 + i as f32).sin() * 5.0;
        blip.fill_rect(sun_x - w * 0.5, y, w, 2.5,
            BlipColor { r: 1.0, g: 0.80, b: 0.52, a: 0.30 - 0.16 * k });
    }
    // The ship and the cranes upside down in it, broken up by the swell.
    for row in 0..14 {
        let y = HORIZON + 14.0 + row as f32 * 2.0 + shake;
        let wob = (t * 2.2 + row as f32 * 0.9).sin() * (1.0 + row as f32 * 0.25);
        let fade = 0.30 * (1.0 - row as f32 / 14.0);
        blip.fill_rect(300.0 + wob, y, 186.0, 1.0, BlipColor { r: 0.08, g: 0.08, b: 0.14, a: fade });
        for x in [96.0f32, 238.0, 566.0] {
            blip.fill_rect(x - 4.0 + wob, y, 8.0, 1.0, BlipColor { r: 0.08, g: 0.06, b: 0.12, a: fade });
        }
    }
    // Glitter on the sun's road: points that wink on and off.
    for i in 0..26 {
        let k = scatter(i, 0x5eb);
        let y = HORIZON + 18.0 + k * 62.0 + shake;
        let x = sun_x + (scatter(i, 0x5ec) - 0.5) * (16.0 + k * 54.0);
        if (t * 3.0 + scatter(i, 0x5ed) * 9.0).sin() > 0.6 {
            blip.fill_rect(x, y, 2.0, 1.0, BlipColor { r: 1.0, g: 0.95, b: 0.75, a: 0.9 });
        }
    }
    for i in 0..22 {
        let sx = scatter(i, 0x5ea);
        let y = HORIZON + 20.0 + sx * 58.0 + shake;
        let x = ((i as f32 * 61.0) + t * (7.0 + sx * 9.0)) % (WIN_W as f32 + 60.0) - 30.0;
        blip.fill_rect(x, y, 14.0 + sx * 20.0, 1.5,
            BlipColor { r: 0.62, g: 0.74, b: 0.92, a: 0.16 });
    }

    // The quay wall the fighters stand on top of, with its bollards and
    // the tyres hung over the edge.
    let quay = BlipColor { r: 0.31, g: 0.22, b: 0.16, a: 1.0 };
    blip.fill_rect(0.0, FLOOR_Y - 30.0 + shake, WIN_W as f32, 30.0, quay);
    blip.fill_rect(0.0, FLOOR_Y - 32.0 + shake, WIN_W as f32, 3.0, blend(quay, BLIP_WHITE, 0.22));
    // Dressed stone in courses, each block a shade of its own, the joints staggered.
    for course in 0..3 {
        let y = FLOOR_Y - 29.0 + course as f32 * 10.0 + shake;
        let stagger = if course % 2 == 0 { 0.0 } else { 15.0 };
        for i in 0..23 {
            let x = i as f32 * 30.0 - stagger;
            blip.fill_rect(x + 1.0, y + 1.0, 28.0, 8.0, shade(quay, 0.90 + 0.18 * scatter(i + course * 31, 0x9a)));
        }
        blip.fill_rect(0.0, y + 9.0, WIN_W as f32, 1.0, shade(quay, 0.65));
    }
    // Wet and weedy along the bottom, where the tide reaches.
    blip.fill_rect(0.0, FLOOR_Y - 6.0 + shake, WIN_W as f32, 6.0, BlipColor { r: 0.14, g: 0.20, b: 0.12, a: 0.55 });
    for i in 0..5 {
        let x = 54.0 + i as f32 * 136.0;
        blip.fill_circle(x, FLOOR_Y - 14.0 + shake, 7.0, shade(quay, 0.55));
        blip.fill_circle(x, FLOOR_Y - 14.0 + shake, 3.5, sea);
    }
    for (x, n) in [(2.0f32, 2), (598.0, 3)] {
        for i in 0..n {
            let y = FLOOR_Y - 26.0 * (i + 1) as f32 + shake;
            let c = BlipColor { r: 0.44, g: 0.30, b: 0.17, a: 1.0 };
            blip.fill_rect(x, y, 38.0, 26.0, c);
            blip.fill_rect(x, y, 38.0, 4.0, blend(c, BLIP_WHITE, 0.2));
            blip.draw_rect(x, y, 38.0, 26.0, shade(c, 0.5));
        }
    }
}
