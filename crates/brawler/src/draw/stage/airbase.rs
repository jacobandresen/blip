//! The air base: midday on the apron, a jet parked behind the ground crew.

use super::*;

/// THE AIR BASE: midday on the apron, a jet parked behind the ground crew.
/// The brightest stage; the tarmac behind the fighters is kept bare.
pub(crate) fn draw_airbase(blip: &Blip, shake: f32, t: f32, roar: Roar) {
    // A high horizon: the jet stands above the fighters' heads and what is
    // behind them is bare tarmac.
    const HORIZON: f32 = 222.0;
    let c = |r: f32, g: f32, b: f32| BlipColor { r, g, b, a: 1.0 };
    let low = c(0.76, 0.87, 0.95);
    draw_sky(blip, shake, HORIZON, c(0.26, 0.50, 0.86), low);

    for i in 0..6 {
        let sx = scatter(i, 0xa1);
        let y = 40.0 + sx * 110.0 + shake;
        let w = 80.0 + scatter(i, 0xa2) * 120.0;
        let x = (t * (4.0 + sx * 4.0) + sx * 900.0) % (WIN_W as f32 + 240.0) - 120.0;
        blip.fill_rect(x, y, w, 8.0, BlipColor { a: 0.9, ..BLIP_WHITE });
        blip.fill_rect(x + 18.0, y + 8.0, w * 0.6, 5.0, c(0.88, 0.93, 0.98));
    }

    // Hills, hangars and the tower, far off.
    blip.fill_rect(0.0, HORIZON - 12.0 + shake, WIN_W as f32, 14.0, hazed(c(0.30, 0.42, 0.30), low, 0.5));
    let shed = hazed(c(0.50, 0.54, 0.58), low, 0.35);
    for x in [36.0f32, 120.0, 500.0] {
        blip.fill_circle(x + 30.0, HORIZON - 14.0 + shake, 30.0, shed);
        blip.fill_rect(x, HORIZON - 14.0 + shake, 60.0, 16.0, shed);
        blip.fill_rect(x + 18.0, HORIZON - 16.0 + shake, 24.0, 18.0, shade(shed, 0.7));
    }
    let tower = hazed(c(0.62, 0.64, 0.66), low, 0.25);
    blip.fill_rect(596.0, HORIZON - 84.0 + shake, 10.0, 86.0, tower);
    blip.fill_rect(584.0, HORIZON - 100.0 + shake, 34.0, 18.0, tower);
    blip.fill_rect(587.0, HORIZON - 96.0 + shake, 28.0, 7.0, c(0.30, 0.48, 0.66));

    // The jet: nose to the left, canopy, wing, a tall fin, on its wheels.
    let y = HORIZON + 4.0 + shake;
    let (skin, dark) = (c(0.66, 0.70, 0.74), c(0.44, 0.48, 0.54));
    for wx in [222.0f32, 372.0, 392.0] {
        blip.draw_line_ex(wx, y - 22.0, wx, y - 4.0, 3.0, dark);
        blip.fill_circle(wx, y - 4.0, 5.0, c(0.14, 0.14, 0.16));
    }
    stroke(blip, V(404.0, y - 36.0), V(446.0, y - 84.0), 9.0, 4.0, dark);
    stroke(blip, V(420.0, y - 30.0), V(462.0, y - 34.0), 5.0, 3.0, dark);
    stroke(blip, V(196.0, y - 26.0), V(436.0, y - 30.0), 13.0, 9.0, skin);
    stroke(blip, V(196.0, y - 26.0), V(150.0, y - 23.0), 13.0, 2.5, dark);
    stroke(blip, V(232.0, y - 38.0), V(270.0, y - 40.0), 7.0, 5.0, c(0.30, 0.52, 0.74));
    stroke(blip, V(300.0, y - 22.0), V(392.0, y - 12.0), 6.0, 3.0, dark);
    blip.fill_rect(200.0, y - 28.0, 228.0, 2.0, c(0.80, 0.22, 0.20));
    blip.fill_circle(338.0, y - 30.0, 6.0, c(0.92, 0.94, 0.96));
    blip.fill_circle(338.0, y - 30.0, 3.5, c(0.20, 0.36, 0.70));

    // The ground crew along the barrier, and bare tarmac down to the kerb.
    draw_crowd(blip, HORIZON + 14.0 + shake, 28.0, hazed(c(0.24, 0.30, 0.20), low, 0.12),
        t, 0xa7c, 30, roar);
    blip.fill_rect(0.0, HORIZON + 10.0 + shake, WIN_W as f32, 9.0, c(0.80, 0.80, 0.78));
    for i in 0..17 { blip.fill_rect(i as f32 * 40.0, HORIZON + 10.0 + shake, 20.0, 9.0, c(0.86, 0.30, 0.24)); }
    let tarmac = c(0.50, 0.52, 0.55);
    blip.fill_rect(0.0, HORIZON + 19.0 + shake, WIN_W as f32, FLOOR_Y - HORIZON - 19.0, tarmac);
    blip.fill_rect(0.0, HORIZON + 40.0 + shake, WIN_W as f32, 2.5, c(0.92, 0.80, 0.28));
    blip.fill_rect(0.0, FLOOR_Y - 8.0 + shake, WIN_W as f32, 8.0, shade(tarmac, 0.8));

    // Ammunition crates stacked at either edge.
    for (x, n) in [(2.0f32, 3), (598.0, 2)] {
        for i in 0..n {
            let top = FLOOR_Y - 26.0 * (i + 1) as f32 + shake;
            let crate_c = c(0.36, 0.42, 0.26);
            blip.fill_rect(x, top, 40.0, 26.0, crate_c);
            blip.fill_rect(x, top, 40.0, 4.0, blend(crate_c, BLIP_WHITE, 0.2));
            blip.fill_rect(x + 8.0, top + 11.0, 24.0, 3.0, c(0.90, 0.86, 0.60));
            blip.draw_rect(x, top, 40.0, 26.0, shade(crate_c, 0.5));
        }
    }
}
