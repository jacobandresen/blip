//! Drawing. Fighters are posed from the same numbers the rules use: the reach
//! and height a punch is judged by are the reach and height it is drawn at.

use super::*;

fn rgb(c: (f32, f32, f32)) -> BlipColor { BlipColor { r: c.0, g: c.1, b: c.2, a: 1.0 } }

/// Width of a string in pixels at size `sz` (a fixed 6px cell). Not blip's
/// `text_cx()`, which returns the x that centres a string on the canvas.
fn text_w(text: &str, sz: f32) -> f32 { text.len() as f32 * 6.0 * sz }
fn rgba(c: (f32, f32, f32), a: f32) -> BlipColor { BlipColor { r: c.0, g: c.1, b: c.2, a } }

pub fn draw(blip: &Blip, g: &Game) {
    #[cfg(feature = "gallery")]
    { draw_gallery(blip, g.now); return; }
    #[allow(unreachable_code)]
    match g.state {
        State::Title => draw_title(blip, g),
        State::Select => draw_select(blip, g),
        _ => {
            draw_stage(blip, g);
            draw_fight(blip, g);
            draw_hud(blip, g);
            draw_banner(blip, g);
        }
    }
}

// ---- locations -----------------------------------------------------------

/// How far the backdrop is pushed back behind the fight: one scrim of air
/// over every layer, so cranes, skyline and crowd stop competing with the
/// fighters at full contrast. (Stages are scenery only: the same floor line,
/// height and walls in both.)
const HAZE: f32 = 0.34;

fn draw_stage(blip: &Blip, g: &Game) {
    // A hit shakes the whole picture, which is most of what selling an
    // impact means when the fighters themselves are simple shapes.
    let shake = if g.shake > 0.0 { (g.shake * 60.0).sin() * g.shake * 30.0 } else { 0.0 };
    let air = if g.stage == 0 {
        draw_dock(blip, shake, g.now, g.shake);
        BlipColor { r: 0.40, g: 0.17, b: 0.20, a: HAZE }
    } else {
        draw_temple(blip, shake, g.now, g.shake);
        BlipColor { r: 0.07, g: 0.07, b: 0.13, a: HAZE }
    };
    blip.fill_rect(0.0, 0.0, WIN_W as f32, FLOOR_Y + shake, air);

    draw_floor(blip, g.stage, shake);
}

/// The ground, seen flat-on: lit boards falling off toward the bottom of the
/// screen, bright enough for the fighters' shadows to land on.
fn draw_floor(blip: &Blip, stage: usize, shake: f32) {
    let near = if stage == 0 {
        BlipColor { r: 0.38, g: 0.28, b: 0.21, a: 1.0 }
    } else {
        BlipColor { r: 0.26, g: 0.25, b: 0.31, a: 1.0 }
    };
    let h = WIN_H as f32 - FLOOR_Y;
    let bands = 10;
    for i in 0..bands {
        let k = i as f32 / (bands - 1) as f32;
        let f = 1.0 - 0.55 * k;
        blip.fill_rect(0.0, FLOOR_Y + shake + k * h, WIN_W as f32, h / bands as f32 + 1.0,
            shade(near, f));
    }
    // Board seams, converging a little so the plane reads as going away
    // from the viewer rather than standing up behind them.
    let seam = shade(near, 0.62);
    for i in 0..17 {
        let x = i as f32 * 40.0;
        blip.draw_line(x, FLOOR_Y + shake, x + (x - WIN_W as f32 / 2.0) * 0.30,
            WIN_H as f32 + shake, seam);
    }
    // The lip where the floor meets the scenery, catching the light.
    blip.fill_rect(0.0, FLOOR_Y + shake - 1.0, WIN_W as f32, 2.0,
        blend(near, BLIP_WHITE, 0.35));
}

/// Deterministic scatter: the same crowd every frame, without storing
/// one. `k` indexes the thing being placed, `salt` separates two uses.
fn scatter(k: usize, salt: u32) -> f32 {
    let mut h = (k as u32).wrapping_mul(2654435761).wrapping_add(salt);
    h ^= h >> 15;
    h = h.wrapping_mul(2246822519);
    h ^= h >> 13;
    (h % 1024) as f32 / 1024.0
}

/// Distance as colour: far things lose contrast toward the sky rather than
/// just darkening.
fn hazed(c: BlipColor, sky: BlipColor, k: f32) -> BlipColor { blend(c, sky, k) }

/// A row of onlookers: small, dense, overlapping silhouettes below the
/// fighters' heads, so they read as a crowd and not more fighters. They bob
/// out of step and stand up for a big hit.
fn draw_crowd(blip: &Blip, ground: f32, h: f32, c: BlipColor, t: f32, salt: u32, n: usize,
              hype: f32) {
    // At this size the bodies merge and the crowd is the line along the top.
    // `hype` is how recently something landed.
    let h = h * (1.0 + 0.30 * hype);
    let mass = ground - h * 0.42;
    blip.fill_rect(0.0, mass, WIN_W as f32, ground - mass, shade(c, 0.88));
    for i in 0..n {
        let sx = scatter(i, salt);
        let x = -10.0 + (i as f32 + 0.5 + (sx - 0.5) * 1.1) * (WIN_W as f32 + 20.0) / n as f32;
        let tall = h * (0.76 + 0.48 * scatter(i, salt ^ 0x5bd1));
        let bob = ((t * 2.6 + sx * 9.1).sin()).max(0.0) * 1.6;
        let top = ground - tall + bob;
        let cc = shade(c, 0.80 + 0.40 * scatter(i, salt ^ 0x77));
        // Shoulders twice the width of a head, which is the proportion
        // that reads as a person at any size.
        let sh = tall * 0.21;
        let neck = top + tall * 0.23;
        blip.fill_rect(x - sh, neck, sh * 2.0, mass - neck + 2.0, cc);
        blip.fill_circle(x, top + tall * 0.13, tall * 0.108, cc);
        // A few of them with their arms up, and more of them when
        // something has just landed.
        if scatter(i, salt ^ 0x1f) < 0.20 + 0.55 * hype {
            let up = tall * (0.30 + 0.10 * ((t * 5.0 + sx * 7.0).sin()));
            blip.fill_rect(x - sh - 1.5, neck - up, 2.6, up + 2.0, cc);
            blip.fill_rect(x + sh - 1.1, neck - up * 0.86, 2.6, up + 2.0, cc);
        }
    }
}

