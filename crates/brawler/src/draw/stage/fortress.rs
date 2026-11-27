//! The crystal fortress: polar night, an aurora, and a hall of leaning ice crystals.

use super::*;

/// THE CRYSTAL FORTRESS: polar night, an aurora, and a hall of leaning ice
/// crystals with nobody in it. The snowfield behind the fighters is the
/// quiet band.
pub(crate) fn draw_fortress(blip: &Blip, shake: f32, t: f32) {
    const HORIZON: f32 = 230.0;
    let c = |r: f32, g: f32, b: f32| BlipColor { r, g, b, a: 1.0 };
    let low = c(0.36, 0.60, 0.74);
    draw_sky(blip, shake, HORIZON, c(0.04, 0.07, 0.20), low);

    // Stars, and the aurora: curtains of green hung across the dark, swaying.
    for i in 0..40 {
        let (x, y) = (scatter(i, 0xf1) * WIN_W as f32, scatter(i, 0xf2) * 120.0);
        let lit = 0.5 + 0.5 * (t * (1.0 + scatter(i, 0xf3) * 2.0) + i as f32).sin();
        blip.fill_rect(x, y + shake, 1.5, 1.5, blend(c(0.10, 0.16, 0.32), BLIP_WHITE, 0.35 + 0.5 * lit));
    }
    for i in 0..64 {
        let x = i as f32 * 10.0;
        let wave = (x * 0.012 + t * 0.25).sin() + 0.5 * (x * 0.031 - t * 0.4).sin();
        let top = 46.0 + wave * 18.0 + shake;
        let len = 46.0 + 22.0 * (x * 0.021 + t * 0.6).sin();
        // Flat colour, mixed with the sky it hangs in: brightest along the top.
        let sky = blend(c(0.04, 0.07, 0.20), low, top / HORIZON);
        let green = c(0.30, 0.95, 0.62);
        blip.fill_rect(x, top, 10.0, len, blend(sky, green, 0.22));
        blip.fill_rect(x, top, 10.0, len * 0.4, blend(sky, green, 0.42));
    }

    // Far ice ridges on the horizon.
    let ridge = hazed(c(0.70, 0.84, 0.94), low, 0.45);
    for i in 0..9 {
        let x = i as f32 * 80.0 - 20.0 + scatter(i, 0xf5) * 30.0;
        let h = 18.0 + scatter(i, 0xf6) * 30.0;
        stroke(blip, V(x, HORIZON + shake), V(x + 8.0, HORIZON - h + shake), 22.0, 1.0, ridge);
    }

    // The crystals: great shafts driven in at every angle, a lit face and a
    // shadowed one, taller toward the middle of the hall.
    let (lit, dark, edge) = (c(0.72, 0.88, 0.98), c(0.40, 0.60, 0.80), c(0.94, 0.98, 1.0));
    let shafts: [(f32, f32, f32, f32); 11] = [
        (40.0, -0.50, 150.0, 15.0), (96.0, 0.30, 120.0, 12.0), (150.0, -0.20, 190.0, 18.0),
        (214.0, 0.42, 150.0, 13.0), (268.0, -0.10, 216.0, 20.0), (330.0, 0.16, 226.0, 21.0),
        (392.0, -0.38, 160.0, 14.0), (446.0, 0.24, 196.0, 18.0), (508.0, -0.28, 130.0, 13.0),
        (560.0, 0.48, 156.0, 15.0), (612.0, -0.12, 110.0, 12.0)];
    for (x, lean, len, w) in shafts {
        let base = V(x, HORIZON + 8.0 + shake);
        let tip = V(x + lean * len, HORIZON + 8.0 - len * (1.0 - lean * lean * 0.5) + shake);
        stroke(blip, base, tip, w, 1.5, dark);
        // The lit face: the same shaft, narrower, moved toward the light.
        stroke(blip, V(base.0 - w * 0.34, base.1), V(tip.0 - 0.6, tip.1), w * 0.60, 1.0, lit);
        stroke(blip, V(base.0 - w * 0.62, base.1), V(tip.0 - 1.0, tip.1), 1.2, 0.6, edge);
    }
    // Grown, not built: beams thrown right across the hall overhead, crossing.
    for (a, b, w) in [(V(-10.0, 150.0), V(420.0, 34.0), 9.0f32), (V(650.0, 136.0), V(190.0, 20.0), 8.0),
                      (V(-10.0, 96.0), V(250.0, 8.0), 6.0)] {
        let (a, b) = (V(a.0, a.1 + shake), V(b.0, b.1 + shake));
        stroke(blip, a, b, w, 1.5, dark);
        stroke(blip, V(a.0, a.1 - w * 0.4), V(b.0, b.1 - 0.5), w * 0.5, 0.8, lit);
    }
    // The console: a bank of crystal rods stood in a block of ice, each one
    // lit from inside and none in step with its neighbour.
    let desk = HORIZON + 2.0 + shake;
    stroke(blip, V(268.0, desk), V(348.0, desk), 9.0, 9.0, dark);
    stroke(blip, V(268.0, desk - 3.0), V(348.0, desk - 3.0), 6.0, 6.0, lit);
    for i in 0..9 {
        let x = 274.0 + i as f32 * 8.5;
        let h = 16.0 + scatter(i, 0xfd) * 22.0;
        let glow = 0.5 + 0.5 * (t * (1.0 + scatter(i, 0xfe)) + i as f32 * 1.7).sin();
        let rod = blend(c(0.62, 0.90, 1.0), BLIP_WHITE, glow);
        blip.fill_rect(x - 2.0, desk - 6.0 - h, 4.0, h, dark);
        blip.fill_rect(x - 1.0, desk - 6.0 - h, 2.0, h, rod);
    }

    // The snowfield, down to the ice the fight is on, and broken crystal at
    // either edge.
    let snow = c(0.84, 0.91, 0.97);
    blip.fill_rect(0.0, HORIZON + 6.0 + shake, WIN_W as f32, FLOOR_Y - HORIZON - 6.0, snow);
    blip.fill_rect(0.0, HORIZON + 6.0 + shake, WIN_W as f32, 4.0, edge);
    blip.fill_rect(0.0, FLOOR_Y - 10.0 + shake, WIN_W as f32, 10.0, shade(snow, 0.88));
    for (x, lean) in [(14.0f32, 0.5f32), (44.0, -0.2), (600.0, -0.5), (628.0, 0.2)] {
        let base = V(x, FLOOR_Y + shake);
        stroke(blip, base, V(x + lean * 40.0, FLOOR_Y - 62.0 + shake), 11.0, 1.2, dark);
        stroke(blip, V(x - 3.5, base.1), V(x + lean * 40.0 - 0.6, FLOOR_Y - 62.0 + shake), 6.5, 1.0, lit);
    }
    // Snow coming down, slowly.
    for i in 0..36 {
        let x = (scatter(i, 0xfa) * WIN_W as f32 + (t * 6.0 + i as f32).sin() * 8.0) % WIN_W as f32;
        let y = (scatter(i, 0xfb) * FLOOR_Y + t * (14.0 + scatter(i, 0xfc) * 16.0)) % FLOOR_Y;
        blip.fill_rect(x, y + shake, 2.0, 2.0, BLIP_WHITE);
    }
}
