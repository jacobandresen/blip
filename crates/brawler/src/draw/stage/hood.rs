//! The hood: a warm dusk block, fire escapes, murals and lowrider lights.

use super::*;

/// THE HOOD: a warm dusk block, fire escapes, murals and lowrider lights.
pub(super) fn draw_hood(blip: &Blip, shift: f32, now: f32) {
    let c = |r, g, b| BlipColor { r, g, b, a: 1.0 };
    blip.fill_rect(0.0, 0.0, WIN_W as f32, FLOOR_Y + shift, c(0.20, 0.12, 0.18));
    blip.fill_circle(500.0, 94.0 + shift, 45.0, c(0.91, 0.42, 0.28));
    for i in 0..9 {
        let x = i as f32 * 78.0 - 10.0;
        let h = 88.0 + scatter(i, 0x2f) * 105.0;
        let w = 66.0 + scatter(i, 0x31) * 22.0;
        let y = FLOOR_Y - h + shift;
        blip.fill_rect(x, y, w, h, c(0.13, 0.12, 0.19));
        blip.fill_rect(x + 4.0, y + 5.0, w - 8.0, 4.0, c(0.38, 0.21, 0.22));
        for row in 0..4 {
            for col in 0..3 {
                let glow = (now * 0.8 + (i * 13 + row * 7 + col) as f32).sin() > -0.45;
                if glow {
                    blip.fill_rect(x + 10.0 + col as f32 * 16.0, y + 20.0 + row as f32 * 22.0,
                        6.0, 9.0, c(0.88, 0.55, 0.28));
                }
            }
        }
    }
    // Painted block letters and a metal fire escape give the skyline its scale.
    let wall = c(0.18, 0.15, 0.21);
    blip.fill_rect(0.0, 188.0 + shift, WIN_W as f32, 56.0, wall);
    for y in [198.0f32, 220.0] {
        stroke(blip, V(25.0, y + shift), V(256.0, y + shift), 1.5, 1.5, c(0.55, 0.30, 0.34));
    }
    for x in [45.0f32, 118.0, 191.0, 264.0] {
        stroke(blip, V(x, 186.0 + shift), V(x, 235.0 + shift), 1.2, 1.2, c(0.62, 0.44, 0.40));
    }
    for (i, col) in [(0, c(0.95, 0.28, 0.30)), (1, c(0.98, 0.69, 0.25)), (2, c(0.25, 0.78, 0.75))] {
        let x = 324.0 + i as f32 * 28.0;
        stroke(blip, V(x, 214.0 + shift), V(x + 10.0, 198.0 + shift), 3.0, 3.0, col);
        stroke(blip, V(x + 10.0, 198.0 + shift), V(x + 20.0, 214.0 + shift), 3.0, 3.0, col);
    }
    // A parked lowrider silhouette, with softly pulsing tail lamps.
    let bob = (now * 1.4).sin() * 1.0;
    blip.fill_rect(432.0, 226.0 + shift + bob, 132.0, 18.0, c(0.10, 0.10, 0.15));
    blip.fill_rect(454.0, 214.0 + shift + bob, 65.0, 14.0, c(0.12, 0.12, 0.18));
    blip.fill_rect(463.0, 216.0 + shift + bob, 22.0, 9.0, c(0.45, 0.61, 0.66));
    blip.fill_rect(489.0, 216.0 + shift + bob, 24.0, 9.0, c(0.45, 0.61, 0.66));
    for x in [456.0, 534.0] {
        blip.fill_circle(x, 243.0 + shift, 9.0, c(0.06, 0.06, 0.08));
        blip.fill_circle(x, 243.0 + shift, 4.0, c(0.59, 0.56, 0.51));
    }
    blip.fill_rect(554.0, 231.0 + shift, 5.0, 4.0, c(0.96, 0.17, 0.14));
}