/// THE DOCKS: sunset over a working harbour. Layers far to near (sky, cloud,
/// skyline, far wharf and people, water, the quay), each hazed toward the sky
/// by distance. The fighters' band is kept quiet, flat water; detail goes
/// above their heads or below their knees.
fn draw_dock(blip: &Blip, shake: f32, t: f32, hit: f32) {
    let hit = (hit / 0.16).clamp(0.0, 1.0);
    const HORIZON: f32 = 250.0;
    let sky_at = |k: f32| BlipColor {
        r: 0.99 - 0.76 * k, g: 0.52 - 0.40 * k, b: 0.34 + 0.06 * k, a: 1.0,
    };
    let bands = 16;
    let h = HORIZON / bands as f32;
    for i in 0..bands {
        let k = i as f32 / (bands - 1) as f32;
        blip.fill_rect(0.0, i as f32 * h + shake, WIN_W as f32, h + 1.0, sky_at(1.0 - k));
    }
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
    }
    // Gantry cranes: a tower, a jib out over the water, and the cable
    // under it. Three legs and a bar read as a pylon; the jib is what
    // makes it a crane.
    let crane = hazed(BlipColor { r: 0.17, g: 0.10, b: 0.15, a: 1.0 }, low, 0.30);
    for (x, hgt, dir) in [(96.0f32, 118.0f32, 1.0f32), (238.0, 92.0, 1.0), (566.0, 106.0, -1.0)] {
        let base = HORIZON + 4.0 + shake;
        let top = base - hgt;
        blip.fill_rect(x - 3.0, top, 6.0, hgt, crane);
        blip.fill_rect(x - 16.0, base - 8.0, 32.0, 8.0, crane);
        blip.draw_line(x, base, x - dir * 16.0, top + 14.0, crane);
        // Jib and the hook hanging off it.
        blip.fill_rect(x.min(x + dir * 62.0), top, 62.0, 5.0, crane);
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
    for i in 0..6 {
        let c = if i % 2 == 0 { BlipColor { r: 0.55, g: 0.30, b: 0.22, a: 1.0 } }
                else { BlipColor { r: 0.26, g: 0.42, b: 0.40, a: 1.0 } };
        blip.fill_rect(414.0 + i as f32 * 12.0, HORIZON - 42.0 + shake, 11.0, 12.0,
            hazed(c, low, 0.3));
    }

    // The far wharf, and the dockers watching from it.
    let wharf = hazed(BlipColor { r: 0.30, g: 0.20, b: 0.20, a: 1.0 }, low, 0.18);
    draw_crowd(blip, HORIZON + 8.0 + shake, 29.0,
        hazed(BlipColor { r: 0.15, g: 0.09, b: 0.13, a: 1.0 }, low, 0.16),
        t, 0x9e3, 34, hit);
    blip.fill_rect(0.0, HORIZON + 4.0 + shake, WIN_W as f32, 10.0, wharf);

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
    for i in 0..22 {
        blip.draw_line(i as f32 * 30.0, FLOOR_Y - 29.0 + shake, i as f32 * 30.0,
            FLOOR_Y + shake, shade(quay, 0.72));
    }
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

/// THE TEMPLE: night under a roof, the mountain behind. Built like the docks;
/// the light comes from the lanterns.
fn draw_temple(blip: &Blip, shake: f32, t: f32, hit: f32) {
    let hit = (hit / 0.16).clamp(0.0, 1.0);
    const HORIZON: f32 = 262.0;
    let sky_at = |k: f32| BlipColor {
        r: 0.05 + 0.13 * k, g: 0.06 + 0.13 * k, b: 0.13 + 0.17 * k, a: 1.0,
    };
    let bands = 14;
    let h = HORIZON / bands as f32;
    for i in 0..bands {
        let k = i as f32 / (bands - 1) as f32;
        blip.fill_rect(0.0, i as f32 * h + shake, WIN_W as f32, h + 1.0, sky_at(k));
    }
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
        BlipColor { r: 0.20, g: 0.20, b: 0.29, a: 1.0 }, t, 0x77a, 30, hit);
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

// ---- anatomy -------------------------------------------------------------
// A player reads an attack off the shape of the body throwing it (weight,
// loaded leg, turned shoulder), so fighters are drawn as bodies:
// 1. **Pose space.** Joints are (forward, up) from the floor under the feet,
// so one pose serves both sides.
// 2. **Two bones per limb.** Hands and feet are placed; knees and elbows are
// solved.
// 3. **Tapered strokes.** Every part runs thick to thin; uniform width is a
// stick figure.

/// A point on a fighter: `f` forward — the way they face — and `u` up
/// from the floor under them.
#[derive(Copy, Clone)]
struct P { f: f32, u: f32 }
const fn p(f: f32, u: f32) -> P { P { f, u } }

impl P {
    /// Part of the way from here to there. Animation is almost entirely
    /// this, which is why it is the only operator a pose needs.
    fn to(self, o: P, k: f32) -> P { p(self.f + (o.f - self.f) * k, self.u + (o.u - self.u) * k) }
}

/// A point on screen.
#[derive(Copy, Clone, Debug)]
pub(crate) struct V(pub f32, pub f32);

fn dist(a: V, b: V) -> f32 { ((b.0 - a.0).powi(2) + (b.1 - a.1).powi(2)).sqrt() }
fn along(a: V, b: V, k: f32) -> V { V(a.0 + (b.0 - a.0) * k, a.1 + (b.1 - a.1) * k) }

/// Where a fighter's pose space lands on screen.
#[derive(Copy, Clone)]
pub(crate) struct Rig {
    cx: f32,
    ground: f32,
    /// Which way "forward" points in pose space.
    fwd: f32,
    /// Which way the face points. The same as `fwd` on a fighter who is
    /// upright, and the opposite on one who has been put on their back.
    face: f32,
    /// How much of a joint's height survives the mapping and which way it
    /// runs on screen: one for the fighter (up), negative for the shadow,
    /// which runs toward the camera since the light is behind the fighters.
    squash: f32,
    /// How far a joint slides sideways per unit of height — the angle
    /// of the light. Zero for the fighter.
    skew: f32,
    /// Height of the feet above the floor, added before projecting: zero for
    /// the fighter, and what makes a jump's shadow fall further off and
    /// smaller.
    rise: f32,
}

impl Rig {
    fn at(self, q: P) -> V {
        let u = q.u + self.rise;
        V(self.cx + self.fwd * q.f + u * self.skew, self.ground - u * self.squash)
    }
    pub(crate) fn upright(cx: f32, ground: f32, fwd: f32, face: f32) -> Rig {
        Rig { cx, ground, fwd, face, squash: 1.0, skew: 0.0, rise: 0.0 }
    }
    /// How long a bone is once this rig has had it. One upright; less
    /// for a shadow, which is the same body seen at a glancing angle.
    fn bone(self) -> f32 { self.squash.abs().max(0.45) }
}

/// Bone lengths in pixels, from figure-drawing proportions: the femur a
/// little longer than the tibia, the hip at half the standing height. The
/// legs add up to more than the hip height, so a fighter stands with bent
/// knees.
const THIGH: f32 = 30.0;
const SHIN: f32 = 29.0;
const UARM: f32 = 24.0;
const FARM: f32 = 21.0;

/// Joint heights standing and crouching: hips and head (the spine between is
/// one bone, see `neck_of`); the crouch puts the head top at `CROUCH_H`. Hips
/// low: at 58 the solver clamped both legs straight (see
/// `a_waiting_fighter_has_their_knees_bent`).
const HIP_U: f32 = 53.0;
const HEAD_U: f32 = 109.0;
const C_HIP: f32 = 31.0;
const C_HEAD: f32 = 64.0;

fn shade(c: BlipColor, k: f32) -> BlipColor {
    BlipColor { r: c.r * k, g: c.g * k, b: c.b * k, a: c.a }
}

fn blend(a: BlipColor, b: BlipColor, k: f32) -> BlipColor {
    BlipColor {
        r: a.r + (b.r - a.r) * k,
        g: a.g + (b.g - a.g) * k,
        b: a.b + (b.b - a.b) * k,
        a: a.a + (b.a - a.a) * k,
    }
}

/// The line around everything. A fighter without one dissolves into
/// whichever stage they are standing in front of the moment the two
/// happen to be a similar colour.
const INK: BlipColor = BlipColor { r: 0.07, g: 0.06, b: 0.09, a: 1.0 };

/// A cast shadow: opaque (overlapping strokes would double any alpha where
/// limbs cross) and a little cooler than the boards, since a shadow takes the
/// sky's colour.
const SHADOW: BlipColor = BlipColor { r: 0.23, g: 0.17, b: 0.18, a: 1.0 };

/// How a fighter is coloured and clothed, already shaded for the limb
/// being drawn.
#[derive(Copy, Clone)]
struct Hide {
    cloth: BlipColor,
    trim: BlipColor,
    skin: BlipColor,
    hair: BlipColor,
    build: Build,
    bulk: f32,
    /// 1.0 for a near limb, less for a far one. Depth, on a flat
    /// picture, is mostly just this.
    tone: f32,
    /// Blown toward white for the frames a hit is frozen on.
    flash: f32,
    /// One breath, -1..1: a slow cycle plus a faster ripple that grows with exertion. Rates are fixed
    /// and only amplitude changes, so a blow landing never skips the phase.
    breath: f32,
    /// How hard this fighter is working, 0 to 1. Sweat, and how deep
    /// the breathing under it is.
    sweat: f32,
    /// Whether this limb is in front of the body and needs a heavier edge: a
    /// white sleeve over a white jacket has no silhouette otherwise.
    front: bool,
}

impl Hide {
    fn c(&self, base: BlipColor) -> BlipColor {
        blend(shade(base, self.tone), BLIP_WHITE, self.flash)
    }
    /// The outline barely darkens with distance. An edge that fades
    /// with the limb it belongs to stops separating it from the limb in
    /// front, which is the entire job of having one.
    fn ink(&self) -> BlipColor { blend(shade(INK, 0.8 + 0.2 * self.tone), BLIP_WHITE, self.flash * 0.6) }
    /// The far side of the body: darker and pulled toward the shadow colour,
    /// not just dimmed, so it does not read as the near limb drawn twice.
    fn far(self) -> Hide {
        let cool = |c: BlipColor| blend(shade(c, 0.58), SHADOW, 0.22);
        Hide {
            cloth: cool(self.cloth),
            trim: cool(self.trim),
            skin: cool(self.skin),
            hair: cool(self.hair),
            front: false,
            ..self
        }
    }
    fn infront(self) -> Hide { Hide { front: true, ..self } }

    /// Cut this limb out of whatever is behind it; each node carries its own
    /// radius, since a limb tapers.
    fn cut(&self, blip: &Blip, nodes: &[(V, f32)]) {
        if !self.front { return; }
        let ink = blend(INK, BLIP_WHITE, self.flash * 0.5);
        for w in nodes.windows(2) {
            stroke(blip, w[0].0, w[1].0, w[0].1, w[1].1, ink);
        }
    }
    /// The belt. Black over a gi, the fighter's own colour otherwise —
    /// a red belt on a white jacket is a cummerbund.
    fn belt(&self) -> BlipColor {
        match self.build {
            Build::Gi => self.c(BlipColor { r: 0.13, g: 0.12, b: 0.15, a: 1.0 }),
            _ => self.c(self.trim),
        }
    }
}

/// Sweat, the only thing on a sixty-pixel figure that says it has been in a
/// fight. Beads are highlights: a pale core over a darker rim.
const WET: BlipColor = BlipColor { r: 0.82, g: 0.92, b: 1.0, a: 0.92 };

fn bead(blip: &Blip, x: f32, y: f32, r: f32, a: f32) {
    if a <= 0.02 { return; }
    // The rim is a hint, not a ring. At this size a bead is two or
    // three pixels across, and a dark edge as wide again turns three
    // beads on a jaw into one pale mass with a beard's outline.
    blip.fill_circle(x, y + r * 0.45, r * 0.9,
        BlipColor { r: 0.30, g: 0.44, b: 0.58, a: a * 0.30 });
    blip.fill_circle(x, y, r, BlipColor { a: a * WET.a, ..WET });
}

/// Beads standing on the skin and running off. `spots` are hand-placed (a
/// random cluster on a ten-pixel face lands on the eye and reads as a
/// bruise); wetness decides how many are used. A bead's life is a function of
/// the clock.
fn sweat_beads(blip: &Blip, at: V, fw: f32, spots: &[(f32, f32)], t: f32, wet: f32, salt: u32) {
    if wet < 0.12 { return; }
    let n = ((1.0 + wet * spots.len() as f32).round() as usize).min(spots.len());
    let cycle = 2.4;
    for j in 0..n {
        let k = j as u32 + salt;
        // Beads take turns rather than random phases, or three on a jaw merge
        // into a beard; evenly spaced, there is usually one on the chin.
        let u = (t / cycle + (j as f32 + scatter(5, salt)) / n as f32).fract();
        // Gather, stand, run off. It is only on the skin for half its
        // turn; the rest of the cycle that spot is dry.
        let run = ((u - 0.38) / 0.16).max(0.0).min(1.0);
        // How wet a fighter is decides how MANY beads there are, not
        // how faint each one is. A sheen made of half-transparent dots
        // is a sheen nobody can see at this size; a bead is a bead.
        let a = (0.62 + 0.38 * wet) * (u * 12.0).min(1.0) * (1.0 - run * run);
        let (sx, sy) = spots[j];
        bead(blip, at.0 + fw * sx, at.1 + sy + run * 16.0, 1.1 + 0.5 * scatter(4, k), a);
    }
}

/// Sweat knocked off a fighter by a blow. Thrown the way the blow went,
/// arcing and falling — the one moment the sheen becomes a spray.
fn sweat_spray(blip: &Blip, f: &Fighter, at: V, shift: f32) {
    if f.act != Act::Hitstun || f.t > 0.30 { return; }
    let k = (1.0 - f.t / 0.30).max(0.0);
    let away = -f.facing;
    for j in 0..7u32 {
        let sp = 60.0 + scatter(1, j) * 150.0;
        let up = 40.0 + scatter(2, j) * 130.0;
        let x = at.0 + away * sp * f.t + (scatter(3, j) - 0.5) * 10.0;
        let y = at.1 - up * f.t + GRAVITY * 0.5 * f.t * f.t + (scatter(4, j) - 0.5) * 8.0;
        bead(blip, x, (y + shift).min(FLOOR_Y + shift), 1.0 + scatter(5, j), k * 0.95);
    }
}

/// A tapered capsule — the only primitive a body is made of.
fn stroke(blip: &Blip, a: V, b: V, r1: f32, r2: f32, c: BlipColor) {
    let d = dist(a, b);
    let n = (d / 1.7).ceil().max(1.0);
    let steps = n as i32;
    for i in 0..=steps {
        let t = i as f32 / n;
        blip.fill_circle(a.0 + (b.0 - a.0) * t, a.1 + (b.1 - a.1) * t, r1 + (r2 - r1) * t, c);
    }
}

/// The same shape, drawn once fatter in ink and once in colour, so the
/// part has an edge. Overlapping parts then read as one in front of the
/// other rather than as a single blob of the same colour.
fn part(blip: &Blip, a: V, b: V, r1: f32, r2: f32, c: BlipColor, ink: BlipColor) {
    stroke(blip, a, b, r1 + 1.7, r2 + 1.7, ink);
    stroke(blip, a, b, r1, r2, c);
    // Light along one side and shadow along the other, across the limb: a
    // cylinder, not a sausage, and what keeps a white arm off a white chest.
    let (dx, dy) = (b.0 - a.0, b.1 - a.1);
    let d = (dx * dx + dy * dy).sqrt().max(0.001);
    // Perpendicular, pointed up-screen: the light is always overhead.
    let (mut nx, mut ny) = (-dy / d, dx / d);
    if ny > 0.0 { nx = -nx; ny = -ny; }
    // Three flat tones with hard steps between them. A soft ramp
    // averages to one mid tone the moment the figure is small or
    // moving; a hard step is a line the eye can find at speed.
    let lit = blend(c, BLIP_WHITE, 0.34);
    let dark = shade(c, 0.60);
    stroke(blip, V(a.0 - nx * r1 * 0.42, a.1 - ny * r1 * 0.42),
                 V(b.0 - nx * r2 * 0.42, b.1 - ny * r2 * 0.42), r1 * 0.50, r2 * 0.50, dark);
    stroke(blip, V(a.0 + nx * r1 * 0.56, a.1 + ny * r1 * 0.56),
                 V(b.0 + nx * r2 * 0.56, b.1 + ny * r2 * 0.56), r1 * 0.30, r2 * 0.30, lit);
}

fn blob(blip: &Blip, at: V, r: f32, c: BlipColor, ink: BlipColor) {
    blip.fill_circle(at.0, at.1, r + 1.7, ink);
    blip.fill_circle(at.0, at.1, r, c);
    // Shadow under, highlight over, both flat — the same three-tone
    // treatment the limbs get in `part`, so a head and an arm are lit
    // by the same sun.
    blip.fill_circle(at.0 + r * 0.22, at.1 + r * 0.30, r * 0.72, shade(c, 0.60));
    blip.fill_circle(at.0, at.1, r * 0.80, c);
    blip.fill_circle(at.0 - r * 0.30, at.1 - r * 0.34, r * 0.46, blend(c, BLIP_WHITE, 0.30));
}

/// How far a two-bone limb can fold: a knee to about thirty degrees, an elbow
/// to about thirty-five.
const KNEE_SHUT: f32 = 0.52;  // ~30 degrees, in radians
const ELBOW_SHUT: f32 = 0.61; // ~35 degrees

/// The two-bone solve: where the joint goes and where the limb's end actually
/// lands. Bones never stretch; the target is clamped into the reachable ring
/// and the settled point returned. `toward` picks the way the joint breaks
/// (knees forward, elbows back).
fn solve(root: V, want: V, l1: f32, l2: f32, shut: f32, toward: V, floor: f32) -> (V, V) {
    let (dx, dy) = (want.0 - root.0, want.1 - root.1);
    let d_raw = (dx * dx + dy * dy).sqrt();
    // How close the two ends can get with the joint fully shut.
    let d_min = (l1 * l1 + l2 * l2 - 2.0 * l1 * l2 * shut.cos()).sqrt();
    let d = d_raw.clamp(d_min, l1 + l2);
    // A target sitting exactly on the root has no direction to go in.
    let (ux, uy) = if d_raw > 0.001 { (dx / d_raw, dy / d_raw) } else { (toward.0, toward.1) };
    let end = V(root.0 + ux * d, root.1 + uy * d);

    let cs = ((l1 * l1 + d * d - l2 * l2) / (2.0 * l1 * d)).clamp(-1.0, 1.0);
    let base = uy.atan2(ux);
    let off = cs.acos();
    let one = V(root.0 + l1 * (base + off).cos(), root.1 + l1 * (base + off).sin());
    let two = V(root.0 + l1 * (base - off).cos(), root.1 + l1 * (base - off).sin());
    // The two solutions mirror about the line between the ends; `toward`
    // always picks. (Keeping joints out of the floor instead sometimes bent a
    // knee backwards; a pose that sinks a knee is fixed in the pose, and a
    // test says which.)
    let _ = floor;
    let score = |c: V| (c.0 - root.0) * toward.0 + (c.1 - root.1) * toward.1;
    (if score(one) >= score(two) { one } else { two }, end)
}

// ---- parts ---------------------------------------------------------------
// Each part earns its place by changing the outline at this size: a foot
// sticking out behind the ankle does, a swelling calf does not.

/// The line a hanging piece of cloth takes: pinned at one end and swinging a
/// little.
fn hang(blip: &Blip, h: &Hide, at: V, dir: (f32, f32), len: f32, sway: f32,
        r1: f32, r2: f32, c: BlipColor) {
    let (px, py) = (-dir.1, dir.0);
    let end = V(at.0 + dir.0 * len + px * sway, at.1 + dir.1 * len + py * sway);
    part(blip, at, end, r1, r2, c, h.ink());
}

fn draw_leg(blip: &Blip, h: &Hide, hip: V, knee: V, ankle: V, fwd: f32, ground: f32, t: f32) {
    let b = h.bulk;
    let ink = h.ink();
    let skin = h.c(h.skin);
    let cloth = h.c(h.cloth);

    // Planted: the leg ends at the ankle joint, sole exactly on the boards
    // (heel bottom + outline = 8.8 below the joint). Nothing is drawn under it.
    let flat = ankle.1 > ground - 8.8 * b;
    let joint = if flat { V(ankle.0, ground - 8.8 * b) } else { ankle };
    let ankle = joint;

    h.cut(blip, &[(hip, 10.6 * b), (knee, 8.8 * b),
                  (ankle, if h.build == Build::Bare { 6.4 } else { 8.0 } * b)]);

    // Two bones, two strokes: the thigh narrowing by a third to the knee, the
    // shin to a thin ankle.
    part(blip, hip, knee, 9.4 * b, 5.3 * b, skin, ink);
    part(blip, knee, ankle, 6.2 * b, 3.0 * b, skin, ink);

    // Clothing is drawn on the bone (thigh and shin are straight, so is the
    // cloth over them); only what hangs past the ankle is free.
    let hem = match h.build {
        Build::Gi => 0.86,
        Build::Bare => 0.30,
        Build::Suit => 0.94,
    };
    let hem_at = along(knee, ankle, hem);
    part(blip, hip, knee, 10.6 * b, 7.0 * b, cloth, ink);
    part(blip, knee, hem_at, 7.0 * b, if h.build == Build::Gi { 6.6 } else { 5.5 } * b,
        cloth, ink);
    match h.build {
        // The loose hem of a gi trouser, trailing off the ankle.
        Build::Gi => {
            // The hem stops at the ankle; it never trails below the foot.
            let (dx, dy) = unit(knee, ankle);
            let reach = dist(hem_at, ankle) + 1.0;
            hang(blip, h, hem_at, (dx, dy), reach, (t * 5.0 + ankle.0 * 0.1).sin() * 0.8,
                6.2 * b, 4.6 * b, cloth);
        }
        // Heavy boots.
        Build::Bare => {
            part(blip, along(knee, ankle, 0.60), ankle, 4.4 * b, 3.2 * b, h.c(h.trim), ink);
        }
        // Wrapped shins: a band at the hem, which is where the eye
        // looks for the joint between cloth and skin.
        Build::Suit => {
            stroke(blip, along(knee, ankle, hem - 0.12), hem_at, 6.4 * b, 6.2 * b, h.c(h.trim));
        }
    }

    // Foot: heel behind the ankle, toes as their own piece. Flat when planted;
    // off the ground it follows the shin.
    let boot = h.build == Build::Bare;
    let foot_c = if boot { shade(h.c(h.trim), 0.72) } else { skin };
    let (fx, fy, ux, uy) = if flat {
        (fwd, 0.0, 0.0, -1.0)
    } else {
        // Walking: the toes stay forward and the heel peels up, pitching
        // the foot about the ball. A kick (ankle well off the ground) aligns
        // the foot with the shin instead; blend between the two.
        let lift = ground - ankle.1;
        let sgn = fwd.signum();
        let pitch = ((lift - 8.8 * b) / 12.0).clamp(0.0, 0.9);
        let (ps, pc) = (pitch.sin(), pitch.cos());
        let (tf, tu) = ((sgn * pc, ps), (sgn * ps, -pc));
        let (dx, dy) = unit(knee, ankle);
        let k = ((lift - 22.0) / 12.0).clamp(0.0, 1.0);
        let k = k * k * (3.0 - 2.0 * k);
        let norm = |x: f32, y: f32| { let m = x.hypot(y).max(1e-3); (x / m, y / m) };
        let (fx, fy) = norm(tf.0 + (dx - tf.0) * k, tf.1 + (dy - tf.1) * k);
        let (ux, uy) = norm(tu.0 + (dy - tu.0) * k, tu.1 + (-dx - tu.1) * k);
        (fx, fy, ux, uy)
    };
    let ank = ankle;
    let at = |along: f32, up: f32| V(ank.0 + fx * along + ux * up, ank.1 + fy * along + uy * up);
    // Feet are drawn big, a fifth of the figure as on a fighting sprite: the
    // ends of the limbs are what hits you, so they are the clearest shapes.
    let fb = if boot { b * 0.92 } else { b };
    let ball = at(7.4 * fb, -5.4 * fb);
    part(blip, at(-5.2 * fb, -3.2 * fb), ball, 3.9 * fb, 3.1 * fb, foot_c, ink);
    part(blip, ball, at(12.6 * fb, -4.9 * fb), if boot { 3.1 } else { 2.9 } * b, 1.7 * b,
        foot_c, ink);
}

/// A hand as one silhouette: all the ink first, then all the skin, so a hand
/// of four shapes does not show four outlines.
fn lumps(blip: &Blip, pieces: &[(V, V, f32, f32)], c: BlipColor, ink: BlipColor) {
    for (a, b, r1, r2) in pieces { stroke(blip, *a, *b, r1 + 1.7, r2 + 1.7, ink); }
    for (a, b, r1, r2) in pieces { stroke(blip, *a, *b, *r1, *r2, c); }
}

/// A fist in profile (the fighters are always seen side-on): not a row of
/// knuckles, which in profile is a claw, but a mitten with a squared knuckle
/// corner at the top front, a thumb at about forty-five degrees from the
/// wrist, and a Y of creases between.
fn draw_fist(blip: &Blip, h: &Hide, wrist: V, hand: V, b: f32) {
    let skin = h.c(h.skin);
    let ink = h.ink();
    let (fx, fy) = unit(wrist, hand);
    // Across the hand in the picture plane: negative is the back of the
    // hand (the knuckle side), positive is the palm, which is where the
    // fingers curl to and the thumb lies over them.
    let (px, py) = (-fy, fx);
    let at = |f: f32, a: f32| V(hand.0 + fx * f * b + px * a * b,
                                hand.1 + fy * f * b + py * a * b);

    lumps(blip, &[
        // The back of the hand: a flat slab from the wrist to the
        // knuckles, and the top edge of the whole shape.
        (at(-5.0, -1.4), at(2.0, -1.9), 2.5 * b, 2.3 * b),
        // The knuckle mass across the front face: the squared top corner the
        // fist is read from.
        (at(2.5, -1.9), at(2.5, 0.6), 2.3 * b, 2.2 * b),
        // The fingers, curled under and back toward the palm.
        (at(0.2, 2.1), at(2.2, 1.9), 2.4 * b, 2.3 * b),
    ], skin, ink);

    // The folds between the digits: an upside-down Y where the curled
    // fingers meet the knuckles. Two strokes, a shade darker, and they
    // are what stop the finger mass reading as one lump of dough.
    let crease = shade(skin, 0.64);
    stroke(blip, at(4.0, 0.7), at(1.5, 1.2), 0.55 * b, 0.5 * b, crease);
    stroke(blip, at(1.5, 1.2), at(1.9, 3.4), 0.5 * b, 0.45 * b, crease);

    // Light along the top of the hand and over the knuckle that lands.
    stroke(blip, at(-2.0, -3.0), at(2.4, -3.0), 1.1 * b, 1.3 * b,
           blend(skin, BLIP_WHITE, 0.30));

    // The thumb, a bar at about forty-five degrees across the fingers:
    // lighter (nearest) and without an ink edge, which at this size would be
    // most of it.
    stroke(blip, at(-3.2, 0.4), at(0.4, 3.0), 1.9 * b, 1.6 * b,
           blend(skin, BLIP_WHITE, 0.20));
    stroke(blip, at(-3.0, -0.9), at(0.0, 1.6), 0.55 * b, 0.55 * b, shade(skin, 0.62));
}

/// An open hand: a palm and three fingers, because a grab and a punch
/// are not the same thing and the picture has to say which.
fn draw_palm(blip: &Blip, h: &Hide, wrist: V, hand: V, b: f32) {
    let skin = h.c(h.skin);
    let ink = h.ink();
    let (fx, fy) = unit(wrist, hand);
    let (px, py) = (-fy, fx);
    let at = |f: f32, a: f32| V(hand.0 + fx * f * b + px * a * b,
                                hand.1 + fy * f * b + py * a * b);

    // Fingers first, so the palm's outline closes over the end of them
    // and the hand is one silhouette with three splits in it rather
    // than four outlined objects touching.
    for (f, a) in [(6.6, -3.0), (7.4, 0.0), (6.4, 3.0)] {
        part(blip, at(1.0, a * 0.5), at(f, a), 2.2 * b, 1.8 * b, skin, ink);
    }
    part(blip, at(-3.0, 0.0), at(1.6, 0.0), 5.0 * b, 5.8 * b, skin, ink);
    // The thumb out to the side, which is the whole of what makes a
    // hand a hand rather than a mitten. Unoutlined, as on the fist.
    stroke(blip, at(-1.2, 3.2), at(1.8, 5.2), 2.2 * b, 1.8 * b, shade(skin, 0.80));
}

fn draw_arm(blip: &Blip, h: &Hide, shoulder: V, elbow: V, hand: V, open: bool, t: f32) {
    let b = h.bulk;
    let ink = h.ink();
    let skin = h.c(h.skin);
    let cloth = h.c(h.cloth);

    // Cut the whole arm out of the body behind it first, in one piece,
    // so the separation follows the arm rather than each segment of it.
    let (ax, ay) = unit(shoulder, elbow);
    let sleeve = if h.build == Build::Bare { 6.6 } else { 7.4 };
    h.cut(blip, &[
        (V(shoulder.0 - ax * 3.0, shoulder.1 - ay * 3.0), (sleeve + 1.6) * b),
        (elbow, 6.4 * b),
        (hand, 6.0 * b),
    ]);

    // The deltoid: a cap over the joint, drawn before the arm and over the
    // torso, so the arm grows from the body.
    blob(blip, V(shoulder.0 - ax * 1.5, shoulder.1 - ay * 1.5), 7.0 * b, skin, ink);

    // Two bones, two strokes — as with the leg, and for the same
    // reason: the forearm's swell below the elbow is real and is four
    // pixels wide.
    part(blip, shoulder, elbow, 7.2 * b, 4.7 * b, skin, ink);
    part(blip, elbow, hand, 5.2 * b, 3.3 * b, skin, ink);

    // The sleeve.
    match h.build {
        Build::Gi => {
            let cuff = along(shoulder, elbow, 0.86);
            part(blip, V(shoulder.0 - ax * 2.0, shoulder.1 - ay * 2.0), cuff,
                 7.0 * b, 5.0 * b, cloth, ink);
            // Only the cuff hangs free; the sleeve above it is
            // stretched over a stiff upper arm.
            hang(blip, h, cuff, (ax * 0.5, ay * 0.5 + 0.7), 5.5,
                (t * 5.0 + shoulder.0 * 0.1).sin() * 1.4, 5.4 * b, 3.4 * b, cloth);
        }
        Build::Suit => {
            // A one-piece suit covers the whole arm to the wrist.
            part(blip, V(shoulder.0 - ax * 2.0, shoulder.1 - ay * 2.0), elbow,
                 7.4 * b, 5.2 * b, cloth, ink);
            part(blip, elbow, along(elbow, hand, 0.72), 5.2 * b, 4.0 * b, cloth, ink);
        }
        // Bare arms are the point of being bare — but the shoulder
        // still gets a strap, so the torso and the arm are not one
        // uninterrupted field of skin.
        Build::Bare => {
            stroke(blip, V(shoulder.0 - ax * 3.0, shoulder.1 - ay * 3.0),
                along(shoulder, elbow, 0.22), 6.4 * b, 5.6 * b, shade(skin, 0.82));
        }
    }

    // Wrist wrap, thin and short of the hand, so the fist never reads as a
    // boxing glove.
    stroke(blip, along(elbow, hand, 0.54), along(elbow, hand, 0.74), 3.0 * b, 3.2 * b,
        h.c(h.trim));
    if open { draw_palm(blip, h, elbow, hand, b); }
    else { draw_fist(blip, h, elbow, hand, b); }
}

/// The pelvis: a short bar between the two hip sockets. Without it the
/// thighs appear to sprout from the same point, and a fighter whose
/// legs share one origin walks like a pair of compasses.
fn draw_pelvis(blip: &Blip, h: &Hide, a: V, b: V) {
    let body = if h.build == Build::Bare { h.c(h.skin) } else { h.c(h.cloth) };
    part(blip, a, b, 9.4 * h.bulk, 9.4 * h.bulk, body, h.ink());
}

fn draw_torso(blip: &Blip, h: &Hide, hip: V, neck: V, t: f32) {
    let b = h.bulk;
    let ink = h.ink();
    let bare = h.build == Build::Bare;
    let body = if bare { h.c(h.skin) } else { h.c(h.cloth) };
    let chest = along(hip, neck, 0.76);

    // The trunk is a ribcage, not a pair of shoulders: shoulder width is the
    // ribcage plus the deltoids, drawn with the arms. The taper is steeper
    // than life, the sprite wedge that says heavyweight. Breathing moves the
    // ribcage, not the waist: half a pixel at rest, nearly two when spent.
    let swell = 1.0 + (0.055 + 0.075 * h.sweat) * h.breath;
    // And the shoulders ride up on it. Drawn, not posed: the neck is
    // where the skeleton put it, the top of the chest just reaches a
    // little further toward it on the way in.
    let top = along(chest, neck, 0.72 + 0.05 * h.breath);
    part(blip, hip, chest, 8.8 * b, 12.6 * b * swell, body, ink);
    part(blip, chest, top, 12.6 * b * swell, 6.4 * b, body, ink);

    let belt_a = along(hip, neck, 0.28);
    let belt_b = along(hip, neck, 0.40);
    let waist = along(belt_a, belt_b, 0.4);

    // The open gi is what says karate rather than pyjamas: bare
    // sternum down the middle, lapel crossing it to the belt.
    match h.build {
        Build::Gi => {
            let v = along(chest, neck, 0.30);
            let low = along(waist, chest, 0.42);
            stroke(blip, v, low, 4.6 * b, 2.0 * b, h.c(h.skin));
            part(blip, along(chest, neck, 0.58), low, 2.6, 1.9,
                shade(body, 0.70), shade(body, 0.52));
        }
        Build::Bare => {
            // Collarbone and the crease under the pectoral. Two marks,
            // and a bare chest stops being a slab of colour.
            let dark = shade(body, 0.70);
            stroke(blip, along(chest, neck, 0.18), along(chest, neck, 0.58), 1.9, 1.5, dark);
            stroke(blip, along(waist, chest, 0.70), along(chest, neck, 0.22), 2.0, 1.6, dark);
        }
        Build::Suit => {
            stroke(blip, along(waist, chest, 0.2), along(chest, neck, 0.45), 2.4, 1.9,
                h.c(h.trim));
        }
    }

    // And on the chest, over whatever is worn on it: the one broad
    // piece of a fighter, and so the one that can carry more than a
    // couple of beads.
    sweat_beads(blip, along(hip, neck, 0.60), 1.0, &[
        (-2.0 * b, -5.0 * b), (4.0 * b, -1.0 * b), (-5.0 * b, 3.0 * b),
        (2.0 * b, 6.0 * b), (6.0 * b, 4.0 * b),
    ], t + 0.7, h.sweat * 0.8, 27);
}

/// The gi skirt and belt, drawn after the legs: jacket over thighs, belt over
/// jacket.
fn draw_skirt(blip: &Blip, h: &Hide, hip: V, neck: V, t: f32) {
    let b = h.bulk;
    let ink = h.ink();
    let belt_a = along(hip, neck, 0.28);
    let belt_b = along(hip, neck, 0.40);
    let waist = along(belt_a, belt_b, 0.4);

    // One piece, flaring: a panel each side of the waist with a gap
    // between them reads as two tubes dangling off the fighter.
    if h.build == Build::Gi {
        let body = h.c(h.cloth);
        let sway = (t * 3.4).sin() * 1.3;
        let top = along(hip, neck, 0.33);
        hang(blip, h, top, (0.0, 1.0), 17.0, sway, 8.8 * b, 10.2 * b, body);
        let hem = V(top.0 + sway, top.1 + 17.0);
        stroke(blip, along(top, hem, 0.3), hem, 1.4, 1.1, shade(body, 0.66));
    }

    // A band across the body, not a capsule along the spine.
    let (sx, sy) = unit(hip, neck);
    let (px, py) = (-sy, sx);
    let w = 8.8 * b;
    part(blip, V(waist.0 - px * w, waist.1 - py * w),
               V(waist.0 + px * w, waist.1 + py * w), 3.4 * b, 3.4 * b, h.belt(), ink);
    // One end hanging off the knot, not two. Two of anything dangling
    // off the middle of a fighter reads as a pair of objects stuck on
    // them, whatever the two things actually are.
    let knot = V(waist.0 + (neck.0 - hip.0).signum() * 2.0, waist.1 + 1.0);
    hang(blip, h, knot, (0.0, 1.0), 9.0, (t * 4.6).sin() * 1.4,
        1.8 * b, 0.9 * b, h.belt());
}

fn draw_head(blip: &Blip, h: &Hide, rig: Rig, head: V, neck: V, t: f32) {
    let fw = rig.face;
    let ink = h.ink();
    let skin = h.c(h.skin);
    let hair = h.c(h.hair);

    // Six heads tall, not the classical seven and a half: arcade
    // sprites trade anatomy for a skull big enough to carry a face.
    // Every offset below is a multiple of `r` so the head scales whole.
    let r = 9.8 + 2.2 * (h.bulk - 1.0);

    // Neck first, so the head sits on it rather than beside it. Drawn
    // long enough to be a neck, and thick: a fighter's neck is a slab
    // of trapezius, and a head this size on a stalk is a lollipop.
    part(blip, neck, V(head.0 - fw * 0.17 * r, head.1 + 0.68 * r), 5.6, 4.7, skin, ink);

    // Skull, then the jaw hung off the front of it. The jaw is most of
    // what makes a profile male and all of what makes it set.
    blob(blip, head, r, skin, ink);
    part(blip, V(head.0 + fw * 0.05 * r, head.1 + 0.13 * r),
         V(head.0 + fw * 0.50 * r, head.1 + 0.61 * r), 0.66 * r, 0.43 * r, skin, ink);

    // Hair: two lumps behind the band, so the extra head size goes to the
    // face.
    blip.fill_circle(head.0 - fw * 0.40 * r, head.1 - 0.68 * r, 0.58 * r, hair);
    blip.fill_circle(head.0 - fw * 0.82 * r, head.1 - 0.02 * r, 0.52 * r, hair);

    // Headband at the hairline and one thin tie trailing behind.
    let band_a = V(head.0 - fw * 0.86 * r, head.1 - 0.40 * r);
    stroke(blip, band_a, V(head.0 + fw * 0.70 * r, head.1 - 0.52 * r),
        0.22 * r, 0.19 * r, h.c(h.trim));
    let sway = (t * 5.5).sin() * 3.0;
    stroke(blip, band_a, V(head.0 - fw * 2.0 * r, head.1 + 0.26 * r + sway),
        0.15 * r, 0.06 * r, h.c(h.trim));


    // A face: brow, eye, nose, mouth. The heavy brow over a small eye is the
    // look at the opponent; the nose is the profile.
    stroke(blip, V(head.0 + fw * 0.24 * r, head.1 - 0.16 * r),
           V(head.0 + fw * 0.74 * r, head.1 - 0.10 * r), 0.13 * r, 0.10 * r, hair);
    blip.fill_circle(head.0 + fw * 0.56 * r, head.1 + 0.14 * r, 0.16 * r, INK);
    blip.fill_circle(head.0 + fw * 0.82 * r, head.1 + 0.22 * r, 0.20 * r, skin);
    stroke(blip, V(head.0 + fw * 0.52 * r, head.1 + 0.56 * r),
           V(head.0 + fw * 0.76 * r, head.1 + 0.52 * r), 0.11 * r, 0.10 * r,
           shade(skin, 0.5));

    // Sweat, last of all: it stands on the face, so it goes on over the
    // face. Drawn before it, the brow and the eye painted straight back
    // over every bead and the fighter stayed bone dry.
    sweat_beads(blip, head, fw, &[
        // Low on the head, away from the brow (a still bead there reads as a
        // stud): on the jaw a bead can gather, run off the chin and fall.
        (0.44 * r, 0.62 * r),    // the point of the chin
        (0.10 * r, 0.74 * r),    // under the jaw
        (-0.36 * r, 0.52 * r),   // behind it, under the ear
    ], t, h.sweat, 11);
}

// ---- poses ---------------------------------------------------------------

/// Every joint that is placed rather than solved.
#[derive(Copy, Clone)]
pub(crate) struct Pose {
    hip: P,
    head: P,
    lead_hand: P,
    rear_hand: P,
    /// The near foot. Attacks with a leg are always thrown with this
    /// one, because the near leg is the one drawn in front of the body.
    lead_foot: P,
    rear_foot: P,
    /// Hands open — a grab, or a palm — rather than closed into fists.
    open: bool,
}

impl Pose {
    fn to(self, o: Pose, k: f32) -> Pose {
        Pose {
            hip: self.hip.to(o.hip, k),
            head: self.head.to(o.head, k),
            lead_hand: self.lead_hand.to(o.lead_hand, k),
            rear_hand: self.rear_hand.to(o.rear_hand, k),
            lead_foot: self.lead_foot.to(o.lead_foot, k),
            rear_foot: self.rear_foot.to(o.rear_foot, k),
            open: if k < 0.5 { self.open } else { o.open },
        }
    }
}

/// The neck, derived from the spine: most of the way up and a little behind, square to it.
pub(crate) const STRIDE: f32 = 0.1208;  // radians of cycle per pixel travelled
pub(crate) const STEP: f32 = 13.0;      // = (2*PI/STRIDE) / 4, or the foot skates
pub(crate) fn foot_cycle(ph: f32, lift: f32) -> (f32, f32) {
    let u = (ph / std::f32::consts::TAU).rem_euclid(1.0);
    if u < 0.5 {
        (STEP * (1.0 - 4.0 * u), 0.0)
    } else {
        let k = (u - 0.5) * 2.0;
        let e = k * k * (3.0 - 2.0 * k);
        (-STEP + 2.0 * STEP * e, lift * (k * std::f32::consts::PI).sin())
    }
}

/// The neck, derived from the spine: most of the way up and a little behind, square to it.
const SPINE: f32 = 0.73;
const SHOULDERS_BACK: f32 = 2.4;

fn neck_of(q: &Pose) -> P {
    let (dx, du) = (q.head.f - q.hip.f, q.head.u - q.hip.u);
    let len = (dx * dx + du * du).sqrt().max(0.001);
    let (ux, uu) = (dx / len, du / len);
    p(q.hip.f + ux * len * SPINE - uu * SHOULDERS_BACK,
      q.hip.u + uu * len * SPINE + ux * SHOULDERS_BACK)
}

/// The stance: weight back, knees bent, both hands up, side on. Everything departs from and returns to this.
/// `breath` runs -1..1 over one slow cycle; each part follows it at its own phase and scale, so it reads as breathing, not a bobbing float.
fn stance(breath: f32) -> Pose {
    let bob = (breath * 2.0).clamp(-1.0, 1.0);
    let roll = -breath;
    Pose {
        hip: p(breath * 1.2, HIP_U - 1.0 + bob * 0.9),
        head: p(2.0 + roll * 1.6, HEAD_U - 0.4 + bob * 0.6),
        lead_hand: p(35.0 + breath * 1.4, 77.0 + bob * 2.0),
        rear_hand: p(12.0 + breath * 0.8, 76.0 + bob * 1.6),
        lead_foot: p(24.0, 0.0),
        rear_foot: p(-22.0, 0.0),
        open: false,
    }
}

fn crouched(breath: f32) -> Pose {
    Pose {
        hip: p(-2.0, C_HIP + breath * 0.4),
        head: p(1.5, C_HEAD + breath * 0.4),
        lead_hand: p(26.0, 38.0 + breath * 0.6),
        rear_hand: p(8.0, 31.0),
        lead_foot: p(18.0, 0.0),
        rear_foot: p(-19.0, 0.0),
        open: false,
    }
}

/// Off the ground, posed from vertical speed: legs trailing at take-off,
/// tucked at the apex, reaching down on the way in, right at every jump
/// height.
fn airborne_pose(vy: f32) -> Pose {
    let r = (-vy / -JUMP_VY).clamp(-1.0, 1.0);
    let rise = r.max(0.0);
    let fall = (-r).max(0.0);
    let tuck = 1.0 - r.abs();
    let tuck = tuck * tuck * (3.0 - 2.0 * tuck);
    Pose {
        hip: p(-2.0 + 2.0 * fall, 48.0 + 4.0 * tuck),
        head: p(-3.0 - 4.0 * rise + 3.0 * fall, 99.0 + 3.0 * tuck),
        lead_hand: p(24.0 + 2.0 * rise + 6.0 * fall, 68.0 + 14.0 * rise - 2.0 * fall),
        rear_hand: p(8.0 + 2.0 * rise + 2.0 * fall, 60.0 + 16.0 * rise - 4.0 * fall),
        lead_foot: p(14.0 + 4.0 * tuck + 6.0 * fall, 8.0 + 24.0 * tuck),
        rear_foot: p(-16.0 + 6.0 * tuck + 2.0 * fall, 4.0 + 24.0 * tuck),
        open: false,
    }
}

/// Flat on the floor, head away from whoever put them there.
fn floored() -> Pose {
    Pose {
        hip: p(8.0, 13.0),
        head: p(-43.0, 9.5),
        lead_hand: p(30.0, 8.0),
        rear_hand: p(8.0, 26.0),
        lead_foot: p(26.0, 6.0),
        rear_foot: p(62.0, 8.0),
        open: false,
    }
}

/// How far through its own animation an attack is: out through startup,
/// held while it can hit, snapped back through recovery.
fn extension(f: &Fighter, m: &MoveData) -> f32 {
    let start = m.startup * F;
    let end = start + m.active * F;
    if f.t < start {
        let k = (f.t / start.max(0.0001)).clamp(0.0, 1.0);
        const COIL: f32 = 0.28;
        if k < COIL {
            return -0.16 * (std::f32::consts::PI * k / COIL).sin();
        }
        let k = (k - COIL) / (1.0 - COIL);
        k * k * (3.0 - 2.0 * k)
    } else if f.t <= end {
        1.0
    } else {
        let k = ((f.t - end) / (m.recovery * F).max(0.0001)).clamp(0.0, 1.0);
        const SNAP: f32 = 0.6;
        let back = (k / SNAP).min(1.0);
        let home = 1.0 - back * back * (3.0 - 2.0 * back);
        let settle = if k > 0.45 {
            -0.10 * (std::f32::consts::PI * (k - 0.45) / 0.55).sin()
        } else {
            0.0
        };
        home + settle
    }
}

/// Every number a kick is made of, in one place.
struct KickShape {
    /// How far the hips drive forward over the support foot.
    hip_drive: f32,
    /// How much they rise doing it. Up, not down: you push off.
    hip_rise: f32,
    /// How far the shoulders fall back to pay for the hips.
    lean: f32,
    /// The same lean in the air, as a fraction — there is nothing to
    /// counterbalance against up there, and a jump kick thrown lying
    /// back is a fighter falling.
    air_lean: f32,
    /// Where the support foot ends up relative to the hips.
    plant: f32,
    /// How far in front of the hip the chambered knee points.
    knee_lead: f32,
    /// Height of the chambered foot, hanging below that knee.
    chamber: f32,
    chamber_air: f32,
}

const KICK: KickShape = KickShape {
    hip_drive: 11.0,
    hip_rise: 2.0,
    lean: 17.0,
    air_lean: 0.3,
    plant: 5.0,
    knee_lead: 16.0,
    chamber: 31.0,
    chamber_air: 25.0,
};

/// Kick progress: 0..1 for the chamber, 1..2 for the extension. The knee comes up fast and waits (the frame a defender reads), then the shin snaps out.
fn kick_curve(ext: f32) -> f32 {
    const SPLIT: f32 = KICK_SNAP;
    if ext < SPLIT {
        let k = ext / SPLIT;
        1.0 - (1.0 - k) * (1.0 - k)
    } else {
        let k = (ext - SPLIT) / (1.0 - SPLIT);
        1.0 + k * k
    }
}

/// The pose an attack is in, `ext` of the way through it.
/// How far the foot reaches past the ankle. Kicks aim the ankle short by this much, or the toes
/// overshoot the hitbox; keep in step with the toe in `draw_leg`.
const FOOT: f32 = 12.6;

/// Same for a punch: how far the fist front is past the wrist, so a jab never draws longer than its range.
const FIST: f32 = 6.4;

fn attack_pose(q: &mut Pose, f: &Fighter, m: &MoveData, ext: f32) {
    let end = p(BODY_W / 2.0 + m.reach, m.height);
    let bulk = f.arch().bulk;
    let tip = p(end.f - FIST * bulk, end.u);
    let toe_tip = p(end.f - FOOT * bulk, end.u);

    enum Shape { Punch(bool), Kick(bool, f32), Fly, Sweep, Throw, Bolt, Rush }
    let shape = match f.mv {
        MoveId::LowPunch | MoveId::HighPunch => Shape::Punch(false),
        MoveId::JumpPunch => Shape::Punch(true),
        MoveId::LowKick => Shape::Kick(false, 0.7),
        MoveId::HighKick => Shape::Kick(false, 1.25),
        MoveId::JumpKick => Shape::Kick(true, 1.0),
        MoveId::FlyingKick => Shape::Fly,
        MoveId::Sweep => Shape::Sweep,
        MoveId::Throw => Shape::Throw,
        MoveId::Special => match f.arch().special {
            Special::ChiBolt => Shape::Bolt,
            Special::BullRush => Shape::Rush,
            Special::TalonKick => Shape::Kick(false, 1.25),
        },
    };

    match shape {
        Shape::Punch(air) => {
            q.hip.f += if air { 12.0 } else { 5.0 } * ext;
            q.head.f += if air { 24.0 } else { 22.0 } * ext;
            if !air {
                q.lead_foot.f += 5.0 * ext;
                q.rear_foot.f -= 3.0 * ext;
            } else {
                q.lead_foot = p(q.hip.f + 14.0, q.lead_foot.u);
                q.rear_foot = p(q.hip.f - 10.0, q.rear_foot.u.min(24.0));
            }
            q.rear_hand = q.rear_hand.to(p(q.rear_hand.f - 4.0, q.rear_hand.u - 22.0), ext);
            q.lead_hand = q.lead_hand.to(tip, ext);
        }
        Shape::Kick(air, lift) => {
            let k = KICK;
            let back = if air { k.air_lean } else { 1.0 } * lift;
            let drive = if air { k.hip_drive + 9.0 } else { k.hip_drive };
            q.hip = p(q.hip.f + drive * ext, q.hip.u + k.hip_rise * ext);
            q.head.f -= k.lean * 1.25 * ext * back;
            q.head.u -= 1.4 * ext;
            if !air {
                q.rear_foot = p(-18.0 + (q.hip.f - k.plant - -18.0) * ext, 0.0);
            } else {
                q.rear_foot = p(q.hip.f - 7.0, 26.0);
            }
            let chamber = p(q.hip.f + k.knee_lead, if air { k.chamber_air } else { k.chamber });
            let kc = kick_curve(ext);
            q.lead_foot = if kc < 1.0 {
                q.lead_foot.to(chamber, kc)
            } else {
                chamber.to(toe_tip, kc - 1.0)
            };
            q.lead_hand = q.lead_hand.to(p(30.0, 62.0), ext);
            q.rear_hand = q.rear_hand.to(p(-4.0, 44.0), ext);
        }
        Shape::Fly => {
            q.hip = p(q.hip.f + 21.0 * ext, q.hip.u + 7.0 * ext);
            q.head = p(q.head.f - 22.0 * ext, q.head.u - 13.0 * ext);
            q.rear_foot = p(q.hip.f - 24.0 - 10.0 * ext, 14.0 + 4.0 * ext);
            q.lead_foot = q.lead_foot.to(toe_tip, ext);
            q.lead_hand = q.lead_hand.to(p(32.0, 70.0), ext);
            q.rear_hand = q.rear_hand.to(p(-28.0, 40.0), ext);
        }
        Shape::Sweep => {
            let hip_f = (tip.f - 52.0).clamp(2.0, 28.0);
            q.hip = p(hip_f * ext, C_HIP + 3.0 * ext);
            q.head = p(q.head.f - 4.0 * ext, C_HEAD - 7.0 * ext);
            q.rear_foot = p(-3.0, 2.0);
            q.lead_foot = q.lead_foot.to(toe_tip, ext);
            q.rear_hand = q.rear_hand.to(p(-26.0, 6.0), ext);
            q.lead_hand = q.lead_hand.to(p(34.0, 44.0), ext);
        }
        Shape::Throw => {
            q.open = true;
            q.hip.f += 4.0 * ext;
            q.head.f += 8.0 * ext;
            q.lead_foot.f += 7.0 * ext;
            q.lead_hand = q.lead_hand.to(p(tip.f, tip.u + 6.0), ext);
            q.rear_hand = q.rear_hand.to(p(tip.f - 6.0, tip.u - 8.0), ext);
        }
        Shape::Bolt => {
            q.open = true;
            q.hip = p(2.0, 51.0);
            q.lead_foot = p(21.0, 0.0);
            q.rear_foot = p(-22.0, 0.0);
            let charge = p(-17.0, 57.0);
            let out = p(44.0, 70.0);
            if ext < 0.55 {
                let k = ext / 0.55;
                q.lead_hand = q.lead_hand.to(charge, k);
                q.rear_hand = q.rear_hand.to(p(charge.f + 5.0, charge.u - 9.0), k);
                q.head.f -= 6.0 * k;
            } else {
                let k = (ext - 0.55) / 0.45;
                q.lead_hand = charge.to(out, k);
                q.rear_hand = p(charge.f + 5.0, charge.u - 9.0).to(p(out.f - 18.0, out.u - 10.0), k);
                q.head.f += -6.0 + 14.0 * k;
            }
        }
        Shape::Rush => {
            q.hip = p(8.0 * ext, HIP_U - 5.0 * ext);
            q.head.f += 20.0 * ext;
            q.lead_foot = p(15.0 + 17.0 * ext, 5.0 * ext);
            q.rear_foot.f -= 11.0 * ext;
            q.lead_hand = q.lead_hand.to(tip, ext);
            q.rear_hand = q.rear_hand.to(p(-6.0, 50.0), ext);
        }
    }
}

/// The pose a fighter is in this frame.
/// The pose a fighter is drawn in, including the tail of the one before it: actions change on
/// one frame, so the hand-over is eased across four frames (the hitbox still follows the move).
pub(crate) fn pose_of(now: f32, f: &Fighter, idx: usize) -> Pose {
    let q = pose_now(now, f, idx);
    if f.blend <= 0.0 { return q; }
    let mut was = *f;
    was.act = f.prev_act;
    was.mv = f.prev_mv;
    was.t = f.prev_t;
    was.blend = 0.0;
    was.y = if f.prev_air { FLOOR_Y - 1.0 } else { FLOOR_Y };
    was.vy = f.prev_vy;
    let k = (1.0 - f.blend).clamp(0.0, 1.0);
    pose_now(now, &was, idx).to(q, k * k * (3.0 - 2.0 * k))
}

/// How hard this fighter is working: nothing at all at full health,
/// everything when there is almost none left.
pub(crate) fn exertion(f: &Fighter) -> f32 {
    let max = FIGHTERS[f.who].health.max(1) as f32;
    (1.0 - f.health as f32 / max).clamp(0.0, 1.0)
}

/// One breath, -1 to 1: the slow breath, plus a faster ripple once hurt.
/// Rates are fixed and only amplitude grows with exertion, since a
/// health-driven rate would jump the phase when a blow lands.
pub(crate) fn breath_of(now: f32, f: &Fighter, idx: usize) -> f32 {
    let hard = exertion(f);
    let slow = (now * 2.6 + idx as f32 * 2.3).sin();
    let pant = (now * 7.4 + idx as f32 * 1.1).sin();
    (slow * (1.0 + 0.55 * hard) + pant * 0.42 * hard).clamp(-1.6, 1.6)
}

/// Time into a fall if this fighter is down: knocked down, or beaten by KO
/// (who stays flat instead of kneeling; a time-up loser kneels).
fn down_time(f: &Fighter) -> Option<f32> {
    match f.act {
        Act::Knockdown => Some(f.t),
        Act::Defeat if f.health <= 0 => Some(f.t.min(0.6)),
        _ => None,
    }
}

fn pose_now(now: f32, f: &Fighter, idx: usize) -> Pose {
    let breath = breath_of(now, f, idx);

    if let Some(t) = down_time(f) {
        let down = floored();
        if t < 0.22 {
            let k = (t / 0.22).clamp(0.0, 1.0);
            let mut air = stance(0.0);
            air.hip = p(-6.0, 48.0);
            air.head = p(-40.0, 62.0);
            air.lead_foot = p(30.0, 44.0);
            air.rear_foot = p(22.0, 24.0);
            air.lead_hand = p(-8.0, 82.0);
            air.rear_hand = p(-30.0, 66.0);
            return air.to(down, k * k);
        }
        if t > 0.85 {
            let k = ((t - 0.85) / 0.30).clamp(0.0, 1.0);
            let mut up = crouched(0.0);
            up.rear_hand = p(22.0, 16.0);
            up.lead_hand = p(28.0, 34.0);
            up.rear_foot = p(-10.0, 0.0);
            return down.to(up, (k * 1.8).min(1.0)).to(stance(0.0), (k - 0.55).max(0.0) / 0.45);
        }
        return down;
    }

    let low = f.crouching()
        || (f.act == Act::Hitstun && matches!(f.prev_act, Act::Crouch))
        || (f.act == Act::Hitstun && f.prev_act == Act::Attack
            && matches!(f.prev_mv, MoveId::Sweep))
        || (f.act == Act::Hitstun && f.prev_act == Act::Block && f.crouch_block);
    let mut q = if f.airborne() {
        airborne_pose(f.vy)
    } else if low {
        crouched(breath)
    } else {
        stance(breath)
    };

    match f.act {
        Act::Walk => {
            let ph = f.x * f.facing * STRIDE;
            let s = ph.sin();
            let (lf, ll) = foot_cycle(ph, 9.0);
            let (rf, rl) = foot_cycle(ph + std::f32::consts::PI, 8.0);
            q.lead_foot = p(18.0 + lf, ll);
            q.rear_foot = p(-16.0 + rf, rl);
            let sink = 1.4 + 1.4 * (2.0 * ph).cos();
            q.hip.u -= sink;
            q.head.u -= sink * 0.7;
            q.hip.f += 1.4 * s;
            q.head.f -= 1.0 * s;
            q.lead_hand.f -= 2.6 * s;
            q.rear_hand.f += 2.2 * s;
            q.lead_hand.u -= sink * 0.7;
            q.rear_hand.u -= sink * 0.7;
        }
        // Turtled up: weight off the front foot, forearms stacked before head or belly.
        Act::Block => {
            let b = breath;
            if f.crouch_block {
                q.lead_hand = p(25.0, 41.0 + b * 0.7);
                q.rear_hand = p(11.0, 31.0 + b * 0.5);
                q.head.f -= 4.0;
            } else {
                q.hip = p(-4.0, HIP_U - 2.0 + b * 0.6);
                q.head = p(-8.0 - b * 0.7, HEAD_U - 2.0 + b * 0.4);
                q.lead_hand = p(23.0 + b * 0.6, 88.0 + b * 0.8);
                q.rear_hand = p(12.0 + b * 0.4, 74.0 + b * 0.6);
                let ph = f.x * f.facing * STRIDE;
                let sink = 1.0 + (2.0 * ph).cos();
                let (lf, ll) = foot_cycle(ph, 7.0);
                let (rf, rl) = foot_cycle(ph + std::f32::consts::PI, 6.0);
                q.lead_foot = p(14.0 + lf, ll);
                q.rear_foot = p(-17.0 + rf, rl);
                q.hip.u -= sink;
                q.head.u -= sink * 0.7;
                q.lead_hand.u -= sink * 0.7;
                q.rear_hand.u -= sink * 0.7;
            }
        }
        // Snapped back off the blow: head first, back foot skidding out.
        Act::Hitstun => {
            let sag = (1.0 - f.t * 6.0).clamp(0.30, 1.0);
            let ring = 0.32 * (-f.t * 8.0).exp() * ((f.t * 30.0).cos() - 1.0);
            let k = sag + ring;
            let drop = if low { 0.5 } else { 1.0 };
            q.hip = p(q.hip.f - 6.0 * k, q.hip.u - 3.0 * k);
            q.head = p(q.head.f - 24.0 * k * drop, q.head.u - 1.6);
            q.lead_foot = p(q.lead_foot.f - 10.0, 0.0);
            q.rear_foot = p(q.rear_foot.f - 6.0, 0.0);
            q.lead_hand = p(q.lead_hand.f - 9.0, q.lead_hand.u - 5.0);
            q.rear_hand = p(q.rear_hand.f - 13.0, q.rear_hand.u - 9.0);
        }
        Act::Attack => {
            let m = f.scaled(move_data(f.mv));
            let ext = extension(f, &m);
            attack_pose(&mut q, f, &m, ext);
        }
        // Won: guard drops, the arm sweeps up and forward (raised straight it hides the face), then held.
        Act::Victory => {
            let t = f.t;
            let settle = (t / 0.30).clamp(0.0, 1.0);
            let raise = ((t - 0.26) / 0.34).clamp(0.0, 1.0);
            let sweep = raise * raise * (3.0 - 2.0 * raise);
            let bob = ((t - 0.6).max(0.0) * 2.2).sin() * 1.8;

            q.head = p(-1.5 + 2.0 * settle - 2.0 * sweep,
                       HEAD_U + 1.2 * settle + 1.5 * sweep + bob * 0.3);
            q.hip = p(0.0, HIP_U + 1.0 * settle);
            q.lead_foot = p(16.0 - 3.0 * settle, 0.0);
            q.rear_foot = p(-21.0 + 4.0 * settle, 0.0);
            q.rear_hand = q.rear_hand.to(p(-4.0, 46.0), settle);
            let load = p(20.0, 58.0);
            let up = p(36.0, 120.0 + bob);
            q.lead_hand = if raise <= 0.0 {
                q.lead_hand.to(load, settle)
            } else {
                load.to(up, sweep)
            };
        }
        // The rei: heels together at attention, open hands at the thighs; a
        // bow of about thirty degrees from the hips toward the opponent,
        // held a beat, and up again. (The guard comes up after, in the
        // blend into Idle.)
        Act::Bow => {
            let ease = |from: f32, len: f32| {
                let k = ((f.t - from) / len).clamp(0.0, 1.0);
                k * k * (3.0 - 2.0 * k)
            };
            let bow = ease(0.3, 0.42) * (1.0 - ease(1.0, 0.42));
            // standing tall: legs all but straight, the hips going back as
            // the torso tips forward
            let hip = p(-6.0 * bow, HIP_U + 5.5 - 1.5 * bow);
            let spine = HEAD_U - HIP_U + 1.0;
            let lean = 0.62 * bow; // about 35 degrees at the bottom
            let head = p(hip.f + spine * lean.sin() + 3.0 * bow, hip.u + spine * lean.cos() - 3.0 * bow);
            q = Pose {
                hip,
                head,
                // open hands at the sides of the thighs, sliding toward the knees
                lead_hand: p(3.0 + 12.0 * bow, 46.0 - 12.0 * bow),
                rear_hand: p(-3.0 + 12.0 * bow, 47.0 - 12.0 * bow),
                lead_foot: p(6.0, 0.0),
                rear_foot: p(-6.0, 0.0),
                open: true,
            };
        }
        Act::Defeat => {
            let fall = |from: f32, len: f32| {
                let k = ((f.t - from) / len).clamp(0.0, 1.0);
                k * k * (3.0 - 2.0 * k)
            };
            let (hips, slump) = (fall(0.0, 0.55), fall(0.2, 0.8));
            let heave = (f.t * 2.4).sin() * slump;
            q.hip = q.hip.to(p(-4.0, 30.0), hips);
            q.head = q.head.to(p(25.0, 72.0 + heave * 0.8), slump);
            q.lead_foot = q.lead_foot.to(p(22.0, 0.0), hips);
            q.rear_foot = q.rear_foot.to(p(-34.0, 3.0), hips);
            q.lead_hand = q.lead_hand.to(p(28.0, 28.0), slump);
            q.rear_hand = q.rear_hand.to(p(0.0, 18.0 + heave * 0.5), slump);
        }
        _ => {}
    }
    if f.land > 0.0 && !f.airborne()
        && matches!(f.act, Act::Idle | Act::Walk | Act::Crouch | Act::Block) {
        let k = 1.0 - (f.land / LAND_ABSORB).clamp(0.0, 1.0);
        let room = if low { 0.3 } else { 1.0 };
        let dip = (std::f32::consts::PI * k).sin() * (1.0 - k) * f.land_force * room;
        q.hip.u -= 9.0 * dip;
        q.head.u -= 6.5 * dip;
        q.lead_hand.u -= 6.0 * dip;
        q.rear_hand.u -= 6.0 * dip;
        q.lead_foot.f += 2.0 * dip;
        q.rear_foot.f -= 2.0 * dip;
    }

    if !f.airborne() && f.act != Act::Knockdown {
        q.lead_foot.u = q.lead_foot.u.max(0.0);
        q.rear_foot.u = q.rear_foot.u.max(0.0);
    }
    q
}

/// The whole fighter, smeared by what moved: pose the fighter a few frames back and draw each
/// past bone in proportion to how far it has travelled since. Fast limbs streak, still ones stay sharp.
fn draw_smear(blip: &Blip, f: &Fighter, now: f32, shift: f32, i: usize, c: BlipColor) {
    let here = skeleton(Rig::upright(f.x, f.y + shift, f.facing, f.facing),
        &pose_of(now, f, i));
    for step in 1..=5 {
        let back = step as f32 * 2.2 * F;
        if f.t < back { break; }
        let mut old = *f;
        old.t = f.t - back;
        // Where they were, not just what they were doing: this move
        // carries the whole body forward, and a smear drawn on the spot
        // is a fighter vibrating rather than travelling.
        old.x = f.x - f.vx * back;
        old.y = f.y - f.vy * back + 0.5 * GRAVITY * back * back;
        let then = skeleton(Rig::upright(old.x, old.y + shift, old.facing, old.facing),
            &pose_of(now - back, &old, i));
        // Older ghosts are fainter, and the whole thing fades out over
        // the move's last frames so it does not hang about after the
        // fighter has stopped.
        let age = 1.0 - (step - 1) as f32 / 5.0;
        for (a0, b0, a1, b1, r) in [
            (here.hip, here.neck, then.hip, then.neck, 9.0f32),
            (here.hip_lead, here.knee_lead, then.hip_lead, then.knee_lead, 7.0),
            (here.knee_lead, here.ankle_lead, then.knee_lead, then.ankle_lead, 5.5),
            (here.hip_rear, here.knee_rear, then.hip_rear, then.knee_rear, 6.0),
            (here.knee_rear, here.ankle_rear, then.knee_rear, then.ankle_rear, 4.5),
            (here.sh_lead, here.elbow_lead, then.sh_lead, then.elbow_lead, 5.5),
            (here.elbow_lead, here.hand_lead, then.elbow_lead, then.hand_lead, 4.5),
            (here.sh_rear, here.elbow_rear, then.sh_rear, then.elbow_rear, 5.0),
            (here.elbow_rear, here.hand_rear, then.elbow_rear, then.hand_rear, 4.0),
        ] {
            // How far this bone went. Nothing below a couple of pixels
            // is motion — it is the pose breathing.
            let moved = (dist(a0, a1) + dist(b0, b1)) * 0.5;
            if moved < 3.0 { continue; }
            let lit = ((moved - 3.0) / 34.0).clamp(0.0, 1.0) * age;
            stroke(blip, a1, b1, r, r * 0.72, BlipColor { a: 0.25 * lit, ..c });
        }
    }
}

/// Where the striking limb was a few frames ago, so a fast blow reads as one limb, not three.
fn draw_trail(blip: &Blip, f: &Fighter, now: f32, rig: Rig, i: usize, trim: BlipColor) {
    if f.act != Act::Attack { return; }
    let m = f.scaled(move_data(f.mv));
    let end = (m.startup + m.active) * F;
    if f.t > end + 2.0 * F { return; }
    let leg = is_kick(f);
    for k in 1..=3 {
        let back = k as f32 * 2.2 * F;
        if f.t < back { continue; }
        let mut old = *f;
        old.t = f.t - back;
        let q = pose_of(now, &old, i);
        let bones = skeleton(rig, &q);
        let (root, tip) = if leg { (bones.knee_lead, bones.ankle_lead) }
                          else { (bones.elbow_lead, bones.hand_lead) };
        let a = 0.22 - 0.06 * k as f32;
        stroke(blip, root, tip, 5.0, 3.4,
            BlipColor { r: trim.r, g: trim.g, b: trim.b, a });
    }
}

/// The fighter, flattened onto the floor: the same skeleton and solver, squashed and sheared
/// toward the light, drawn opaque so overlapping limbs leave no seams.
fn draw_shadow(blip: &Blip, rig: Rig, q: &Pose, bulk: f32, c: BlipColor) {
    let k = skeleton(rig, q);
    let b = bulk;
    for (hip, knee, ankle) in [(k.hip_rear, k.knee_rear, k.ankle_rear),
                               (k.hip_lead, k.knee_lead, k.ankle_lead)] {
        stroke(blip, hip, knee, 7.0 * b, 5.0 * b, c);
        stroke(blip, knee, ankle, 5.0 * b, 3.0 * b, c);
    }
    for (sh, elbow, hand) in [(k.sh_rear, k.elbow_rear, k.hand_rear),
                              (k.sh_lead, k.elbow_lead, k.hand_lead)] {
        stroke(blip, sh, elbow, 5.5 * b, 4.0 * b, c);
        stroke(blip, elbow, hand, 4.0 * b, 3.5 * b, c);
    }
    stroke(blip, k.hip_rear, k.hip_lead, 8.0 * b, 8.0 * b, c);
    stroke(blip, k.hip, k.neck, 9.5 * b, 12.0 * b, c);
    blip.fill_circle(k.head.0, k.head.1, 7.5 * b, c);
}

/// Every joint of one fighter in screen coordinates, after the solver. The fighter, shadow and
/// smear are all drawn from it and the anatomy tests run against it.
#[derive(Copy, Clone, Debug)]
pub(crate) struct Skeleton {
    pub hip: V,
    pub neck: V,
    pub head: V,
    pub hip_lead: V,
    pub hip_rear: V,
    pub sh_lead: V,
    pub sh_rear: V,
    pub knee_lead: V,
    pub knee_rear: V,
    pub ankle_lead: V,
    pub ankle_rear: V,
    pub elbow_lead: V,
    pub elbow_rear: V,
    pub hand_lead: V,
    pub hand_rear: V,
}

#[cfg_attr(not(test), allow(dead_code))]
impl Skeleton {
    /// The solved bones, with the length each one is supposed to be.
    /// A bone whose ends are not that far apart is a bone that got
    /// stretched, and a stretched bone is a joint in the wrong place.
    pub fn bones(&self) -> [(&'static str, V, V, f32); 8] {
        [
            ("thigh-lead", self.hip_lead, self.knee_lead, THIGH),
            ("shin-lead", self.knee_lead, self.ankle_lead, SHIN),
            ("thigh-rear", self.hip_rear, self.knee_rear, THIGH),
            ("shin-rear", self.knee_rear, self.ankle_rear, SHIN),
            ("upperarm-lead", self.sh_lead, self.elbow_lead, UARM),
            ("forearm-lead", self.elbow_lead, self.hand_lead, FARM),
            ("upperarm-rear", self.sh_rear, self.elbow_rear, UARM),
            ("forearm-rear", self.elbow_rear, self.hand_rear, FARM),
        ]
    }

    /// Each hinge, as (name, the joint, its two neighbours, how far it
    /// is allowed to shut).
    pub fn hinges(&self) -> [(&'static str, V, V, V, f32); 4] {
        [
            ("knee-lead", self.knee_lead, self.hip_lead, self.ankle_lead, KNEE_SHUT),
            ("knee-rear", self.knee_rear, self.hip_rear, self.ankle_rear, KNEE_SHUT),
            ("elbow-lead", self.elbow_lead, self.sh_lead, self.hand_lead, ELBOW_SHUT),
            ("elbow-rear", self.elbow_rear, self.sh_rear, self.hand_rear, ELBOW_SHUT),
        ]
    }
}

/// Solve one fighter's pose into joints. `rig` maps pose space to the screen,
/// for the fighter or their flattened shadow.
pub(crate) fn skeleton(rig: Rig, q: &Pose) -> Skeleton {
    let hip = rig.at(q.hip);
    let neck = rig.at(neck_of(q));
    let head = rig.at(q.head);

    // Sockets hang off the spine and turn with it; fixed screen offsets put
    // the arm out of the wrong side of a leaning chest.
    let (sx, sy) = unit(hip, neck);
    let (px, py) = (-sy, sx); // across the body, square to the spine
    let socket = |at: V, half: f32, up: f32| {
        (V(at.0 + px * half * rig.fwd + sx * up, at.1 + py * half * rig.fwd + sy * up),
         V(at.0 - px * half * rig.fwd + sx * up, at.1 - py * half * rig.fwd + sy * up))
    };
    // Sockets well out from the spine: that span plus the deltoids is the
    // shoulders, the wedge a fighting sprite is drawn as.
    let (sh_lead, sh_rear) = socket(neck, 8.6, -8.5);
    let (hip_lead, hip_rear) = socket(hip, 4.5, 0.0);

    // Parallax on the far side: the far arm and leg sit slightly back, so a
    // guard shows two fists and a stance two feet. Applied to the target so
    // bone lengths stay. The camera is at chest height: a far foot appears
    // higher (toward the horizon), a far hand barely moves; shifting the foot
    // down puts it through the floor.
    let hand_back = |v: V| V(v.0 - rig.fwd * 3.0, v.1 + 1.0);
    let foot_back = |v: V| V(v.0 - rig.fwd * 3.5, v.1 - 1.5);

    // Bones scale with the rig, so a squashed shadow bends where a
    // squashed shadow should bend. Upright, this is 1.0.
    let b = rig.bone();
    let leg_at = |root: V, want: V| {
        // The knee's bend side turns with the thigh: a leg thrown out
        // horizontally has its anatomical forward pointing up, and a fixed
        // "forward" inverted the knee on the high kicks. So it comes from the
        // limb's own direction, a quarter turn from it.
        let (dx, dy) = unit(root, want);
        let anterior = V(dy * rig.fwd, -dx * rig.fwd);
        solve(root, want, THIGH * b, SHIN * b, KNEE_SHUT, anterior, rig.ground)
    };
    let arm_at = |root: V, want: V| {
        // The elbow breaks to the back of the arm and turns with it. A fixed
        // direction sat near the tie between the two mirrored solutions, so
        // the forearm flipped with a pixel of movement; squared to the arm it
        // is as far from the tie as possible. One rule at every angle (see
        // `no_elbow_sticks_out_behind_the_back`).
        let (dx, dy) = unit(root, want);
        let posterior = V(-dy * rig.fwd, dx * rig.fwd);
        solve(root, want, UARM * b, FARM * b, ELBOW_SHUT, posterior, rig.ground)
    };

    let (knee_lead, ankle_lead) = leg_at(hip_lead, rig.at(q.lead_foot));
    let (knee_rear, ankle_rear) = leg_at(hip_rear, foot_back(rig.at(q.rear_foot)));
    let (elbow_lead, hand_lead) = arm_at(sh_lead, rig.at(q.lead_hand));
    let (elbow_rear, hand_rear) = arm_at(sh_rear, hand_back(rig.at(q.rear_hand)));

    Skeleton {
        hip, neck, head, hip_lead, hip_rear, sh_lead, sh_rear,
        knee_lead, knee_rear, ankle_lead, ankle_rear,
        elbow_lead, elbow_rear, hand_lead, hand_rear,
    }
}

fn unit(a: V, b: V) -> (f32, f32) {
    let d = dist(a, b).max(0.001);
    ((b.0 - a.0) / d, (b.1 - a.1) / d)
}

// ---- fighters ------------------------------------------------------------

/// One fighter, posed and built out of bones. Drawing order is depth: far
/// leg, far arm, body, head, near leg, near arm, so an attack arrives in
/// front of the body that threw it.
fn draw_fighter(blip: &Blip, g: &Game, i: usize) {
    let shake = if g.shake > 0.0 { (g.shake * 60.0).sin() * g.shake * 30.0 } else { 0.0 };
    // The docks' sun is low on the right, the temple's moon high on the left;
    // shadows fall away from it.
    let light = if g.stage == 0 { -1.0 } else { 1.0 };

    // In an exchange the two fighters overlap four frames in five, so in
    // versus a mark in each player's colour on the boards under them shows
    // which is yours.
    if g.mode == Mode::Versus {
        let c = if i == 0 { BlipColor { r: 1.0, g: 0.35, b: 0.35, a: 0.55 } }
                else { BlipColor { r: 0.40, g: 0.72, b: 1.0, a: 0.55 } };
        let f = &g.p[i];
        let w = 21.0 * f.arch().bulk;
        stroke(blip, V(f.x - w, FLOOR_Y + shake + 2.0), V(f.x + w, FLOOR_Y + shake + 2.0),
            2.2, 2.2, c);
    }
    pose_and_draw_lit(blip, &g.p[i], g.now, shake, g.hitstop, i, light);
}

/// `shift` moves the fighter down the screen without the floor: the hit
/// shake, and the gallery's row offset. Not part of `f.y`, which is where the
/// fighter is for the rules.
fn pose_and_draw(blip: &Blip, f: &Fighter, now: f32, shift: f32, hitstop: f32, i: usize) {
    pose_and_draw_lit(blip, f, now, shift, hitstop, i, 0.0);
}

/// `light` is which way shadows fall: negative for a light source off to
/// the right, positive for one off to the left, and zero for no cast
/// shadow at all — the select screen, which has no floor to cast on.
fn pose_and_draw_lit(blip: &Blip, f: &Fighter, now: f32, shift: f32, hitstop: f32, i: usize,
                     light: f32) {
    let f = *f;
    let a = f.arch();
    let prone = down_time(&f).is_some_and(|t| t < 0.9);

    let rig = Rig::upright(f.x, f.y + shift, f.facing, if prone { -f.facing } else { f.facing });

    let flash = if hitstop > 0.0 && f.act == Act::Hitstun { 0.5 }
                else if invulnerable(&f) && ((now * 30.0) as i32) % 2 == 0 { 0.3 }
                else { 0.0 };
    let hide = Hide {
        cloth: rgb(a.color),
        trim: rgb(a.trim),
        skin: rgb(a.skin),
        hair: rgb(a.hair),
        build: a.build,
        bulk: a.bulk,
        tone: 1.0,
        flash,
        breath: breath_of(now, &f, i),
        sweat: exertion(&f),
        front: false,
    };
    let far = hide.far();
    let near = hide.infront();

    // The smear is the moving thing's colour washed to white, reading as a
    // ribbon; the flying kick smears the whole body, every other attack one
    // limb.
    if f.act == Act::Attack && f.mv == MoveId::FlyingKick {
        draw_smear(blip, &f, now, shift, i, blend(rgb(a.color), BLIP_WHITE, 0.62));
    } else {
        draw_trail(blip, &f, now, rig, i, blend(rgb(a.color), BLIP_WHITE, 0.55));
    }

    // Dust off the boards where a body lands. Knockdowns are the one
    // moment the floor is part of the fight.
    if let Some(t) = down_time(&f).filter(|t| *t < 0.26) {
        let k = (1.0 - t / 0.26).max(0.0);
        for j in 0..5 {
            let dx = (j as f32 - 2.0) * 13.0 - f.facing * 14.0;
            let rise = (1.0 - k) * 16.0;
            blip.fill_circle(f.x + dx, FLOOR_Y + shift - 3.0 - rise, 3.0 + 7.0 * (1.0 - k),
                BlipColor { r: 0.62, g: 0.55, b: 0.46, a: 0.34 * k });
        }
    }

    let q = pose_of(now, &f, i);

    // Cast shadow, then the contact patch under the feet (what glues the body
    // down), then the fighter.
    if light != 0.0 {
        let shadow = Rig {
            cx: f.x,
            ground: FLOOR_Y + shift,
            fwd: f.facing,
            face: f.facing,
            squash: -0.24,
            skew: light * 0.42,
            rise: FLOOR_Y - f.y,
        };
        draw_shadow(blip, shadow, &q, a.bulk, SHADOW);
    }
    let lift = ((FLOOR_Y - f.y) / 120.0).clamp(0.0, 1.0);
    let sw = 19.0 * a.bulk * (1.0 - 0.5 * lift);
    stroke(blip, V(f.x - sw, FLOOR_Y + shift), V(f.x + sw, FLOOR_Y + shift), 2.6, 2.6,
        BlipColor { r: 0.0, g: 0.0, b: 0.0, a: 0.30 * (1.0 - 0.55 * lift) });

    // One solve per fighter. The cloth wanted a second pose four
    // frames back to difference the joints, which cost eight.
    let k = skeleton(rig, &q);
    let t = now + i as f32;

    draw_leg(blip, &far, k.hip_rear, k.knee_rear, k.ankle_rear, rig.fwd, rig.ground, t);
    draw_pelvis(blip, &hide, k.hip_rear, k.hip_lead);
    draw_torso(blip, &hide, k.hip, k.neck, t);
    // The far arm goes after the chest but before the head, or a raised guard
    // wipes out the face.
    draw_arm(blip, &far, k.sh_rear, k.elbow_rear, k.hand_rear, q.open, t);
    draw_head(blip, &hide, rig, k.head, k.neck, t);
    draw_leg(blip, &near, k.hip_lead, k.knee_lead, k.ankle_lead, rig.fwd, rig.ground, t);
    draw_skirt(blip, &hide, k.hip, k.neck, t);
    draw_arm(blip, &near, k.sh_lead, k.elbow_lead, k.hand_lead, q.open, t);

    // Sweat knocked loose, over everything: it is in the air in front
    // of the fighter, not on them.
    sweat_spray(blip, &f, k.head, shift);
}

fn draw_fight(blip: &Blip, g: &Game) {
    for i in 0..2 { draw_fighter(blip, g, i); }

    for b in g.bolts.iter() {
        if !b.active { continue; }
        let c = rgb(FIGHTERS[g.p[b.owner].who].trim);
        blip.fill_glow_circle(b.x, b.y, 11.0, rgba(FIGHTERS[g.p[b.owner].who].trim, 0.35));
        blip.fill_circle(b.x, b.y, 7.0, c);
        blip.fill_circle(b.x - b.vx.signum() * 5.0, b.y, 4.0, BLIP_WHITE);
    }

    for s in g.hitspark.iter() {
        if s.ttl <= 0.0 { continue; }
        let k = s.ttl * 6.0;
        // Gold for damage, cold blue for a guard: whether the last exchange
        // cost you is the whole of defence.
        let c = if s.blocked {
            BlipColor { r: 0.55, g: 0.85, b: 1.0, a: k.min(0.9) }
        } else {
            BlipColor { r: 1.0, g: 0.95, b: 0.6, a: k.min(0.9) }
        };
        // A burst of spikes rather than a ball of light. A circle is a
        // glow; spokes are a hit, because the eye reads the radiating
        // lines as force leaving a point.
        let spread = 8.0 + 26.0 * (1.0 - k).max(0.0);
        for j in 0..7 {
            let ang = j as f32 * 0.897 + s.x * 0.05;
            let (sx, sy) = (ang.cos(), ang.sin());
            let inner = 2.0 + 5.0 * k;
            let outer = inner + spread * (0.55 + 0.45 * ((j * 3) % 5) as f32 / 4.0);
            stroke(blip, V(s.x + sx * inner, s.y + sy * inner),
                V(s.x + sx * outer, s.y + sy * outer), 2.6 * k.min(1.0) + 0.6, 0.5, c);
        }
        blip.fill_circle(s.x, s.y, 3.0 + 9.0 * k, c);
        blip.fill_circle(s.x, s.y, 1.5 + 5.0 * k, BLIP_WHITE);
    }

    // The combo count, over the shoulder of whoever earned it.
    if g.combo_t > 0.0 && g.combo_shown > 1 {
        let f = g.p[g.combo_side];
        let text = format!("{} HITS", g.combo_shown);
        let rise = (1.1 - g.combo_t) * 26.0;
        blip.draw_text(&text, f.x - text_w(&text, 2.0) / 2.0, f.y - STAND_H - 26.0 - rise, 2.0,
            BlipColor { r: 1.0, g: 0.85, b: 0.25, a: g.combo_t.min(1.0) });
    }
}

// ---- HUD -----------------------------------------------------------------

/// Health, rounds won and the clock. Bars drain toward the centre, so who is
/// ahead reads without reading.
fn draw_hud(blip: &Blip, g: &Game) {
    let pad = 16.0;
    let bar_w = (WIN_W as f32 - pad * 3.0 - 56.0) / 2.0;
    let bar_h = 16.0;
    let y = 22.0;

    for i in 0..2 {
        let f = g.p[i];
        let frac = (f.health as f32 / f.arch().health as f32).clamp(0.0, 1.0);
        let x = if i == 0 { pad } else { WIN_W as f32 - pad - bar_w };
        blip.fill_rect(x - 2.0, y - 2.0, bar_w + 4.0, bar_h + 4.0,
            BlipColor { r: 0.10, g: 0.10, b: 0.12, a: 1.0 });
        blip.fill_rect(x, y, bar_w, bar_h, BlipColor { r: 0.35, g: 0.06, b: 0.06, a: 1.0 });
        let w = bar_w * frac;
        // Both bars empty away from the centre of the screen.
        let fx = if i == 0 { x + bar_w - w } else { x };
        let c = if frac > 0.45 { BlipColor { r: 0.95, g: 0.85, b: 0.2, a: 1.0 } }
                else if frac > 0.2 { BlipColor { r: 0.95, g: 0.55, b: 0.15, a: 1.0 } }
                else { BlipColor { r: 0.95, g: 0.25, b: 0.2, a: 1.0 } };
        blip.fill_rect(fx, y, w, bar_h, c);
        blip.draw_rect(x, y, bar_w, bar_h, BlipColor { r: 0.8, g: 0.8, b: 0.85, a: 0.7 });

        let name = f.arch().name;
        let nx = if i == 0 { pad } else { WIN_W as f32 - pad - text_w(name, 1.0) };
        blip.draw_text(name, nx, y + bar_h + 6.0, 1.0, BLIP_WHITE);

        // Round pips: a fighter needs two, so two lamps say everything.
        for r in 0..ROUNDS_TO_WIN {
            let px = if i == 0 { pad + r as f32 * 14.0 } else { WIN_W as f32 - pad - 10.0 - r as f32 * 14.0 };
            let lit = f.rounds > r;
            blip.fill_circle(px + 5.0, y + bar_h + 22.0, 5.0,
                if lit { BLIP_YELLOW } else { BlipColor { r: 0.25, g: 0.25, b: 0.28, a: 1.0 } });
        }
    }

    let secs = g.clock.max(0.0).ceil() as i32;
    let cx = WIN_W as f32 / 2.0;
    blip.fill_rect(cx - 26.0, y - 4.0, 52.0, bar_h + 12.0,
        BlipColor { r: 0.10, g: 0.10, b: 0.12, a: 1.0 });
    let tc = if secs <= 10 { BlipColor { r: 1.0, g: 0.35, b: 0.3, a: 1.0 } } else { BLIP_WHITE };
    let text = format!("{secs}");
    blip.draw_text(&text, cx - text_w(&text, 2.0) / 2.0, y + 2.0, 2.0, tc);

    let stage = if g.stage == 0 { "THE DOCKS" } else { "MOONLIT TEMPLE" };
    blip.fill_rect(cx - text_w(stage, 1.0) / 2.0 - 5.0, y + bar_h + 11.0,
        text_w(stage, 1.0) + 10.0, 12.0, BlipColor { r: 0.08, g: 0.08, b: 0.10, a: 0.75 });
    blip.draw_text(stage, cx - text_w(stage, 1.0) / 2.0, y + bar_h + 14.0, 1.0,
        BlipColor { r: 0.92, g: 0.92, b: 0.96, a: 1.0 });
}

fn draw_banner(blip: &Blip, g: &Game) {
    let cy = 150.0;
    match g.state {
        State::RoundIntro => {
            let r = format!("ROUND {}", g.round);
            blip.draw_centered(&r, cy, 4.0, BLIP_YELLOW);
            if g.phase.remaining() < 0.7 {
                blip.draw_centered("FIGHT!", cy + 46.0, 5.0, BLIP_WHITE);
            }
        }
        State::RoundEnd => blip.draw_centered(g.banner, cy, 5.0, BLIP_YELLOW),
        State::MatchEnd => blip.draw_centered(g.banner, cy, 4.0, BLIP_YELLOW),
        State::Over if g.mode == Mode::Versus => {
            // Two people played a match; one of them won it. "GAME
            // OVER" and a score belong to a run against the ladder and
            // say nothing at all about what just happened here.
            let (a, b) = (g.p[0].rounds, g.p[1].rounds);
            let (text, col) = if a > b {
                ("PLAYER 1 WINS", BlipColor { r: 1.0, g: 0.35, b: 0.35, a: 1.0 })
            } else if b > a {
                ("PLAYER 2 WINS", BlipColor { r: 0.40, g: 0.72, b: 1.0, a: 1.0 })
            } else {
                ("DRAW GAME", BLIP_YELLOW)
            };
            // On a plate, because the result sits over two fighters
            // still standing on the stage and yellow on a sunset is
            // not text, it is a smudge.
            blip.fill_rect(0.0, cy - 18.0, WIN_W as f32, 122.0,
                BlipColor { r: 0.02, g: 0.02, b: 0.05, a: 0.78 });
            blip.draw_centered(text, cy, 4.0, col);
            blip.draw_centered(&format!("{a} - {b}"), cy + 46.0, 3.0, BLIP_WHITE);
            if !g.phase.active() {
                blip.draw_centered("PRESS TO PLAY AGAIN", cy + 84.0, 2.0, BLIP_YELLOW);
            }
        }
        State::Over => {
            blip.draw_centered("GAME OVER", cy, 5.0, BlipColor { r: 0.95, g: 0.3, b: 0.3, a: 1.0 });
            let s = format!("SCORE {}", g.sess.score);
            blip.draw_centered(&s, cy + 50.0, 2.0, BLIP_WHITE);
            if !g.phase.active() { blip.draw_centered("PRESS FIRE", cy + 86.0, 2.0, BLIP_YELLOW); }
        }
        State::Won => {
            blip.draw_centered("CHAMPION", cy, 5.0, BLIP_YELLOW);
            let s = format!("SCORE {}", g.sess.score);
            blip.draw_centered(&s, cy + 50.0, 2.0, BLIP_WHITE);
            if !g.phase.active() { blip.draw_centered("PRESS FIRE", cy + 86.0, 2.0, BLIP_YELLOW); }
        }
        _ => {}
    }
}

// ---- front of house ------------------------------------------------------

/// Text as a lit tube. blip's `draw_text_glow` is a one-pixel halo, lost
/// under a big logo: this blooms outward in colour over several passes, lays
/// fattened glass, and puts a near-white core inside.
fn neon(blip: &Blip, text: &str, y: f32, sz: f32, c: BlipColor, lit: f32) {
    let x = blip.text_cx(text, sz as i32) as f32;
    const RING: [(f32, f32); 8] = [(-1.0, 0.0), (1.0, 0.0), (0.0, -1.0), (0.0, 1.0),
                                   (-0.7, -0.7), (0.7, -0.7), (-0.7, 0.7), (0.7, 0.7)];
    for (r, a) in [(1.05f32, 0.09f32), (0.7, 0.13), (0.42, 0.20)] {
        let col = BlipColor { a: a * lit, ..c };
        for (dx, dy) in RING {
            blip.draw_text(text, x + dx * r * sz, y + dy * r * sz, sz, col);
        }
    }
    let glass = BlipColor { a: lit, ..c };
    for (dx, dy) in RING {
        blip.draw_text(text, x + dx * 0.3 * sz, y + dy * 0.3 * sz, sz, glass);
    }
    blip.draw_text(text, x, y, sz, BlipColor { a: lit, ..blend(c, BLIP_WHITE, 0.78) });
}

/// A tube's flicker: mains hum, and the odd dropout of one that is on
/// its way out. Both are what makes a sign read as lit rather than as
/// coloured text.
fn tube(t: f32, phase: f32) -> f32 {
    let hum = 0.93 + 0.07 * ((t * 9.3 + phase).sin() * 0.6 + (t * 23.0).sin() * 0.4);
    let stutter = ((t * 0.83 + phase).sin() * (t * 7.7).sin()).abs();
    if stutter > 0.97 { hum * 0.42 } else { hum }
}

/// The title screen says only what is needed to start: what moves you, what
/// attacks, and that the kicks are heights. The rest is on the select screen
/// and the round-start reminder.
fn draw_title(blip: &Blip, g: &Game) {
    let now = g.now;
    // A night stage behind it, the two fighters squaring off on it, and
    // the whole thing knocked back so the sign is the brightest object
    // on screen — which is the only way a neon sign ever reads.
    draw_temple(blip, 0.0, now, 0.0);
    draw_floor(blip, 1, 0.0);
    // Out at the edges, leaving the middle band clear for the control
    // rows, which are the reason anybody looks at this screen twice.
    for (i, (x, face)) in [(62.0f32, 1.0f32), (578.0, -1.0)].iter().enumerate() {
        let mut f = Fighter::new(if i == 0 { 0 } else { 1 }, *x, *face);
        f.act = Act::Idle;
        pose_and_draw_lit(blip, &f, now, 0.0, 0.0, i, 1.0);
    }
    blip.fill_rect(0.0, 0.0, WIN_W as f32, WIN_H as f32,
        BlipColor { r: 0.02, g: 0.01, b: 0.08, a: 0.72 });

    let sign = BlipColor { r: 1.0, g: 0.16, b: 0.38, a: 1.0 };
    let glow = tube(now, 0.0);
    for (r, a) in [(150.0f32, 0.045f32), (96.0, 0.05)] {
        blip.fill_glow_circle(320.0, 72.0, r, BlipColor { a: a * glow, ..sign });
    }
    neon(blip, "BRAWLER", 30.0, 9.0, sign, glow);
    neon(blip, "TWO FIGHTERS ENTER", 112.0, 2.0,
        BlipColor { r: 0.35, g: 0.95, b: 1.0, a: 1.0 }, tube(now, 2.1));

    // The mode, chosen on a stick and one button because that is all a
    // cabinet has. The line you are on is lit; the other is glass.
    let dark = BlipColor { r: 0.42, g: 0.30, b: 0.46, a: 1.0 };
    for (i, label) in ["1 PLAYER", "2 PLAYERS"].iter().enumerate() {
        let y = 148.0 + i as f32 * 38.0;
        if g.menu == i {
            neon(blip, label, y, 3.0, BlipColor { r: 1.0, g: 0.85, b: 0.25, a: 1.0 },
                0.62 + 0.38 * (now * 3.4).sin().max(0.0));
        } else {
            blip.draw_centered(label, y, 3.0, dark);
        }
    }

    // Both players' controls, padded to the same width so the columns line
    // up.
    let dim = BlipColor { r: 0.74, g: 0.78, b: 0.86, a: 1.0 };
    let p1c = BlipColor { r: 1.0, g: 0.35, b: 0.35, a: 1.0 };
    let p2c = BlipColor { r: 0.40, g: 0.72, b: 1.0, a: 1.0 };
    // Two buttons each: punch, then kick. The stick picks the height.
    const ROWS: [(&str, &str, bool); 2] = [
        ("P1", "W A S D   F PUNCH  G KICK", true),
        ("P2", "ARROWS    J PUNCH  K KICK", false),
    ];
    let x = (WIN_W as f32 - 32.0 * 12.0) / 2.0;
    for (i, (tag, body, one)) in ROWS.iter().enumerate() {
        let y = 236.0 + i as f32 * 24.0;
        // Player two greys out on the one-player line: the row is still
        // there to say the mode exists, without claiming those keys do
        // anything in the game you are about to start.
        let live = *one || g.menu == 1;
        let tint = if live { 1.0 } else { 0.45 };
        let col = if *one { p1c } else { p2c };
        blip.draw_text(tag, x, y, 2.0, BlipColor { a: tint, ..col });
        blip.draw_text(body, x + 48.0, y, 2.0, BlipColor { a: tint, ..dim });
    }

    blip.draw_centered("TOWARD + BUTTON HITS HIGH   BOTH TOGETHER SPECIAL", 292.0, 1.0, dim);
    blip.draw_centered("W S CHOOSE     F OR SPACE START", 314.0, 2.0,
        BlipColor { r: 0.95, g: 0.88, b: 0.60, a: 1.0 });
}

/// The select screen: each player's cursor over the roster, the fighters
/// standing in their boxes.
fn draw_select(blip: &Blip, g: &Game) {
    let versus = g.mode == Mode::Versus;
    blip.draw_centered(if versus { "CHOOSE YOUR FIGHTERS" } else { "CHOOSE YOUR FIGHTER" },
        30.0, 3.0, BLIP_YELLOW);
    let p1c = BlipColor { r: 1.0, g: 0.35, b: 0.35, a: 1.0 };
    let p2c = BlipColor { r: 0.40, g: 0.72, b: 1.0, a: 1.0 };
    for (i, a) in FIGHTERS.iter().enumerate() {
        let x = 70.0 + i as f32 * 180.0;
        // Who is on this fighter, and have they committed to it.
        let on1 = g.pick == i;
        let on2 = versus && g.pick2 == i;
        let picked = on1 || on2;
        let box_c = if on1 && on2 { BLIP_WHITE }
            else if on1 && versus { p1c }
            else if on2 { p2c }
            else if picked { rgb(a.trim) }
            else { BlipColor { r: 0.3, g: 0.3, b: 0.35, a: 1.0 } };
        blip.draw_rect(x, 84.0, 150.0, 190.0, box_c);
        if picked { blip.draw_rect(x - 2.0, 82.0, 154.0, 194.0, box_c); }

        // The fighter drawn by the same code as in a match, so the select
        // screen shows what you will control.
        let cx = x + 75.0;
        let ground = 256.0;
        let mut who = Fighter::new(i, cx, 1.0);
        who.y = FLOOR_Y;
        // Everyone in their fighting stance: the winner's pose reads as a
        // guard dropped.
        who.act = Act::Idle;
        who.facing = 1.0;
        pose_and_draw(blip, &who, g.now, ground - FLOOR_Y, 0.0, i);

        blip.draw_text(a.name, cx - text_w(a.name, 2.0) / 2.0, 90.0, 2.0,
            if picked { BLIP_WHITE } else { BlipColor { r: 0.6, g: 0.6, b: 0.66, a: 1.0 } });

        // Whose cursor is here, and whether it is still moving. A
        // locked choice says so, because in versus the other player is
        // waiting on it and needs to know they are the hold-up.
        if versus {
            for (who_i, on, col, tx) in [(0usize, on1, p1c, x + 6.0),
                                         (1, on2, p2c, x + 150.0 - 6.0 - text_w("P2", 2.0))] {
                if !on { continue; }
                let lit = if g.locked[who_i] { 1.0 }
                    else { 0.45 + 0.55 * (g.now * 5.0).sin().max(0.0) };
                let c = BlipColor { a: lit, ..col };
                blip.draw_text(if who_i == 0 { "P1" } else { "P2" }, tx, 64.0, 2.0, c);
                if g.locked[who_i] {
                    blip.draw_text("READY", tx.min(x + 150.0 - 6.0 - text_w("READY", 1.0)),
                        250.0, 1.0, c);
                }
            }
        }

        // The three numbers that actually differ, as bars — a player
        // choosing between archetypes needs the trade, not a biography.
        let stats = [("PWR", a.power / 1.4), ("SPD", a.walk / 150.0),
                     ("HP ", a.health as f32 / 120.0)];
        for (r, (label, v)) in stats.iter().enumerate() {
            let sy = 266.0 + r as f32 * 13.0;
            blip.draw_text(label, x + 8.0, sy, 1.0, BlipColor { r: 0.7, g: 0.7, b: 0.8, a: 1.0 });
            blip.fill_rect(x + 40.0, sy, 100.0 * v.clamp(0.0, 1.0), 7.0, rgb(a.trim));
            blip.draw_rect(x + 40.0, sy, 100.0, 7.0,
                BlipColor { r: 0.3, g: 0.3, b: 0.36, a: 1.0 });
        }
    }

    if versus {
        // The movement keys, and a reminder of the two rules a player cannot
        // find by pressing buttons; the button panel is on the title screen.
        blip.draw_text("P1", 40.0, 322.0, 2.0, p1c);
        blip.draw_text("W A S D", 72.0, 322.0, 2.0,
            BlipColor { r: 0.80, g: 0.74, b: 0.78, a: 1.0 });
        blip.draw_text("P2", 400.0, 322.0, 2.0, p2c);
        blip.draw_text("ARROWS", 432.0, 322.0, 2.0,
            BlipColor { r: 0.76, g: 0.80, b: 0.88, a: 1.0 });
        blip.draw_centered("PUNCH AND KICK TOGETHER IS THE SPECIAL", 348.0, 1.0,
            BlipColor { r: 0.66, g: 0.66, b: 0.74, a: 1.0 });
        blip.draw_centered("NO TWO THE SAME", 364.0, 1.0,
            BlipColor { r: 0.55, g: 0.55, b: 0.62, a: 1.0 });
        let waiting = !g.locked[0] || !g.locked[1];
        blip.draw_centered(if waiting { "PRESS TO LOCK IN" } else { "FIGHT" },
            382.0, 2.0, BLIP_WHITE);
    } else {
        let a = FIGHTERS[g.pick];
        let s = format!("SPECIAL: {}    PUNCH AND KICK", a.special_name);
        blip.draw_centered(&s, 326.0, 2.0, rgb(a.trim));
        // The two rules a player cannot find by pressing buttons, put
        // where there is nothing else to do but read them.
        blip.draw_centered("GUARD HIGH OR LOW TO MATCH THE ATTACK", 350.0, 1.0,
            BlipColor { r: 0.74, g: 0.77, b: 0.84, a: 1.0 });
        blip.draw_centered("WALK IN CLOSE AND PUNCH TO THROW", 366.0, 1.0,
            BlipColor { r: 0.74, g: 0.77, b: 0.84, a: 1.0 });
        blip.draw_centered("PRESS FIRE TO START", 384.0, 2.0, BLIP_WHITE);
    }
}

// ---- pose gallery (development only) -------------------------------------
// `cargo build -p brawler --features gallery` replaces the game with a
// looping contact sheet of every pose, since reaching a sweep in play takes a
// dozen inputs and shows it for four frames.
#[cfg(feature = "gallery")]
pub fn draw_gallery(blip: &Blip, now: f32) {
    blip.clear(BlipColor { r: 0.15, g: 0.16, b: 0.21, a: 1.0 });
    let acts: [(&str, Act, MoveId); 20] = [
        ("LOW PUNCH", Act::Attack, MoveId::LowPunch),
        ("HIGH PUNCH", Act::Attack, MoveId::HighPunch),
        ("LOW KICK", Act::Attack, MoveId::LowKick),
        ("HIGH KICK", Act::Attack, MoveId::HighKick),
        ("SWEEP", Act::Attack, MoveId::Sweep),
        ("THROW", Act::Attack, MoveId::Throw),
        ("SPECIAL", Act::Attack, MoveId::Special),
        ("JUMP KICK", Act::Attack, MoveId::JumpKick),
        ("FLYING KICK", Act::Attack, MoveId::FlyingKick),
        ("JUMP PUNCH", Act::Attack, MoveId::JumpPunch),
        ("IDLE", Act::Idle, MoveId::LowPunch),
        ("WALK", Act::Walk, MoveId::LowPunch),
        ("CROUCH", Act::Crouch, MoveId::LowPunch),
        ("BLOCK", Act::Block, MoveId::LowPunch),
        ("HITSTUN", Act::Hitstun, MoveId::LowPunch),
        ("KNOCKDOWN", Act::Knockdown, MoveId::LowPunch),
        ("VICTORY", Act::Victory, MoveId::LowPunch),
        ("DEFEAT", Act::Defeat, MoveId::LowPunch),
        ("KO", Act::Defeat, MoveId::LowPunch),
        ("BOW", Act::Bow, MoveId::LowPunch),
    ];
    // One move at a time, five frames of it across the screen, stepped with
    // left / right so a capture lands on a known frame.
    use std::sync::atomic::{AtomicUsize, Ordering};
    static AT: AtomicUsize = AtomicUsize::new(usize::MAX);
    static WHO: AtomicUsize = AtomicUsize::new(0);
    // BLIP_POSE / BLIP_WHO aim a capture at one cell from the command
    // line; with BLIP_SCREENSHOT_OUT the sheet can be photographed.
    if AT.load(Ordering::Relaxed) == usize::MAX {
        let env = |k: &str| std::env::var(k).ok().and_then(|v| v.parse::<usize>().ok());
        AT.store(env("BLIP_POSE").unwrap_or(0), Ordering::Relaxed);
        WHO.store(env("BLIP_WHO").unwrap_or(0), Ordering::Relaxed);
    }
    if blip::input::key_pressed(BLIP_KEY_RIGHT) { AT.fetch_add(1, Ordering::Relaxed); }
    if blip::input::key_pressed(BLIP_KEY_LEFT) { AT.fetch_add(acts.len() - 1, Ordering::Relaxed); }
    if blip::input::key_pressed(BLIP_KEY_UP) { WHO.fetch_add(1, Ordering::Relaxed); }
    let n = AT.load(Ordering::Relaxed) % acts.len();
    let who = WHO.load(Ordering::Relaxed) % 3;
    let (label, act, mv) = acts[n];
    blip.fill_rect(0.0, FLOOR_Y, WIN_W as f32, WIN_H as f32 - FLOOR_Y,
        BlipColor { r: 0.11, g: 0.10, b: 0.13, a: 1.0 });

    let probe = Fighter::new(who, 0.0, 1.0);
    let m = probe.scaled(move_data(mv));
    let total = (m.startup + m.active + m.recovery) * F;
    for k in 0..5 {
        let x = 74.0 + k as f32 * 124.0;
        let mut f = Fighter::new(who, x, 1.0);
        f.act = act;
        f.mv = mv;
        f.y = FLOOR_Y;
        // Half a health bar down on the left and nearly out on the right, so
        // sweat and breathing show.
        f.health = (FIGHTERS[who].health as f32 * (0.62 - 0.13 * k as f32)) as i32;
        if label == "KO" { f.health = 0; }
        f.t = match act {
            // Sampled inside the startup, because the startup is where
            // the shape of a move is decided and the part a defender
            // has to read.
            Act::Attack => [m.startup * F * 0.35, m.startup * F * 0.7, m.startup * F,
                            (m.startup + m.active) * F, total * 0.8][k],
            Act::Knockdown => [0.05, 0.22, 0.6, 0.95, 1.12][k],
            Act::Hitstun => [0.02, 0.06, 0.12, 0.2, 0.3][k],
            // Everything else gets a time sweep too, so the sheet shows
            // whether a pose moves.
            Act::Victory | Act::Defeat => [0.0, 0.18, 0.45, 0.9, 1.6][k],
            Act::Bow => [0.1, 0.5, 0.85, 1.2, 1.55][k],
            _ => now + k as f32 * 0.09,
        };
        if matches!(mv, MoveId::JumpKick | MoveId::JumpPunch) && act == Act::Attack {
            f.y = FLOOR_Y - 46.0;
        }
        // The flying kick is drawn along its own arc, since the pose is
        // read off vertical speed and a still one says nothing.
        if mv == MoveId::FlyingKick && act == Act::Attack {
            let k = k as f32 / 4.0;
            f.y = FLOOR_Y - 44.0 * (1.0 - (2.0 * k - 1.0).powi(2));
            f.vy = FLY_VY * (1.0 - 2.0 * k);
            f.vx = f.facing * FLY_SPEED;
        }
        if act == Act::Walk { f.x = x + (now * 60.0) % 24.0; }
        blip.draw_line(x - 60.0, FLOOR_Y, x + 74.0, FLOOR_Y,
            BlipColor { r: 0.34, g: 0.34, b: 0.44, a: 0.8 });
        // The hitbox this frame, so the picture can be checked against
        // the thing it claims to be a picture of.
        if let Some((hx, hy, hw, hh)) = f.hit_box() {
            blip.fill_rect(hx, hy, hw, hh, BlipColor { r: 1.0, g: 0.3, b: 0.3, a: 0.30 });
        }
        pose_and_draw(blip, &f, now, 0.0, 0.0, k);
    }
    blip.draw_centered(label, 22.0, 3.0, BLIP_YELLOW);
    blip.draw_centered(FIGHTERS[who].name, 54.0, 2.0, BLIP_WHITE);
}
