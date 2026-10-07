//! The stages: seven backdrops and the floor under them. Scenery only: the
//! floor line, the walls and the height are the same everywhere.

use super::*;

/// Backdrop contrast reduction keeps fighters prominent; all stages share floor, height and walls.
pub(crate) const HAZE: f32 = 0.34;

/// The current stage: its backdrop, the knockout burst if there is one, and
/// the floor.
pub(crate) fn draw_stage(blip: &Blip, g: &Game) {
    // A hit shakes the whole picture, which is most of what selling an
    // impact means when the fighters themselves are simple shapes.
    let shake = g.shake_px();
    draw_backdrop(blip, g.stage, shake, g.now, g.shake);
    if g.slow > 0.0 { draw_finish(blip, g); }
    draw_floor(blip, g.stage, shake);
}

/// One stage's scenery and the air in front of it, moved down by `shift`.
pub(crate) fn draw_backdrop(blip: &Blip, stage: usize, shift: f32, now: f32, hit: f32) {
    let air = |r: f32, g: f32, b: f32, a: f32| BlipColor { r, g, b, a };
    let air = match stage {
        0 => { draw_dock(blip, shift, now, hit); air(0.40, 0.17, 0.20, HAZE) }
        1 => { draw_temple(blip, shift, now, hit); air(0.07, 0.07, 0.13, HAZE) }
        // The daylight stages are bright already: less air, or they wash out.
        2 => { draw_airbase(blip, shift, now, hit); air(0.70, 0.80, 0.90, HAZE * 0.6) }
        3 => { draw_bathhouse(blip, shift, now, hit); air(0.50, 0.38, 0.26, HAZE * 0.8) }
        5 => { draw_fortress(blip, shift, now); air(0.55, 0.75, 0.90, HAZE * 0.5) }
        6 => { draw_rooftop(blip, shift, now, hit); air(0.42, 0.26, 0.34, HAZE * 0.8) }
        _ => { draw_village(blip, shift, now, hit); air(0.50, 0.56, 0.36, HAZE * 0.7) }
    };
    blip.fill_rect(0.0, 0.0, WIN_W as f32, FLOOR_Y + shift, air);
    draw_life(blip, stage, shift, now);
}

/// Something small and alive in front of the air of three stages: gulls
/// over the docks, petals in the temple yard, fireflies by the river.
pub(crate) fn draw_life(blip: &Blip, stage: usize, shift: f32, now: f32) {
    let w = WIN_W as f32;
    match stage {
        0 => for i in 0..3 {
            // Each gull glides across on its own line and beats its wings.
            let x = (now * (14.0 + 5.0 * i as f32) + scatter(i, 0xe1) * 900.0).rem_euclid(w + 80.0) - 40.0;
            let y = 70.0 + scatter(i, 0xe2) * 60.0 + (now * 0.7 + i as f32).sin() * 6.0 + shift;
            let flap = (now * 5.0 + i as f32 * 2.0).sin() * 3.5;
            let bird = BlipColor { r: 0.16, g: 0.10, b: 0.14, a: 1.0 };
            stroke(blip, V(x - 7.0, y - flap), V(x, y), 0.9, 1.1, bird);
            stroke(blip, V(x + 7.0, y - flap), V(x, y), 0.9, 1.1, bird);
        },
        1 => for i in 0..16 {
            // Petals slide down and across, turning as they fall.
            let fall = 16.0 + scatter(i, 0xe4) * 14.0;
            let y = (scatter(i, 0xe5) * FLOOR_Y + now * fall).rem_euclid(FLOOR_Y);
            let x = (scatter(i, 0xe6) * w + now * 9.0 + (now * 1.3 + i as f32).sin() * 12.0).rem_euclid(w);
            let turn = (now * 3.0 + i as f32 * 1.7).sin();
            blip.fill_rect(x, y + shift, 2.0 + turn.abs() * 1.5, 2.0,
                BlipColor { r: 0.98, g: 0.72, b: 0.80, a: 1.0 });
        },
        4 => for i in 0..12 {
            // Fireflies wander and wink.
            let x = scatter(i, 0xe7) * w + (now * 0.6 + i as f32 * 2.1).sin() * 18.0;
            let y = 150.0 + scatter(i, 0xe8) * 150.0 + (now * 0.9 + i as f32).cos() * 10.0 + shift;
            if (now * 1.4 + scatter(i, 0xe9) * 6.0).sin() > 0.1 {
                blip.fill_rect(x, y, 2.0, 2.0, BlipColor { r: 0.92, g: 1.0, b: 0.50, a: 1.0 });
            }
        },
        _ => {}
    }
}

