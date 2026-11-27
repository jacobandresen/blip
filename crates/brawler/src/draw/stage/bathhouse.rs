//! The bath house: steam off the tub, and the painted mountain on the wall.

use super::*;

/// THE BATH HOUSE: indoors, steam off the tub, and on the wall the painted
/// mountain every bath house has. The tub's wooden side is the quiet band.
pub(crate) fn draw_bathhouse(blip: &Blip, shake: f32, t: f32, roar: Roar) {
    const HORIZON: f32 = 268.0;
    let c = |r: f32, g: f32, b: f32| BlipColor { r, g, b, a: 1.0 };
    let wall = c(0.86, 0.80, 0.66);
    let wood = c(0.42, 0.27, 0.16);
    blip.fill_rect(0.0, shake, WIN_W as f32, FLOOR_Y, wall);

    // The mural: sky, a red sun, the mountain with its snow, a line of surf.
    let (mx, my, mw, mh) = (70.0, 44.0 + shake, 500.0, 164.0);
    blip.fill_rect(mx, my, mw, mh, c(0.56, 0.78, 0.92));
    blip.fill_circle(452.0, my + 52.0, 30.0, c(0.90, 0.26, 0.20));
    let rows = 16;
    for r in 0..rows {
        let f = r as f32 / rows as f32;
        let w = 330.0 * (1.0 - f) + 26.0;
        let peak = if f > 0.72 { c(0.96, 0.97, 1.0) } else { c(0.28, 0.42, 0.70) };
        blip.fill_rect(300.0 - w / 2.0, my + mh - 18.0 - (r + 1) as f32 * 8.0, w, 9.0, peak);
    }
    blip.fill_rect(mx, my + mh - 18.0, mw, 18.0, c(0.22, 0.46, 0.66));
    for i in 0..21 { blip.fill_circle(mx + 12.0 + i as f32 * 24.0, my + mh - 17.0, 7.0, c(0.90, 0.95, 1.0)); }
    for (x, y, w, h) in [(mx - 6.0, my - 6.0, mw + 12.0, 6.0), (mx - 6.0, my + mh, mw + 12.0, 6.0),
                         (mx - 6.0, my, 6.0, mh), (mx + mw, my, 6.0, mh)] {
        blip.fill_rect(x, y, w, h, wood);
    }

    // The beam, and paper lanterns hung from it.
    blip.fill_rect(0.0, shake, WIN_W as f32, 26.0, wood);
    blip.fill_rect(0.0, 26.0 + shake, WIN_W as f32, 4.0, shade(wood, 0.7));
    for i in 0..4 {
        let x = 34.0 + i as f32 * 190.0;
        let glow = 0.88 + 0.12 * (t * 8.0 + i as f32 * 2.0).sin();
        blip.fill_glow_circle(x, 46.0 + shake, 18.0, BlipColor { r: 1.0, g: 0.78, b: 0.40, a: 0.16 * glow });
        blip.fill_circle(x, 46.0 + shake, 9.0, c(0.96, 0.90, 0.72));
        blip.fill_rect(x - 9.0, 44.0 + shake, 18.0, 2.0, c(0.74, 0.20, 0.18));
    }

    // Tiles down to the water, the bathers in it, and steam rising off it.
    let tile = c(0.72, 0.84, 0.88);
    blip.fill_rect(0.0, 214.0 + shake, WIN_W as f32, HORIZON - 214.0, tile);
    for i in 0..33 { blip.draw_line(i as f32 * 20.0, 214.0 + shake, i as f32 * 20.0, HORIZON + shake, shade(tile, 0.86)); }
    for j in 0..3 { blip.draw_line(0.0, 214.0 + j as f32 * 18.0 + shake, WIN_W as f32, 214.0 + j as f32 * 18.0 + shake, shade(tile, 0.86)); }
    draw_crowd(blip, HORIZON + 8.0 + shake, 26.0, c(0.62, 0.44, 0.34), t, 0xb47, 22, roar);
    blip.fill_rect(0.0, HORIZON + shake, WIN_W as f32, 9.0, c(0.36, 0.64, 0.74));
    for i in 0..9 {
        let sx = scatter(i, 0xb5);
        let rise = (t * (0.10 + sx * 0.08) + sx).fract();
        blip.fill_circle(30.0 + i as f32 * 72.0 + (t + sx * 6.0).sin() * 6.0, HORIZON - rise * 70.0 + shake,
            9.0 + rise * 12.0, BlipColor { r: 1.0, g: 1.0, b: 1.0, a: 0.16 * (1.0 - rise) });
    }

    // The tub's side: one flat run of boards under a pale rim.
    let tub = c(0.60, 0.42, 0.24);
    blip.fill_rect(0.0, HORIZON + 9.0 + shake, WIN_W as f32, FLOOR_Y - HORIZON - 9.0, tub);
    blip.fill_rect(0.0, HORIZON + 7.0 + shake, WIN_W as f32, 6.0, blend(tub, BLIP_WHITE, 0.3));
    for i in 0..17 { blip.draw_line(i as f32 * 40.0, HORIZON + 13.0 + shake, i as f32 * 40.0, FLOOR_Y + shake, shade(tub, 0.74)); }
    for y in [HORIZON + 26.0, FLOOR_Y - 12.0] { blip.fill_rect(0.0, y + shake, WIN_W as f32, 3.0, shade(tub, 0.6)); }

    // Wash buckets stacked at either edge.
    for (x, n) in [(6.0f32, 3), (606.0, 2)] {
        for i in 0..n {
            let top = FLOOR_Y - 18.0 * (i + 1) as f32 + shake;
            blip.fill_rect(x, top, 28.0, 18.0, c(0.84, 0.68, 0.36));
            blip.fill_rect(x, top + 5.0, 28.0, 2.0, c(0.50, 0.36, 0.18));
            blip.draw_rect(x, top, 28.0, 18.0, c(0.46, 0.32, 0.16));
        }
    }
}