/// The finishing blow: for as long as the slow motion lasts the stage is
/// gone, and there is only the winner's colour bursting out from where the
/// loser was hit.
pub(crate) fn draw_finish(blip: &Blip, g: &Game) {
    let Some(lost) = g.p.iter().position(|f| f.health == 0) else { return };
    let (winner, loser) = (g.p[1 - lost].arch(), &g.p[lost]);
    let base = blend(rgb(winner.trim), rgb(winner.color), 0.25);
    blip.fill_rect(0.0, 0.0, WIN_W as f32, FLOOR_Y, shade(base, 0.22));
    let c = V(loser.x, loser.y - loser.height() * 0.6);
    // The rays jump round a notch at a time: a flicker, not a spin.
    let turn = (g.now * 14.0).floor() * 0.21;
    blip.fill_sunburst(c.0, c.1, 14, turn, 760.0, [shade(base, 0.62), shade(base, 0.40)]);
}

/// The ground, seen flat-on: lit boards falling off toward the bottom of the
/// screen, bright enough for the fighters' shadows to land on.
pub(crate) fn draw_floor(blip: &Blip, stage: usize, shake: f32) {
    // Boards on the docks, stone in the temple, tarmac, wet boards, a jetty,
    // ice, a tarred roof.
    const FLOORS: [(f32, f32, f32); STAGES] = [(0.38, 0.28, 0.21), (0.26, 0.25, 0.31),
        (0.46, 0.48, 0.50), (0.56, 0.42, 0.26), (0.42, 0.31, 0.19),
        (0.62, 0.76, 0.88), (0.30, 0.27, 0.30)];
    let near = rgb(FLOORS[stage % STAGES]);
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

/// Distance as colour: far things lose contrast toward the sky rather than
/// just darkening.
pub(crate) fn hazed(c: BlipColor, sky: BlipColor, k: f32) -> BlipColor { blend(c, sky, k) }

/// A row of onlookers: small, dense, overlapping silhouettes below the
/// fighters' heads, so they read as a crowd and not more fighters. They bob
/// out of step and stand up for a big hit.
pub(crate) fn draw_crowd(blip: &Blip, ground: f32, h: f32, c: BlipColor, t: f32, salt: u32, n: usize,
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

/// How often a cameo crosses the back of a stage, and how long the crossing
/// takes: rare enough to be a surprise, never in front of the fight.
pub(crate) const CAMEO_EVERY: f32 = 36.0;
pub(crate) const CAMEO_SECS: f32 = 9.0;

/// Where a cameo is on its way across, if one is crossing now.
pub(crate) fn cameo_x(t: f32, offset: f32) -> Option<f32> {
    let u = (t + offset).rem_euclid(CAMEO_EVERY) / CAMEO_SECS;
    (u < 1.0).then(|| -30.0 + u * (WIN_W as f32 + 80.0))
}

/// Along the far wharf: a yellow mouth on the run from a red ghost, the
/// oldest chase in the arcade. `gap` is the colour behind it, for the mouth.
pub(crate) fn draw_chase(blip: &Blip, y: f32, t: f32, gap: BlipColor) {
    let Some(x) = cameo_x(t, 12.0) else { return };
    blip.fill_circle(x, y, 4.5, BlipColor { r: 0.98, g: 0.86, b: 0.20, a: 1.0 });
    if (t * 9.0).sin() > 0.0 {
        for up in [-2.0f32, 0.0, 2.0] { blip.draw_line(x, y, x + 5.5, y + up, gap); }
    }
    let gx = x - 26.0;
    let red = BlipColor { r: 0.90, g: 0.20, b: 0.18, a: 1.0 };
    blip.fill_circle(gx, y - 0.5, 4.5, red);
    blip.fill_rect(gx - 4.5, y - 0.5, 9.0, 5.0, red);
    for dx in [-0.8f32, 2.6] {
        blip.fill_rect(gx + dx, y - 2.5, 2.0, 2.4, BLIP_WHITE);
    }
}

/// Across the night sky behind the pillars: a white fighter and the bee it
/// is chasing, out of a shooting gallery of 1981.
pub(crate) fn draw_flyby(blip: &Blip, y: f32, t: f32) {
    let Some(x) = cameo_x(t, 0.0) else { return };
    let white = BlipColor { r: 0.92, g: 0.94, b: 1.0, a: 1.0 };
    let red = BlipColor { r: 0.90, g: 0.20, b: 0.20, a: 1.0 };
    let blue = BlipColor { r: 0.30, g: 0.45, b: 0.95, a: 1.0 };
    // The fighter: nose, wings, red tips.
    blip.fill_rect(x - 1.0, y - 7.0, 2.0, 9.0, white);
    blip.fill_rect(x - 6.0, y - 1.0, 12.0, 3.0, white);
    blip.fill_rect(x - 6.0, y - 4.0, 2.0, 3.0, red);
    blip.fill_rect(x + 4.0, y - 4.0, 2.0, 3.0, red);
    blip.fill_rect(x - 1.0, y - 3.0, 2.0, 2.0, blue);
    // Its shot, and the bee weaving ahead of it.
    let shot = (t * 2.0).fract();
    blip.fill_rect(x + 18.0 + shot * 22.0, y - 3.0, 4.0, 1.5, white);
    let (bx, by) = (x + 58.0, y - 6.0 + (t * 3.0).sin() * 9.0);
    blip.fill_rect(bx - 2.0, by - 3.0, 4.0, 6.0, BlipColor { r: 0.98, g: 0.84, b: 0.20, a: 1.0 });
    blip.fill_rect(bx - 5.0, by - 2.0, 3.0, 4.0, blue);
    blip.fill_rect(bx + 2.0, by - 2.0, 3.0, 4.0, blue);
    blip.fill_rect(bx - 2.0, by - 4.0, 4.0, 1.5, red);
}

/// Docks layers run from sky to quay with distance haze; keep the fighter band low-detail.
pub(crate) fn draw_dock(blip: &Blip, shake: f32, t: f32, hit: f32) {
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
pub(crate) fn draw_temple(blip: &Blip, shake: f32, t: f32, hit: f32) {
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

/// A sky in flat bands from `top` down to `low` at the horizon, and `low`
/// carried on down to the floor so no layer leaves a hole.
pub(crate) fn draw_sky(blip: &Blip, shake: f32, horizon: f32, top: BlipColor, low: BlipColor) {
    let bands = 14;
    let h = horizon / bands as f32;
    for i in 0..bands {
        let k = i as f32 / (bands - 1) as f32;
        blip.fill_rect(0.0, i as f32 * h + shake, WIN_W as f32, h + 1.0, blend(top, low, k));
    }
    blip.fill_rect(0.0, horizon + shake, WIN_W as f32, FLOOR_Y - horizon, low);
}

/// THE AIR BASE: midday on the apron, a jet parked behind the ground crew.
/// The brightest stage; the tarmac behind the fighters is kept bare.
pub(crate) fn draw_airbase(blip: &Blip, shake: f32, t: f32, hit: f32) {
    let hit = (hit / 0.16).clamp(0.0, 1.0);
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
        t, 0xa7c, 30, hit);
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

/// QUEENS ROOFTOP: dusk over the river, the city lit up across it, the
/// elevated train going by, and the neighbours watching from the next roof.
/// The brick parapet behind the fighters is the quiet band.
pub(crate) fn draw_rooftop(blip: &Blip, shake: f32, t: f32, hit: f32) {
    let hit = (hit / 0.16).clamp(0.0, 1.0);
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
    draw_crowd(blip, roof + 2.0, 26.0, hazed(c(0.20, 0.14, 0.22), low, 0.10), t, 0xb9d, 26, hit);

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

/// THE BATH HOUSE: indoors, steam off the tub, and on the wall the painted
/// mountain every bath house has. The tub's wooden side is the quiet band.
pub(crate) fn draw_bathhouse(blip: &Blip, shake: f32, t: f32, hit: f32) {
    let hit = (hit / 0.16).clamp(0.0, 1.0);
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
    draw_crowd(blip, HORIZON + 8.0 + shake, 26.0, c(0.62, 0.44, 0.34), t, 0xb47, 22, hit);
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

/// THE RIVER VILLAGE: a jetty on a slow green river, a hut on stilts, the
/// jungle behind and something long and green in the big tree.
pub(crate) fn draw_village(blip: &Blip, shake: f32, t: f32, hit: f32) {
    let hit = (hit / 0.16).clamp(0.0, 1.0);
    const HORIZON: f32 = 250.0;
    let c = |r: f32, g: f32, b: f32| BlipColor { r, g, b, a: 1.0 };
    let low = c(0.94, 0.93, 0.70);
    draw_sky(blip, shake, HORIZON, c(0.50, 0.78, 0.76), low);

    // Jungle in two ranges of rounded crowns, and palms above them.
    for (base, r0, salt, k, n) in [(HORIZON - 26.0, 22.0f32, 0xc1u32, 0.50f32, 22usize),
                                   (HORIZON - 6.0, 28.0, 0xc2, 0.22, 16)] {
        let leaf = hazed(c(0.12, 0.32, 0.18), low, k);
        for i in 0..n {
            let x = (i as f32 + scatter(i, salt)) * WIN_W as f32 / (n - 1) as f32;
            blip.fill_circle(x, base + shake, r0 + scatter(i, salt ^ 9) * 16.0, leaf);
        }
        blip.fill_rect(0.0, base + shake, WIN_W as f32, HORIZON - base + 8.0, leaf);
    }
    let palm = hazed(c(0.10, 0.26, 0.16), low, 0.30);
    for (x, top) in [(232.0f32, 120.0f32), (402.0, 96.0), (470.0, 138.0)] {
        let sway = (t * 0.8 + x).sin() * 3.0;
        blip.draw_line_ex(x, HORIZON - 20.0 + shake, x + sway, top + shake, 4.0, palm);
        for a in [-2.6f32, -2.0, -1.2, -0.5, 0.1] {
            stroke(blip, V(x + sway, top + shake), V(x + sway + a.cos() * 34.0, top - 6.0 + a.sin().abs() * 20.0 + shake),
                3.0, 1.0, palm);
        }
    }

    // The far bank and the villagers on it; the river, slow and green.
    draw_crowd(blip, HORIZON + 8.0 + shake, 28.0, hazed(c(0.24, 0.18, 0.12), low, 0.12), t, 0xc5d, 28, hit);
    blip.fill_rect(0.0, HORIZON + 4.0 + shake, WIN_W as f32, 10.0, c(0.46, 0.36, 0.22));
    let river = c(0.30, 0.42, 0.32);
    blip.fill_rect(0.0, HORIZON + 14.0 + shake, WIN_W as f32, FLOOR_Y - HORIZON - 14.0, river);
    for i in 0..20 {
        let sx = scatter(i, 0xc7);
        let y = HORIZON + 20.0 + sx * 50.0 + shake;
        let x = ((i as f32 * 67.0) + t * (5.0 + sx * 7.0)) % (WIN_W as f32 + 60.0) - 30.0;
        blip.fill_rect(x, y, 16.0 + sx * 22.0, 1.5, BlipColor { r: 0.80, g: 0.90, b: 0.70, a: 0.20 });
    }

    // A hut on stilts, upstream.
    let (hx, hy) = (36.0, HORIZON - 30.0 + shake);
    for sx in [8.0f32, 44.0, 84.0, 118.0] { blip.draw_line_ex(hx + sx, hy + 44.0, hx + sx, HORIZON + 30.0 + shake, 3.0, c(0.30, 0.20, 0.12)); }
    blip.fill_rect(hx, hy, 126.0, 46.0, c(0.52, 0.36, 0.20));
    blip.fill_rect(hx + 50.0, hy + 14.0, 24.0, 32.0, c(0.16, 0.10, 0.08));
    for r in 0..6 {
        let f = r as f32 / 6.0;
        blip.fill_rect(hx - 14.0 + f * 60.0, hy - 8.0 - r as f32 * 8.0, 154.0 - f * 120.0, 9.0,
            shade(c(0.72, 0.58, 0.30), 1.0 - 0.06 * (r % 2) as f32));
    }

    // The big tree at the water's edge, its vines, and the snake asleep on a
    // branch: only its tail moves.
    let bark = c(0.28, 0.19, 0.12);
    blip.fill_rect(566.0, shake, 50.0, FLOOR_Y - 30.0, bark);
    blip.fill_rect(566.0, shake, 10.0, FLOOR_Y - 30.0, blend(bark, BLIP_WHITE, 0.12));
    stroke(blip, V(580.0, 108.0 + shake), V(470.0, 84.0 + shake), 10.0, 5.0, bark);
    for (x, y, r) in [(560.0f32, 26.0f32, 46.0f32), (610.0, 50.0, 40.0), (500.0, 40.0, 34.0), (640.0, 10.0, 40.0)] {
        blip.fill_circle(x, y + shake, r, c(0.14, 0.36, 0.20));
    }
    for (x, len) in [(520.0f32, 70.0f32), (548.0, 104.0), (626.0, 90.0)] {
        blip.draw_line(x, 60.0 + shake, x + (t * 0.9 + x).sin() * 3.0, 60.0 + len + shake, c(0.20, 0.44, 0.24));
    }
    let scale = c(0.36, 0.62, 0.22);
    let mut at = V(540.0, 92.0 + shake);
    for i in 0..9 {
        let next = V(at.0 - 9.0, 86.0 + shake + if i % 2 == 0 { 9.0 } else { -3.0 });
        stroke(blip, at, next, 4.0, 4.0, scale);
        at = next;
    }
    stroke(blip, at, V(at.0 - 4.0, at.1 + 18.0 + (t * 1.6).sin() * 4.0), 3.0, 1.0, scale);
    blip.fill_circle(544.0, 90.0 + shake, 5.5, scale);
    blip.fill_circle(546.0, 88.0 + shake, 1.3, c(0.96, 0.90, 0.30));

    // The jetty's edge: posts and a rope.
    let post = c(0.40, 0.28, 0.16);
    blip.fill_rect(0.0, FLOOR_Y - 30.0 + shake, WIN_W as f32, 30.0, post);
    blip.fill_rect(0.0, FLOOR_Y - 32.0 + shake, WIN_W as f32, 3.0, blend(post, BLIP_WHITE, 0.22));
    for i in 0..22 { blip.draw_line(i as f32 * 30.0, FLOOR_Y - 29.0 + shake, i as f32 * 30.0, FLOOR_Y + shake, shade(post, 0.72)); }
    blip.fill_rect(0.0, FLOOR_Y - 16.0 + shake, WIN_W as f32, 2.0, c(0.74, 0.66, 0.46));
}
