//! Drawing. Fighters are posed from the same numbers the rules use: the reach
//! and height a punch is judged by are the reach and height it is drawn at.

use super::*;

fn rgb(c: (f32, f32, f32)) -> BlipColor { BlipColor { r: c.0, g: c.1, b: c.2, a: 1.0 } }

/// Width of a string in pixels at size `sz` (a fixed 6px cell). Not blip's
/// `text_cx()`, which returns the x that centres a string on the canvas.
fn text_w(text: &str, sz: f32) -> f32 { text.len() as f32 * 6.0 * sz }
fn rgba(c: (f32, f32, f32), a: f32) -> BlipColor { BlipColor { r: c.0, g: c.1, b: c.2, a } }

pub fn draw(blip: &Blip, g: &Game) {
    // BLIP_SELECT=n shows the select screen, cursor on fighter n, instead of
    // the pose sheet.
    #[cfg(feature = "gallery")]
    {
        match std::env::var("BLIP_SELECT").ok().and_then(|v| v.parse::<usize>().ok()) {
            Some(pick) => draw_select(blip, &Game { pick: pick % FIGHTERS.len(), now: g.now, ..Game::new() }),
            None => draw_gallery(blip, g.now),
        }
        return;
    }
    #[allow(unreachable_code)]
    if g.poster { return draw_poster(blip, g); }
    match g.state {
        State::Title => draw_title(blip, g),
        State::Select => draw_select(blip, g),
        State::Vs => draw_vs(blip, g),
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
    draw_backdrop(blip, g.stage, shake, g.now, g.shake);
    if g.slow > 0.0 { draw_finish(blip, g); }
    draw_floor(blip, g.stage, shake);
}

/// One stage's scenery and the air in front of it, moved down by `shift`.
fn draw_backdrop(blip: &Blip, stage: usize, shift: f32, now: f32, hit: f32) {
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
}

/// The finishing blow: for as long as the slow motion lasts the stage is
/// gone, and there is only the winner's colour bursting out from where the
/// loser was hit.
fn draw_finish(blip: &Blip, g: &Game) {
    let Some(lost) = g.p.iter().position(|f| f.health == 0) else { return };
    let (winner, loser) = (g.p[1 - lost].arch(), &g.p[lost]);
    let base = blend(rgb(winner.trim), rgb(winner.color), 0.25);
    blip.fill_rect(0.0, 0.0, WIN_W as f32, FLOOR_Y, shade(base, 0.22));
    let c = V(loser.x, loser.y - loser.height() * 0.6);
    // The rays jump round a notch at a time: a flicker, not a spin.
    let turn = (g.now * 14.0).floor() * 0.21;
    let rays = 14;
    for j in 0..rays {
        let ang = turn + j as f32 * std::f32::consts::TAU / rays as f32;
        let (dx, dy) = (ang.cos(), ang.sin());
        let tone = if j % 2 == 0 { 0.62 } else { 0.40 };
        // Circles spaced by their own size: a wedge in forty, not four hundred.
        let mut d = 26.0;
        while d < 760.0 {
            let r = d * 0.105;
            blip.fill_circle(c.0 + dx * d, c.1 + dy * d, r, shade(base, tone));
            d += r * 0.8;
        }
    }
}

/// The ground, seen flat-on: lit boards falling off toward the bottom of the
/// screen, bright enough for the fighters' shadows to land on.
fn draw_floor(blip: &Blip, stage: usize, shake: f32) {
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

/// How often a cameo crosses the back of a stage, and how long the crossing
/// takes: rare enough to be a surprise, never in front of the fight.
const CAMEO_EVERY: f32 = 36.0;
const CAMEO_SECS: f32 = 9.0;

fn cameo_x(t: f32, offset: f32) -> Option<f32> {
    let u = (t + offset).rem_euclid(CAMEO_EVERY) / CAMEO_SECS;
    (u < 1.0).then(|| -30.0 + u * (WIN_W as f32 + 80.0))
}

/// Along the far wharf: a yellow mouth on the run from a red ghost, the
/// oldest chase in the arcade. `gap` is the colour behind it, for the mouth.
fn draw_chase(blip: &Blip, y: f32, t: f32, gap: BlipColor) {
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
fn draw_flyby(blip: &Blip, y: f32, t: f32) {
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
fn draw_sky(blip: &Blip, shake: f32, horizon: f32, top: BlipColor, low: BlipColor) {
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
fn draw_airbase(blip: &Blip, shake: f32, t: f32, hit: f32) {
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
fn draw_fortress(blip: &Blip, shake: f32, t: f32) {
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
fn draw_rooftop(blip: &Blip, shake: f32, t: f32, hit: f32) {
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
fn draw_bathhouse(blip: &Blip, shake: f32, t: f32, hit: f32) {
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
fn draw_village(blip: &Blip, shake: f32, t: f32, hit: f32) {
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

// ---- anatomy -------------------------------------------------------------
// Fighters are drawn like the HD redraws of Street Fighter II: a bold line
// round each shape, flat colour inside, one darker tone for the far limbs.
// 1. **Pose space.** Joints are (forward, up) from the floor under the feet,
// so one pose serves both sides.
// 2. **Two bones per limb.** Hands and feet are placed; knees and elbows are
// solved.
// 3. **One outline per shape.** A limb is inked whole and then filled, so a
// knee is a bend in a leg and not a hinge between two parts.

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
    /// The fighter's size: pose space is a full-size fighter's.
    scale: f32,
}

impl Rig {
    fn at(self, q: P) -> V {
        V(self.cx + self.fwd * q.f * self.scale, self.ground - q.u * self.scale)
    }
    pub(crate) fn upright(cx: f32, ground: f32, fwd: f32, face: f32, scale: f32) -> Rig {
        Rig { cx, ground, fwd, face, scale }
    }
}

/// Bone lengths in pixels. The legs add up to more than the hip height, so a
/// fighter stands with bent knees.
const THIGH: f32 = 30.0;
const SHIN: f32 = 29.0;
const UARM: f32 = 24.0;
const FARM: f32 = 21.0;

/// Joint heights standing and crouching: hips and the centre of the head (the
/// spine between is one bone, see `neck_of`). Hips low: at 58 the solver
/// clamped both legs straight (see `a_waiting_fighter_has_their_knees_bent`).
const HIP_U: f32 = 53.0;
const HEAD_U: f32 = 105.0;
const C_HIP: f32 = 31.0;
const C_HEAD: f32 = 61.0;

/// Five heads tall, not life's seven and a half: a cartoon's proportions,
/// and a skull big enough to carry a face.
pub(crate) fn head_r(bulk: f32) -> f32 { 12.0 + 1.5 * (bulk - 1.0) }

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

/// The line around everything, and its width. A fighter without one dissolves
/// into whichever stage happens to be a similar colour.
const INK: BlipColor = BlipColor { r: 0.07, g: 0.06, b: 0.09, a: 1.0 };
const LINE: f32 = 2.0;

/// How much darker the far arm and leg are: the one shadow tone.
const FAR: f32 = 0.80;

/// What the face is doing. An expression says who just got hit faster than
/// a health bar does.
#[derive(Copy, Clone, PartialEq, Eq)]
enum Mood { Calm, Shout, Hurt, Happy }

/// How a fighter is coloured and clothed.
#[derive(Copy, Clone)]
struct Look {
    cloth: BlipColor,
    trim: BlipColor,
    skin: BlipColor,
    hair: BlipColor,
    build: Build,
    /// Every radius on the body is multiplied by this: the fighter's bulk
    /// times their size.
    bulk: f32,
    size: f32,
    far: bool,
    /// How dark this part is drawn: 1.0 in the light, less for what is
    /// further from it.
    tone: f32,
    /// Which way loose ends trail, in head radii: against the way the fighter
    /// is moving.
    wind: (f32, f32),
    /// Blown toward white for the frames a hit is frozen on.
    flash: f32,
}

impl Look {
    fn c(&self, base: BlipColor) -> BlipColor {
        blend(shade(base, self.tone), BLIP_WHITE, self.flash)
    }
    fn ink(&self) -> BlipColor { blend(INK, BLIP_WHITE, self.flash * 0.6) }
}

/// A tapered capsule — the only primitive a body is made of.
type Cap = (V, V, f32, f32);

fn stroke(blip: &Blip, a: V, b: V, r1: f32, r2: f32, c: BlipColor) {
    let n = (dist(a, b) / 1.7).ceil().max(1.0);
    for i in 0..=n as i32 {
        let t = i as f32 / n;
        blip.fill_circle(a.0 + (b.0 - a.0) * t, a.1 + (b.1 - a.1) * t, r1 + (r2 - r1) * t, c);
    }
}

/// One shape out of several capsules: all the ink, then all the colour, so
/// the outline runs round the whole and not round each piece.
fn shape(blip: &Blip, caps: &[Cap], fill: BlipColor, ink: BlipColor) {
    for &(a, b, r1, r2) in caps { stroke(blip, a, b, r1 + LINE, r2 + LINE, ink); }
    for &(a, b, r1, r2) in caps { stroke(blip, a, b, r1, r2, fill); }
}

/// How far a two-bone limb can fold: a knee to about thirty degrees, an elbow
/// to about thirty-five.
const KNEE_SHUT: f32 = 0.52;
const ELBOW_SHUT: f32 = 0.61;
/// The nearest a hand is drawn to its own shoulder.
const ARM_MIN: f32 = 18.0;

/// The two-bone solve: where the joint goes and where the limb's end actually
/// lands. Bones never stretch; the target is clamped into the reachable ring
/// and the settled point returned. `toward` picks the way the joint breaks
/// (knees forward, elbows back).
fn solve(root: V, want: V, l1: f32, l2: f32, shut: f32, toward: V) -> (V, V) {
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
    // always picks.
    let score = |c: V| (c.0 - root.0) * toward.0 + (c.1 - root.1) * toward.1;
    (if score(one) >= score(two) { one } else { two }, end)
}

// ---- parts ---------------------------------------------------------------
// Big fists, big feet, big head: the ends of a fighter are what hit and what
// get hit, so they are the clearest shapes on it.

/// Height of the ankle over the sole, and how far the toes reach past it.
/// Kicks aim the ankle short by `FOOT`, or the toes overshoot the hitbox.
const ANKLE: f32 = 7.8;
pub(crate) const FOOT: f32 = 16.0;

/// The same for a punch: how far the front of the fist is past the wrist.
pub(crate) const FIST: f32 = 7.0;

/// Which way the toes point, and how much of a kick this is (0 on or near the
/// floor, 1 well off it): a standing foot lies flat, a kicking one points
/// along the shin.
pub(crate) fn foot_dir(knee: V, ankle: V, fwd: f32, ground: f32, size: f32) -> (f32, f32, f32) {
    let (sx, sy) = unit(knee, ankle);
    let k = ((ground - ankle.1 - 22.0 * size) / (12.0 * size)).clamp(0.0, 1.0);
    let k = k * k * (3.0 - 2.0 * k);
    let (x, y) = (fwd + (sx - fwd) * k, sy * k);
    let m = x.hypot(y).max(1e-3);
    (x / m, y / m, k)
}

/// A point `at` turned about `pivot` by `ang`, clockwise on screen for a
/// fighter facing right (`fwd` mirrors it).
fn turn(at: V, pivot: V, ang: f32, fwd: f32) -> V {
    let (s, c) = ((ang * fwd).sin(), (ang * fwd).cos());
    let (x, y) = (at.0 - pivot.0, at.1 - pivot.1);
    V(pivot.0 + x * c - y * s, pivot.1 + x * s + y * c)
}

/// A leg and its foot. `heel` is how far the heel is raised, in radians: on
/// the floor the foot bends at the ball and the toes stay down, the way a
/// fighter stands on the back foot; in the air the whole foot points.
fn draw_leg(blip: &Blip, l: &Look, hip: V, knee: V, ankle: V, fwd: f32, ground: f32, heel: f32) {
    let b = l.bulk;
    let ink = l.ink();
    let (skin, cloth, trim) = (l.c(l.skin), l.c(l.cloth), l.c(l.trim));
    let booted = matches!(l.build, Build::Bare | Build::Suit | Build::Spider | Build::Caped);

    // The pose places the sole; a kick (well off the floor) places the ankle
    // itself, with the foot pointed along the shin.
    let (dx, dy, k) = foot_dir(knee, ankle, fwd, ground, l.size);
    let rest = ground - ANKLE * b - LINE;
    let planted = ankle.1 - (ANKLE * b + LINE) * (1.0 - k) >= rest - 0.5;
    let flat = V(ankle.0, (ankle.1 - (ANKLE * b + LINE) * (1.0 - k)).min(rest));

    // The foot in its own frame: `d` toward the toes, `n` toward the sole.
    let (nx, ny) = (-dy * fwd, dx * fwd);
    let at = |d: f32, n: f32| V(flat.0 + (dx * d + nx * n) * b, flat.1 + (dy * d + ny * n) * b);
    let toe_r = if booted { 3.4 } else { 2.8 };
    let (heel_at, ball, toe) = (at(-3.0, ANKLE - 5.0), at(8.0, ANKLE - 3.6),
                                at(FOOT - toe_r - LINE / b, ANKLE - toe_r));
    // Heel up: about the ball on the floor, about the ankle off it.
    let heel = if planted { heel.max(0.0) } else { heel };
    let (ankle, heel_at, ball, toe) = if planted {
        let pivot = at(8.0, ANKLE);
        (turn(flat, pivot, heel, fwd), turn(heel_at, pivot, heel, fwd), ball, toe)
    } else {
        (flat, turn(heel_at, flat, heel, fwd), turn(ball, flat, heel, fwd), turn(toe, flat, heel, fwd))
    };
    let pad = if booted { 0.5 } else { 0.0 };
    let foot = [(heel_at, ball, (5.0 + pad) * b, (3.6 + pad) * b), (ball, toe, (3.6 + pad) * b, toe_r * b)];

    // Thigh, a calf that swells below the knee, and a thin ankle.
    let (top, mid, calf, thin) = match l.build {
        Build::Gi => (10.0, 8.0, 7.6, 3.8),
        Build::Suit | Build::Spider | Build::Caped => (8.8, 6.4, 6.2, 4.2),
        Build::Bare => (10.0, 7.6, 7.2, 5.0),
        Build::Turtle | Build::Giant => (10.0, 7.4, 7.2, 4.0),
    };
    let swell = along(knee, ankle, 0.30);
    let thigh = (hip, knee, top * b, mid * b);
    let shin = [(knee, swell, mid * b, calf * b), (swell, ankle, calf * b, thin * b)];
    match l.build {
        // Bare legs and feet, one skin: knee pads, or trousers torn at the knee.
        Build::Turtle | Build::Giant => {
            shape(blip, &[thigh, shin[0], shin[1], foot[0], foot[1]], skin, ink);
            if l.build == Build::Turtle { blip.fill_circle(knee.0, knee.1, mid * b, trim); }
            else { stroke(blip, hip, along(hip, knee, 0.85), top * b, (mid + 0.4) * b, cloth); }
        }
        // Loose trousers to the shin, bare below.
        Build::Gi => {
            let hem = along(swell, ankle, 0.72);
            shape(blip, &[(hem, ankle, 4.6 * b, thin * b), foot[0], foot[1]], skin, ink);
            shape(blip, &[thigh, shin[0], (swell, hem, calf * b, 6.6 * b)], cloth, ink);
        }
        // Boots, with a shaft up the shin: dark leather on the wrestler.
        _ => {
            shape(blip, &[thigh, shin[0], shin[1]], cloth, ink);
            let boot = if l.build == Build::Bare { l.c(BOOT) } else { trim };
            let cuff = along(swell, ankle, 0.45);
            shape(blip, &[(cuff, ankle, (thin + 1.6) * b, (thin + 0.6) * b), foot[0], foot[1]], boot, ink);
        }
    }

    // The back of the near leg in shadow, like the back of the body.
    if !l.far {
        let behind = |a: V, c: V| {
            let (x, y) = unit(a, c);
            let (p, q) = ((y, -x), (-y, x));
            // Under the leg when it is raised, behind it when it stands.
            if -0.4 * p.0 * fwd + p.1 >= -0.4 * q.0 * fwd + q.1 { p } else { q }
        };
        let thigh_c = if l.build == Build::Turtle { skin } else { cloth };
        let (px, py) = behind(hip, knee);
        let off = |v: V, d: f32| V(v.0 + px * d * b, v.1 + py * d * b);
        stroke(blip, off(along(hip, knee, 0.25), top * 0.85 - 3.0), off(knee, mid - 2.8), 2.8 * b, 2.6 * b,
            shade(thigh_c, 0.84));
    }

    // What makes it a foot and not a shoe-shape, on the near one only: the
    // big toe set apart and the ankle bone, or a sole under the boot.
    if l.far { return; }
    let dark = shade(skin, 0.62);
    if booted {
        let sole = shade(if l.build == Build::Bare { l.c(BOOT) } else { trim }, 0.55);
        let under = |p: V, r: f32| V(p.0 + nx * r, p.1 + ny * r);
        stroke(blip, under(heel_at, 4.3 * b), under(ball, 3.0 * b), 1.1 * b, 1.0 * b, sole);
        stroke(blip, under(ball, 3.0 * b), under(toe, (toe_r - 1.0) * b), 1.0 * b, 0.9 * b, sole);
    } else {
        let (tx, ty) = unit(ball, toe);
        let gap = V(toe.0 - tx * 2.2 * b, toe.1 - ty * 2.2 * b);
        stroke(blip, V(gap.0 - nx * 2.2 * b, gap.1 - ny * 2.2 * b), gap, 0.6, 0.6, dark);
        if l.build == Build::Turtle {
            // Two broad toes, not five.
            let mid = along(ball, toe, 0.2);
            stroke(blip, V(mid.0 - nx * 2.8 * b, mid.1 - ny * 2.8 * b), mid, 0.6, 0.6, dark);
        } else {
            blip.fill_circle(ankle.0 - dx * 0.8 * b, ankle.1 + 0.6 * b, 1.0 * b.min(1.3), dark);
        }
    }
}

/// An arm and its hand. `hand` is the middle of the fist; the wrist is short
/// of it. `fwd` keeps the palm side down whichever way the fighter faces.
fn draw_arm(blip: &Blip, l: &Look, shoulder: V, elbow: V, hand: V, open: bool, fwd: f32) {
    let b = l.bulk;
    let ink = l.ink();
    let (skin, trim) = (l.c(l.skin), l.c(l.trim));
    let sleeved = matches!(l.build, Build::Suit | Build::Spider | Build::Caped);
    let (dx, dy) = unit(elbow, hand);
    let (px, py) = (-dy * fwd, dx * fwd);
    let at = |f: f32, a: f32| V(hand.0 + (dx * f + px * a) * b, hand.1 + (dy * f + py * a) * b);

    // Shoulder cap, an upper arm that swells at the biceps, a forearm that
    // swells below the elbow and narrows to the wrist.
    let wrist = at(-4.0, 0.0);
    let biceps = along(shoulder, elbow, 0.45);
    let swell = along(elbow, wrist, 0.30);
    shape(blip, &[
        (shoulder, biceps, 6.8 * b, 6.2 * b),
        (biceps, elbow, 6.2 * b, 4.6 * b),
        (elbow, swell, 4.6 * b, 5.2 * b),
        (swell, wrist, 5.2 * b, 3.6 * b),
    ], if sleeved { l.c(l.cloth) } else { skin }, ink);
    // What is worn on the forearm, inside the arm's own outline.
    let band = |from: f32, to: f32| stroke(blip, along(swell, wrist, from), along(swell, wrist, to),
        (5.2 - 1.6 * from) * b, (5.2 - 1.6 * to) * b, trim);
    match l.build {
        Build::Giant | Build::Caped => {}
        Build::Spider => band(0.2, 1.0),
        Build::Turtle => {
            blip.fill_circle(elbow.0, elbow.1, 4.6 * b, trim);
            band(0.45, 1.0);
        }
        _ => band(0.45, 1.0),
    }

    let lines = !l.far;
    let dark = shade(skin, 0.60);
    let crease = |a: V, c: V| stroke(blip, a, c, 0.5 * b.max(1.0), 0.5 * b.max(1.0), dark);
    if open {
        // A hand: palm, fingers a little apart, the thumb out on top. A
        // turtle has three fingers.
        let fingers: &[f32] = if l.build == Build::Turtle { &[-2.0, 0.0, 2.0] } else { &[-2.4, -0.8, 0.8, 2.4] };
        let r = if l.build == Build::Turtle { 1.5 } else { 1.15 };
        let mut caps = vec![(at(-3.0, 0.0), at(1.0, 0.0), 3.3 * b, 3.3 * b),
                            (at(-1.0, -2.6), at(2.4, -4.8), 1.4 * b, 1.1 * b)];
        for a in fingers { caps.push((at(2.0, *a), at(6.2, a * 1.25 + 0.6), r * b, (r - 0.2) * b)); }
        shape(blip, &caps, skin, ink);
        if lines {
            for w in fingers.windows(2) {
                let a = (w[0] + w[1]) / 2.0;
                crease(at(2.8, a), at(6.0, a * 1.25 + 0.6));
            }
        }
    } else {
        // A fist in profile: the flat back of the hand, the fingers curled
        // down the front under two knuckles, the thumb locked across them.
        shape(blip, &[
            (at(-3.2, -0.8), at(1.6, -1.2), 3.3 * b, 3.3 * b),
            (at(2.0, -1.0), at(1.2, 2.0), 2.9 * b, 2.9 * b),
            (at(2.6, -3.5), at(0.6, -3.9), 1.3 * b, 1.2 * b),
        ], skin, ink);
        if lines {
            // The thumb's edge and tip, and the fold of the fingers.
            crease(at(-1.6, 0.9), at(2.3, 1.9));
            crease(at(2.3, 1.9), at(2.5, 3.5));
            crease(at(4.0, -1.6), at(4.4, 0.4));
        }
    }
}

/// A wedge: narrow hips, wide chest.
const WAIST_R: f32 = 8.6;
const CHEST_R: f32 = 12.8;
const CHEST_AT: f32 = 0.60;
const BELT_AT: f32 = 0.30;
/// The body's radius at the belt.
const BELT_R: f32 = WAIST_R + (CHEST_R - WAIST_R) * BELT_AT / CHEST_AT;

const GOLD: BlipColor = BlipColor { r: 1.0, g: 0.84, b: 0.22, a: 1.0 };
const BOOT: BlipColor = BlipColor { r: 0.24, g: 0.16, b: 0.13, a: 1.0 };
const PLASTRON: BlipColor = BlipColor { r: 0.92, g: 0.80, b: 0.44, a: 1.0 };

fn draw_torso(blip: &Blip, l: &Look, hip: V, neck: V, fwd: f32) {
    let b = l.bulk;
    let ink = l.ink();
    let chest = along(hip, neck, CHEST_AT);
    let front = |k: f32, out: f32| { let v = along(hip, neck, k); V(v.0 + fwd * out * b, v.1) };
    let body = match l.build {
        Build::Gi | Build::Suit | Build::Caped => l.c(l.cloth),
        Build::Bare | Build::Giant => l.c(l.skin),
        Build::Turtle => l.c(PLASTRON),
        Build::Spider => l.c(l.trim),
    };
    // The shell, behind the body.
    if l.build == Build::Turtle {
        shape(blip, &[(front(0.10, -7.0), front(0.72, -8.0), 10.0 * b, 11.5 * b)], l.c(l.cloth), ink);
    }
    // The ribcage, and above it the shoulders narrowing to the neck, both
    // centred on the spine: the back is as flat as the chest is deep, and
    // slopes in to the neck instead of standing up behind it.
    let trunk = [(hip, chest, WAIST_R * b, CHEST_R * b),
                 (chest, along(hip, neck, 0.90), CHEST_R * b, 6.5 * b)];
    // Under it the pelvis, the seat both thighs come out of, in the near
    // side's colour and inside the same outline: it covers the root of the
    // far leg, so that leg is not stuck on behind.
    let (sx, sy) = unit(hip, neck);
    let seat_at = |across: f32| V(hip.0 + (-sy * fwd * across - sx) * b, hip.1 + (sx * fwd * across - sy) * b);
    let seat = (seat_at(-2.5), seat_at(3.0), 9.0 * b, 9.0 * b);
    let seat_c = if l.build == Build::Turtle { l.c(l.skin) } else { l.c(l.cloth) };
    for &(a, c, r1, r2) in trunk.iter().chain([&seat]) { stroke(blip, a, c, r1 + LINE, r2 + LINE, ink); }
    stroke(blip, seat.0, seat.1, seat.2, seat.3, seat_c);
    for &(a, c, r1, r2) in &trunk { stroke(blip, a, c, r1, r2, body); }
    let crease = |a: V, b: V| stroke(blip, a, b, 1.0, 1.0, shade(body, 0.62));
    match l.build {
        // The open collar of a gi: skin in a V down to the belt.
        Build::Gi => stroke(blip, front(0.92, 1.0), front(0.42, 2.0), 4.6 * b, 1.2 * b, l.c(l.skin)),
        // Trousers to the waist, and the line under the chest.
        Build::Bare | Build::Giant => {
            stroke(blip, hip, along(hip, neck, BELT_AT), WAIST_R * b, BELT_R * b, l.c(l.cloth));
            // That stroke ends in a dome above the belt; the skin is put
            // back over it in slices across the body, so the trousers stop
            // at the waist.
            let (sx, sy) = unit(hip, neck);
            let len = dist(hip, neck);
            let mut t = BELT_AT;
            while t < BELT_AT + (BELT_R * b + 2.0) / len {
                let at = along(hip, neck, t);
                let w = (WAIST_R + (CHEST_R - WAIST_R) * (t / CHEST_AT).min(1.0)) * b - 1.2;
                stroke(blip, V(at.0 + sy * w, at.1 - sx * w), V(at.0 - sy * w, at.1 + sx * w), 1.2, 1.2, body);
                t += 1.0 / len;
            }
            // The chest faces the viewer: a line under each side of it, the
            // breastbone between, and the top of the stomach.
            crease(front(0.66, -8.0), front(0.61, -1.0));
            crease(front(0.61, 1.5), front(0.66, 9.0));
            crease(front(0.84, 0.5), front(0.63, 0.5));
            crease(front(0.50, 0.5), front(0.40, 0.5));
        }
        // A stripe down the front of the suit.
        Build::Suit => stroke(blip, front(0.36, 1.0), front(0.82, 1.0), 2.0 * b, 2.0 * b, l.c(l.trim)),
        // The plates of the plastron.
        Build::Turtle => {
            crease(front(0.52, -8.0), front(0.50, 10.0));
            crease(front(0.74, -7.0), front(0.72, 10.0));
            crease(front(0.86, 1.0), front(0.34, 1.0));
        }
        // The shield on the chest. No trunks: a round red seat under a belt
        // is a nappy.
        Build::Caped => {
            let at = front(0.70, 1.0);
            blip.fill_circle(at.0, at.1, 5.6 * b, l.c(GOLD));
            blip.fill_circle(at.0, at.1, 3.4 * b, l.c(l.trim));
        }
        // The spider on the chest.
        Build::Spider => {
            let at = front(0.66, 1.0);
            blip.fill_circle(at.0, at.1, 2.4 * b, ink);
            for (dx, dy) in [(-1.0f32, -1.0f32), (1.0, -1.0), (-1.0, 1.0), (1.0, 1.0)] {
                stroke(blip, at, V(at.0 + dx * 5.0 * b, at.1 + dy * 4.0 * b), 0.8, 0.8, ink);
            }
        }
    }

    // One shadow tone down the back and under the seat: the light is in
    // front and above, and a body with no dark side is a paper cut-out.
    if l.build == Build::Turtle { return; }
    let radius = |t: f32| if t <= CHEST_AT { WAIST_R + (CHEST_R - WAIST_R) * t / CHEST_AT }
        else { CHEST_R + (6.5 - CHEST_R) * (t - CHEST_AT) / (0.90 - CHEST_AT) };
    let back = |t: f32, r: f32| {
        let v = along(hip, neck, t);
        let out = (radius(t) - r) * b;
        V(v.0 + sy * fwd * out, v.1 - sx * fwd * out)
    };
    let dim = |c: BlipColor| shade(c, 0.84);
    let lower = if matches!(l.build, Build::Bare | Build::Giant) { l.c(l.cloth) } else { body };
    stroke(blip, back(0.38, 3.4), back(0.62, 3.6), 3.4 * b, 3.6 * b, dim(body));
    stroke(blip, back(0.62, 3.6), back(0.84, 2.6), 3.6 * b, 2.6 * b, dim(body));
    stroke(blip, back(0.04, 3.0), back(0.24, 3.2), 3.0 * b, 3.2 * b, dim(lower));
    let under = |across: f32, down: f32| V(seat.0 .0 + (-sy * fwd * across - sx * down) * b,
                                           seat.0 .1 + (sx * fwd * across - sy * down) * b);
    stroke(blip, under(-5.0, 1.0), under(-1.5, 5.6), 3.2 * b, 3.0 * b, dim(seat_c));
}

/// The cape: pinned across the shoulders and lying down the back before it
/// hangs, so it trails behind a body that leans instead of dropping between
/// the legs; in flight (`level`) it streams straight back along the body.
fn draw_cape(blip: &Blip, l: &Look, hip: V, neck: V, fwd: f32, level: f32, now: f32) {
    let b = l.bulk;
    let (sx, sy) = unit(hip, neck);
    let top = V(neck.0 - sx * 6.0 * b - fwd * 5.0 * b, neck.1 - sy * 6.0 * b);
    // Half down the spine, half straight down with a little drift behind.
    let follow = 0.5 + 0.5 * level;
    let (hx, hy) = (-fwd * 0.30, 1.0);
    let (dx, dy) = (hx + (-sx - hx) * follow, hy + (-sy - hy) * follow);
    let m = dx.hypot(dy).max(1e-3);
    let (dx, dy) = (dx / m, dy / m);
    let wave = (now * (3.0 + 6.0 * level)).sin() * (1.5 + 3.0 * level) * b;
    let len = 64.0 * b;
    let at = |k: f32, side: f32| V(top.0 + dx * len * k - dy * side, top.1 + dy * len * k + dx * side);
    shape(blip, &[(top, at(0.45, 0.0), 8.0 * b, 11.0 * b), (at(0.45, 0.0), at(1.0, wave), 11.0 * b, 7.0 * b)],
        l.c(l.trim), l.ink());
}

/// The belt, drawn after the near leg so it lies over the top of the thigh.
fn draw_belt(blip: &Blip, l: &Look, hip: V, neck: V, fwd: f32) {
    let b = l.bulk;
    let c = match l.build {
        Build::Gi => l.c(BlipColor { r: 0.13, g: 0.12, b: 0.15, a: 1.0 }),
        Build::Turtle => l.c(BlipColor { r: 0.38, g: 0.24, b: 0.13, a: 1.0 }),
        Build::Bare | Build::Suit => l.c(l.trim),
        Build::Caped => l.c(GOLD),
        Build::Spider | Build::Giant => return,
    };
    let at = along(hip, neck, BELT_AT);
    let (sx, sy) = unit(hip, neck);
    let (px, py) = (-sy, sx);
    let r = 2.8 * b;
    let w = BELT_R * b - r;
    stroke(blip, V(at.0 - px * w, at.1 - py * w), V(at.0 + px * w, at.1 + py * w), r, r, c);
    let knot = V(at.0 + fwd * 1.0 * b, at.1);
    match l.build {
        // The knot and one end hanging off it.
        Build::Gi => stroke(blip, knot, V(knot.0 + fwd * 1.5, knot.1 + 10.0 * b), 2.2 * b, 1.2 * b, c),
        // A buckle in the mask's colour.
        Build::Turtle => blip.fill_circle(knot.0, knot.1, 3.4 * b, l.c(l.trim)),
        _ => {}
    }
}

fn draw_head(blip: &Blip, l: &Look, fw: f32, head: V, neck: V, mood: Mood) {
    let r = head_r(l.bulk / l.size) * l.size;
    let ink = l.ink();
    let hair = l.c(l.hair);
    let masked = l.build == Build::Spider;
    let skin = if masked { l.c(l.trim) } else { l.c(l.skin) };
    // Every offset is in head radii, so the head scales whole: `f` toward
    // the face, `d` down.
    let at = |f: f32, d: f32| V(head.0 + fw * f * r, head.1 + d * r);
    let dot = |f: f32, d: f32, k: f32| (at(f, d), at(f, d), k * r, k * r);

    // Hair first, since it sticks out behind the skull and needs the line.
    let locks: &[Cap] = match l.build {
        // Short and spiked.
        Build::Gi => &[dot(-0.22, -0.42, 0.80), dot(-0.66, 0.02, 0.52),
                       (at(0.25, -0.80), at(0.62, -1.02), 0.30 * r, 0.10 * r),
                       (at(-0.25, -0.90), at(-0.30, -1.22), 0.34 * r, 0.10 * r)],
        // Swept back into a tail.
        Build::Suit => &[dot(-0.26, -0.40, 0.78),
                         (at(-0.90, -0.10), at(-1.45 + l.wind.0, 0.85 + l.wind.1), 0.36 * r, 0.14 * r)],
        // A mop, falling over the brow.
        Build::Giant => &[dot(-0.26, -0.50, 0.78), dot(-0.70, -0.02, 0.50),
                          dot(0.22, -0.86, 0.36)],
        // Neat, with one curl on the forehead.
        Build::Caped => &[dot(-0.22, -0.44, 0.80), dot(-0.66, 0.00, 0.50),
                          dot(0.52, -0.66, 0.20)],
        Build::Bare | Build::Turtle | Build::Spider => &[],
    };
    for &(a, b, r1, r2) in locks { stroke(blip, a, b, r1 + LINE, r2 + LINE, ink); }

    // Neck, skull and jaw as one silhouette. The jaw hung off the front of
    // the skull is what makes it a profile; a turtle has a muzzle instead.
    let neck = (neck, at(-0.12, 0.55), 5.2 * l.bulk, 4.8 * l.bulk);
    match l.build {
        Build::Turtle => shape(blip, &[neck, dot(0.0, 0.0, 1.0),
            (at(0.16, 0.36), at(0.56, 0.42), 0.62 * r, 0.56 * r)], skin, ink),
        Build::Spider => shape(blip, &[neck, dot(0.0, 0.0, 1.0),
            (at(0.08, 0.30), at(0.40, 0.62), 0.60 * r, 0.42 * r)], skin, ink),
        _ => shape(blip, &[neck, dot(0.0, 0.0, 1.0),
            (at(0.08, 0.30), at(0.44, 0.64), 0.60 * r, 0.42 * r),
            dot(0.98, 0.14, 0.15)], skin, ink),
    }
    for &(a, b, r1, r2) in locks { stroke(blip, a, b, r1, r2, hair); }

    // A band round the head with two ends trailing behind: a headband at the
    // hairline, or a turtle's mask across the eyes.
    let band = |d: f32, w: f32| {
        let c = l.c(l.trim);
        let knot = at(-0.94, d + 0.14);
        let (wf, wd) = l.wind;
        stroke(blip, knot, at(0.92, d), w * r, w * r, c);
        stroke(blip, knot, at(-1.80 + wf, d + 0.42 + wd), 0.15 * r, 0.07 * r, c);
        stroke(blip, knot, at(-1.62 + wf * 0.8, d + 0.78 + wd * 1.2), 0.15 * r, 0.07 * r, c);
    };
    match l.build {
        Build::Gi => band(-0.40, 0.19),
        Build::Turtle => band(-0.14, 0.30),
        // A beard along the jaw.
        Build::Bare => stroke(blip, at(-0.10, 0.52), at(0.50, 0.70), 0.44 * r, 0.40 * r, hair),
        _ => {}
    }
    let dark = blend(INK, BLIP_WHITE, l.flash * 0.6);
    let line = |a: V, b: V, k: f32| stroke(blip, a, b, k * r, k * r, dark);

    // The mask has no face, only two big lenses that narrow when hurt.
    if masked {
        let k = if mood == Mood::Hurt { 0.10 } else { 0.20 };
        stroke(blip, at(0.30, -0.26), at(0.72, 0.02), (k + 0.08) * r, (k + 0.06) * r, dark);
        stroke(blip, at(0.30, -0.26), at(0.72, 0.02), k * r, (k - 0.02) * r, BLIP_WHITE);
        return;
    }
    if l.build != Build::Turtle {
        let ear = at(-0.14, 0.08);
        blip.fill_circle(ear.0, ear.1, 0.17 * r, shade(skin, 0.80));
    }

    // The face: brow, eye, mouth, and each one changes with the mood.
    let brow = if l.build == Build::Giant { 0.14 } else { 0.09 };
    if mood == Mood::Hurt {
        // Screwed shut.
        line(at(0.34, -0.24), at(0.66, -0.08), 0.07);
        line(at(0.66, -0.08), at(0.34, 0.08), 0.07);
    } else {
        let eye = at(0.50, -0.08);
        blip.fill_circle(eye.0, eye.1, 0.22 * r, BLIP_WHITE);
        blip.fill_circle(eye.0 + fw * 0.08 * r, eye.1, 0.12 * r, dark);
        if mood == Mood::Happy { line(at(0.24, -0.44), at(0.80, -0.44), brow); }
        else { line(at(0.20, -0.40), at(0.84, -0.24), brow); }
    }
    match mood {
        Mood::Calm => line(at(0.50, 0.50), at(0.80, 0.48), 0.06),
        Mood::Happy => {
            line(at(0.42, 0.44), at(0.62, 0.56), 0.06);
            line(at(0.62, 0.56), at(0.84, 0.42), 0.06);
        }
        Mood::Shout | Mood::Hurt => {
            let m = at(0.62, 0.50);
            blip.fill_circle(m.0, m.1, 0.17 * r, BlipColor { r: 0.36, g: 0.08, b: 0.10, a: 1.0 });
        }
    }
}

fn look_of(a: &Archetype, flash: f32) -> Look {
    Look {
        cloth: rgb(a.color),
        trim: rgb(a.trim),
        skin: rgb(a.skin),
        hair: rgb(a.hair),
        build: a.build,
        bulk: a.bulk * a.size,
        size: a.size,
        far: false,
        tone: 1.0,
        wind: (0.0, 0.0),
        flash,
    }
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
    /// How far each heel is raised, in radians (see `draw_leg`).
    lead_heel: f32,
    rear_heel: f32,
    /// How far the shoulders are turned: 0 is the open stance, chest to the
    /// viewer, the near shoulder back and the far one forward; 1 has the near
    /// shoulder driven through in front, as at the end of a punch.
    twist: f32,
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
            lead_heel: self.lead_heel + (o.lead_heel - self.lead_heel) * k,
            rear_heel: self.rear_heel + (o.rear_heel - self.rear_heel) * k,
            twist: self.twist + (o.twist - self.twist) * k,
        }
    }
}

/// The walk: one foot's (forward, lift) through a stride. Planted, it slides
/// back at exactly the body's speed, so on the boards it stays put.
pub(crate) const STRIDE: f32 = std::f32::consts::PI / (2.0 * STEP);  // radians of cycle per pixel travelled
pub(crate) const STEP: f32 = 18.0;
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

/// The heel through the same stride: it peels off the floor as the foot
/// pushes away, hangs toes-down through the swing, and the toes come up just
/// before the foot is set down again.
fn heel_cycle(ph: f32) -> f32 {
    let u = (ph / std::f32::consts::TAU).rem_euclid(1.0);
    if u < 0.5 {
        let k = ((u - 0.28) / 0.22).clamp(0.0, 1.0);
        0.7 * k * k * (3.0 - 2.0 * k)
    } else {
        let k = (u - 0.5) * 2.0;
        0.7 * (1.0 - k) * (1.0 - k) - 0.3 * k * (k * std::f32::consts::PI).sin()
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

/// How one fighter carries themself: offsets on the shared stance, so nine
/// fighters do not stand as one.
struct Style {
    /// The idle bounce: how fast (radians a second) and how big.
    rate: f32,
    amp: f32,
    /// Hips this much lower, feet this much further apart, head this much
    /// further forward.
    sink: f32,
    wide: f32,
    lean: f32,
    /// Where the guard is held, as offsets on the standard hands.
    lead: P,
    rear: P,
    /// Open hands in the guard: a grappler, not a boxer.
    open: bool,
    /// Wins with both arms in the air instead of one.
    cheer: bool,
}

const fn style(rate: f32, amp: f32, sink: f32, wide: f32, lean: f32, lead: P, rear: P,
               open: bool, cheer: bool) -> Style {
    Style { rate, amp, sink, wide, lean, lead, rear, open, cheer }
}

/// One per fighter, in roster order.
const STYLES: [Style; FIGHTERS.len()] = [
    // RYUKA: the textbook karate stance.
    style(4.4, 1.0, 0.0, 0.0, 0.0, p(0.0, 0.0), p(0.0, 0.0), false, false),
    // BRUTUS: a wrestler, wide and low, open hands ready to grab.
    style(3.0, 0.8, 3.0, 4.0, 2.0, p(-4.0, -8.0), p(-4.0, -13.0), true, true),
    // KESTREL: a kickboxer on the toes, hands high.
    style(6.5, 1.4, 0.0, -5.0, -2.0, p(-2.0, 8.0), p(5.0, 3.0), false, false),
    // GIOTTO: the disciplined one, rear fist by the chin.
    style(4.0, 0.9, 1.0, 0.0, 0.0, p(2.0, -2.0), p(3.0, -4.0), false, false),
    // TITIAN: the hothead, hunched with the fists low.
    style(3.6, 1.0, 4.0, 3.0, 7.0, p(-6.0, -6.0), p(-2.0, -10.0), false, false),
    // VERMEER: tall, the lead hand held out long and open.
    style(4.2, 0.8, 0.0, -2.0, -3.0, p(5.0, 4.0), p(-4.0, -6.0), true, false),
    // BOSCH: never still.
    style(7.5, 1.8, 1.0, 0.0, 2.0, p(-4.0, 6.0), p(0.0, -12.0), false, true),
    // WEBBER: low to the floor, fingers spread.
    style(5.5, 1.2, 7.0, 8.0, 8.0, p(-2.0, -18.0), p(8.0, -32.0), true, false),
    // GAMMA: hunched, the arms hanging heavy.
    style(2.6, 1.2, 4.0, 6.0, 9.0, p(-8.0, -14.0), p(-4.0, -20.0), false, true),
    // ZENITH: upright and unbothered, the fists low.
    style(3.2, 0.7, 0.0, -3.0, -3.0, p(-8.0, -14.0), p(-5.0, -22.0), false, false),
];

/// The stance: weight back, knees bent, both hands up, side on. Everything
/// departs from and returns to this. `bob` is the idle bounce, -1..1: the
/// whole body rides it, the hands a little further.
fn stance(bob: f32, who: usize) -> Pose {
    let s = &STYLES[who];
    let bob = bob * s.amp;
    Pose {
        hip: p(0.0, HIP_U - 1.0 - s.sink + bob * 1.2),
        head: p(3.0 + s.lean, HEAD_U - 1.0 - s.sink - s.lean * 0.35 + bob * 1.4),
        // The near fist is held back at the chest; the far one leads, out
        // past it and higher, so both show.
        lead_hand: p(22.0 + s.rear.f, 70.0 + s.rear.u - s.sink + bob * 1.8),
        rear_hand: p(35.0 + s.lead.f, 79.0 + s.lead.u - s.sink + bob * 2.2),
        lead_foot: p(24.0 + s.wide, 0.0),
        rear_foot: p(-22.0 - s.wide, 0.0),
        open: s.open,
        lead_heel: 0.0,
        rear_heel: 0.30 + 0.10 * bob,
        twist: 0.0,
    }
}

fn crouched(bob: f32) -> Pose {
    Pose {
        hip: p(-2.0, C_HIP + bob * 0.5),
        head: p(2.0, C_HEAD + bob * 0.6),
        lead_hand: p(12.0, 34.0 + bob * 0.6),
        rear_hand: p(28.0, 41.0 + bob * 0.8),
        lead_foot: p(18.0, 0.0),
        rear_foot: p(-19.0, 0.0),
        open: false,
        lead_heel: 0.10,
        rear_heel: 0.55,
        twist: 0.0,
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
        lead_hand: p(15.0 + 2.0 * rise + 2.0 * fall, 60.0 + 14.0 * rise - 4.0 * fall),
        rear_hand: p(27.0 + 2.0 * rise + 6.0 * fall, 69.0 + 14.0 * rise - 2.0 * fall),
        lead_foot: p(14.0 + 4.0 * tuck + 6.0 * fall, 8.0 + 24.0 * tuck),
        rear_foot: p(-16.0 + 6.0 * tuck + 2.0 * fall, 4.0 + 24.0 * tuck),
        open: false,
        lead_heel: 0.45,
        rear_heel: 0.55,
        twist: 0.0,
    }
}

/// A flier in the air: hanging upright with the fists on the hips, or laid
/// out level behind one fist, `level` of the way between.
fn flying_pose(level: f32) -> Pose {
    let hover = Pose {
        hip: p(0.0, 54.0),
        head: p(2.0, 106.0),
        lead_hand: p(11.0, 50.0),
        rear_hand: p(22.0, 57.0),
        lead_foot: p(12.0, 12.0),
        rear_foot: p(-8.0, 0.0),
        open: false,
        lead_heel: 0.5,
        rear_heel: 0.6,
        twist: 0.0,
    };
    let fly = Pose {
        hip: p(-10.0, 56.0),
        head: p(38.0, 72.0),
        lead_hand: p(78.0, 76.0),
        rear_hand: p(8.0, 50.0),
        lead_foot: p(-66.0, 46.0),
        rear_foot: p(-62.0, 56.0),
        open: false,
        lead_heel: 0.7,
        rear_heel: 0.7,
        twist: 1.0,
    };
    hover.to(fly, level * level * (3.0 - 2.0 * level))
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
        lead_heel: 0.3,
        rear_heel: 0.3,
        twist: 0.0,
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
        1.0 - back * back * (3.0 - 2.0 * back)
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

fn attack_pose(q: &mut Pose, f: &Fighter, m: &MoveData, ext: f32) {
    q.open = false;
    // The move is in screen pixels; the pose is a full-size fighter's.
    let z = f.size();
    let end = p(BODY_W / 2.0 + m.reach / z, m.height / z);
    let bulk = f.arch().bulk;
    let tip = p(end.f - FIST * bulk, end.u);
    let toe_tip = p(end.f - FOOT * bulk, end.u);

    enum Shape { Punch(bool), Kick(bool, f32), Fly, Sweep, Throw, Bolt, Laser, Rush, Call }
    let shape = match f.mv {
        MoveId::LowPunch | MoveId::HighPunch => Shape::Punch(false),
        MoveId::JumpPunch => Shape::Punch(true),
        MoveId::LowKick => Shape::Kick(false, 0.7),
        MoveId::HighKick => Shape::Kick(false, 1.25),
        MoveId::JumpKick => Shape::Kick(true, 1.0),
        MoveId::FlyingKick => Shape::Fly,
        MoveId::Sweep => Shape::Sweep,
        MoveId::Throw => Shape::Throw,
        MoveId::Special if f.calling => Shape::Call,
        MoveId::Special => match f.arch().special {
            Special::ChiBolt => Shape::Bolt,
            Special::LaserVision => Shape::Laser,
            Special::BullRush => Shape::Rush,
            Special::TalonKick => Shape::Kick(false, 1.25),
        },
    };

    match shape {
        // The epic punch: the fist drawn right back, then the whole body
        // thrown after it, back foot off the floor.
        Shape::Punch(air) if f.flies() => {
            let reach = if ext < 0.0 { ext * 5.0 } else { ext };
            let drive = ext.max(0.0);
            q.hip.f += 14.0 * drive;
            q.head.f += 30.0 * drive - 6.0 * (reach.min(0.0)).abs();
            if !air {
                q.lead_foot.f += 8.0 * drive;
                q.rear_foot = p(q.rear_foot.f - 12.0 * drive, 9.0 * drive);
            }
            q.rear_hand = q.rear_hand.to(p(q.head.f + 6.0, q.head.u - 30.0), drive);
            q.lead_hand = q.lead_hand.to(tip, reach);
            q.rear_heel = 0.9 * drive;
            q.twist = drive;
        }
        Shape::Punch(true) => {
            q.hip.f += 12.0 * ext;
            q.head.f += 24.0 * ext;
            q.lead_foot = p(q.hip.f + 14.0, q.lead_foot.u);
            q.rear_foot = p(q.hip.f - 10.0, q.rear_foot.u.min(24.0));
            q.rear_hand = q.rear_hand.to(p(q.head.f + 6.0, q.head.u - 30.0), ext.max(0.0));
            q.lead_hand = q.lead_hand.to(tip, ext);
            q.twist = ext.max(0.0);
        }
        Shape::Punch(false) => {
            let e = ext.max(0.0);
            // Step in and sit into it; a low one is reached by sinking at the
            // knees, not by folding over.
            let low = ((60.0 - tip.u) / 30.0).clamp(0.0, 1.0) * e;
            q.hip = p(q.hip.f + 4.0 * ext, q.hip.u - 2.0 * e - 9.0 * low);
            q.head.u -= 1.5 * e + 10.0 * low;
            q.lead_foot.f += 8.0 * ext;
            q.rear_foot.f -= 3.0 * ext;
            // The shoulder goes where a straight arm puts it: the body turns
            // and leans just far enough for the fist to land at full reach,
            // and no further.
            const ARM: f32 = UARM + FARM - 3.0;
            let drop = q.hip.u + SPINE * (q.head.u - q.hip.u) - 8.5 - tip.u;
            let shoulder = tip.f - (ARM * ARM - drop * drop).max(0.0).sqrt();
            let lean = (q.hip.f + (shoulder - 7.0 - q.hip.f) / SPINE - q.head.f).clamp(2.0, 24.0);
            q.head.f += lean * ext;
            if f.alt_punch {
                // A chain: the far fist goes straight down the middle from
                // the shoulder that is already forward, and the near one
                // comes back to the chest under it.
                q.rear_hand = q.rear_hand.to(tip, ext);
                q.lead_hand = q.lead_hand.to(p(q.head.f + 9.0, tip.u - 16.0), e.sqrt());
                q.head.f -= lean * 0.45 * ext;
            } else {
                // The far fist snaps back to guard the chin, out of the way
                // before the punch comes through where it was.
                q.rear_hand = q.rear_hand.to(p(q.head.f + 6.0, q.head.u - 22.0), e.sqrt());
                q.lead_hand = q.lead_hand.to(tip, ext);
                // The shoulder comes through with the hip.
                q.twist = e;
            }
            // The back foot turns over onto its ball as the hip comes through.
            q.rear_heel += 0.5 * e;
        }
        Shape::Kick(air, lift) => {
            let k = KICK;
            let e = ext.max(0.0);
            // 0 for the low kick, 1 for the high one.
            let high = ((lift - 0.7) / 0.55).clamp(0.0, 1.0);
            // The higher the kick the further the shoulders go back to pay
            // for it, and the more the standing leg straightens under it.
            let back = if air { k.air_lean * lift } else { 0.35 + 0.90 * high };
            let drive = if air { k.hip_drive + 9.0 } else { k.hip_drive };
            let rise = k.hip_rise + if air { 0.0 } else { 4.0 * high };
            q.hip = p(q.hip.f + drive * ext, q.hip.u + rise * ext);
            q.head.f -= k.lean * 1.25 * ext * back;
            q.head.u -= 1.4 * ext;
            if !air {
                q.rear_foot = p(-18.0 + (q.hip.f - k.plant - -18.0) * ext, 0.0);
            } else {
                q.rear_foot = p(q.hip.f - 7.0, 26.0);
            }
            // Up on the ball of the standing foot. A high kick lands with
            // the foot pointed; a low one with it pulled back, toes up.
            if !air { q.rear_heel = 0.25 + (0.35 + 0.25 * high) * e; }
            q.lead_heel = if !air && high < 0.5 { -0.55 * e } else { 0.45 * e };
            let chamber = p(q.hip.f + k.knee_lead,
                if air { k.chamber_air } else { k.chamber * (0.72 + 0.28 * high) });
            let kc = kick_curve(ext);
            q.lead_foot = if kc < 1.0 {
                q.lead_foot.to(chamber, kc)
            } else {
                chamber.to(toe_tip, kc - 1.0)
            };
            if air && f.stomp {
                // The rage jump: both fists over the head on the way down.
                q.lead_hand = q.lead_hand.to(p(20.0, 128.0), ext);
                q.rear_hand = q.rear_hand.to(p(-12.0, 124.0), ext);
            } else {
                // The far fist stays out as the guard; the near one tucks in
                // at the ribs, in front of the body.
                q.rear_hand = q.rear_hand.to(p(34.0, 70.0), ext);
                let tuck = if air { p(q.hip.f + 10.0, q.hip.u - 6.0) } else { p(q.hip.f + 12.0, q.hip.u + 12.0) };
                q.lead_hand = q.lead_hand.to(tuck, ext);
            }
        }
        Shape::Fly => {
            q.hip = p(q.hip.f + 21.0 * ext, q.hip.u + 7.0 * ext);
            q.head = p(q.head.f - 22.0 * ext, q.head.u - 13.0 * ext);
            q.rear_foot = p(q.hip.f - 24.0 - 10.0 * ext, 14.0 + 4.0 * ext);
            q.lead_foot = q.lead_foot.to(toe_tip, ext);
            q.rear_hand = q.rear_hand.to(p(32.0, 70.0), ext);
            // The near fist is held to the chest, on the front of a body
            // that is laid right back.
            let chest = q.hip.to(q.head, 0.5);
            q.lead_hand = q.lead_hand.to(p(chest.f + 7.0, chest.u + 7.0), ext);
        }
        Shape::Sweep => {
            let hip_f = (tip.f - 52.0).clamp(2.0, 28.0);
            q.hip = p(hip_f * ext, C_HIP + 3.0 * ext);
            q.head = p(q.head.f - 4.0 * ext, C_HEAD - 7.0 * ext);
            q.rear_foot = p(-3.0, 2.0);
            q.lead_foot = q.lead_foot.to(toe_tip, ext);
            q.lead_hand = q.lead_hand.to(p(q.hip.f + 12.0, 24.0), ext);
            q.rear_hand = q.rear_hand.to(p(34.0, 44.0), ext);
            q.rear_heel = 0.7;
            q.lead_heel = 0.0;
        }
        Shape::Throw => {
            q.open = true;
            q.hip.f += 4.0 * ext;
            q.head.f += 8.0 * ext;
            q.lead_foot.f += 7.0 * ext;
            q.rear_hand = q.rear_hand.to(p(tip.f, tip.u + 6.0), ext);
            q.lead_hand = q.lead_hand.to(p(tip.f - 6.0, tip.u - 8.0), ext);
            q.twist = 0.5 * ext.max(0.0);
        }
        Shape::Bolt => {
            q.open = true;
            q.hip = p(2.0, 51.0);
            q.lead_foot = p(21.0, 0.0);
            q.rear_foot = p(-22.0, 0.0);
            // Drawn back to the hip, as far as the fighter's lean leaves room for.
            let charge = p(2.0 + STYLES[f.who].lean, 56.0);
            // The far hand cups the charge from above, and rides out over
            // the near one: wrists together, one palm over the other.
            let cup = p(charge.f + 16.0, charge.u + 6.0);
            let out = p(44.0, 70.0);
            if ext < 0.55 {
                let k = ext / 0.55;
                q.lead_hand = q.lead_hand.to(charge, k);
                q.rear_hand = q.rear_hand.to(cup, k);
                q.head.f -= 6.0 * k;
            } else {
                // The shoulders come round behind the palms as they go out.
                let k = (ext - 0.55) / 0.45;
                q.lead_hand = charge.to(out, k);
                q.rear_hand = cup.to(p(out.f - 8.0, out.u + 12.0), k);
                q.head.f += -6.0 + 14.0 * k;
                q.twist = k;
            }
        }
        // Fists on the hips, chin down, eyes on the target.
        Shape::Laser => {
            q.hip = q.hip.to(p(0.0, HIP_U + 1.0), ext);
            q.head = q.head.to(p(8.0, HEAD_U - 2.0), ext);
            q.lead_hand = q.lead_hand.to(p(11.0, 49.0), ext);
            q.rear_hand = q.rear_hand.to(p(23.0, 56.0), ext);
        }
        // A fist in the air and a shout over the shoulder: the others are
        // coming.
        Shape::Call => {
            let e = ext.max(0.0);
            q.head = p(q.head.f - 4.0 * e, q.head.u + 2.0 * e);
            q.lead_hand = q.lead_hand.to(p(30.0, 122.0), e);
            q.rear_hand = q.rear_hand.to(p(10.0, 46.0), e);
            q.twist = e;
        }
        Shape::Rush => {
            q.hip = p(8.0 * ext, HIP_U - 5.0 * ext);
            q.head.f += 20.0 * ext;
            q.lead_foot = p(15.0 + 17.0 * ext, 5.0 * ext);
            q.rear_foot.f -= 11.0 * ext;
            q.lead_hand = q.lead_hand.to(tip, ext);
            q.rear_hand = q.rear_hand.to(p(q.head.f + 6.0, q.head.u - 30.0), ext);
            q.rear_heel = 0.8 * ext.max(0.0);
            q.twist = ext.max(0.0);
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

/// Mid-stride, is the far foot the one in front? Where a walk stops, that
/// is the stance it leaves.
pub(crate) fn far_foot_leads(f: &Fighter) -> bool {
    let ph = f.x * f.facing * STRIDE / f.size();
    let (near, _) = foot_cycle(ph, 9.0);
    let (far, _) = foot_cycle(ph + std::f32::consts::PI, 8.0);
    far > near
}

/// Where both feet swing about in a walk, forward of the fighter's centre.
const WALK_MID: f32 = 2.0;
/// How far the hips come up out of the stance to walk.
const WALK_RISE: f32 = 3.5;

/// The idle bounce, -1 to 1, at the fighter's own rate. The two sides are out
/// of phase, or a mirror match reads as one animation played twice.
pub(crate) fn bounce_of(now: f32, who: usize, idx: usize) -> f32 {
    (now * STYLES[who].rate + idx as f32 * 2.3).sin()
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
    let bob = bounce_of(now, f.who, idx);

    if let Some(t) = down_time(f) {
        let down = floored();
        if t < 0.22 {
            let k = (t / 0.22).clamp(0.0, 1.0);
            let mut air = stance(0.0, f.who);
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
            return down.to(up, (k * 1.8).min(1.0)).to(stance(0.0, f.who), (k - 0.55).max(0.0) / 0.45);
        }
        return down;
    }

    let low = f.crouching()
        || (f.act == Act::Hitstun && matches!(f.prev_act, Act::Crouch))
        || (f.act == Act::Hitstun && f.prev_act == Act::Attack
            && matches!(f.prev_mv, MoveId::Sweep))
        || (f.act == Act::Hitstun && f.prev_act == Act::Block && f.crouch_block);
    let mut q = if f.soaring() {
        flying_pose(f.soar)
    } else if f.airborne() {
        airborne_pose(f.vy)
    } else if low {
        crouched(bob)
    } else {
        stance(bob, f.who)
    };

    match f.act {
        Act::Walk => {
            let ph = f.x * f.facing * STRIDE / f.size();
            let s = ph.sin();
            let (lf, ll) = foot_cycle(ph, 9.0);
            let (rf, rl) = foot_cycle(ph + std::f32::consts::PI, 8.0);
            // A real walk: the feet pass each other under the body, so the
            // front leg changes with every step.
            q.lead_foot = p(WALK_MID + lf, ll);
            q.rear_foot = p(WALK_MID + rf, rl);
            q.lead_heel = heel_cycle(ph);
            q.rear_heel = heel_cycle(ph + std::f32::consts::PI);
            // Up out of the stance to walk: the legs are under the body now,
            // and nearly straight as each foot passes.
            // (A fighter who sits low in the stance comes up most of that too.)
            let sink = 1.4 + 1.4 * (2.0 * ph).cos() - WALK_RISE - 0.8 * STYLES[f.who].sink;
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
            let b = bob;
            if f.crouch_block {
                q.lead_hand = p(12.0, 31.0 + b * 0.5);
                q.rear_hand = p(27.0, 42.0 + b * 0.7);
                q.head.f -= 4.0;
            } else {
                q.hip = p(-4.0, HIP_U - 2.0 + b * 0.6);
                q.head = p(-8.0 - b * 0.7, HEAD_U - 2.0 + b * 0.4);
                q.lead_hand = p(13.0 + b * 0.4, 73.0 + b * 0.6);
                q.rear_hand = p(26.0 + b * 0.6, 88.0 + b * 0.8);
                let ph = f.x * f.facing * STRIDE / f.size();
                let sink = 1.0 + (2.0 * ph).cos();
                let (lf, ll) = foot_cycle(ph, 7.0);
                let (rf, rl) = foot_cycle(ph + std::f32::consts::PI, 6.0);
                q.lead_foot = p(14.0 + lf, ll);
                q.rear_foot = p(-17.0 + rf, rl);
                q.lead_heel = heel_cycle(ph) * 0.6;
                q.rear_heel = heel_cycle(ph + std::f32::consts::PI) * 0.6;
                q.hip.u -= sink;
                q.head.u -= sink * 0.7;
                q.lead_hand.u -= sink * 0.7;
                q.rear_hand.u -= sink * 0.7;
            }
        }
        // Snapped back off the blow: head first, back foot skidding out.
        Act::Hitstun => {
            // The head goes first and furthest, and comes back last.
            let k = (1.0 - f.t * 3.4).clamp(0.40, 1.0);
            let drop = if low { 0.5 } else { 1.0 };
            q.hip = p(q.hip.f - 6.0 * k, q.hip.u - 3.0 * k);
            q.head = p(q.head.f - 24.0 * k * drop, q.head.u - 1.6);
            q.lead_foot = p(q.lead_foot.f - 10.0, 0.0);
            q.rear_foot = p(q.rear_foot.f - 6.0, 0.0);
            // The arms are thrown up and out by it, and drop back in.
            let fling = ((k - 0.4) / 0.6).max(0.0);
            q.lead_hand = p(q.lead_hand.f - 9.0 - 4.0 * fling, q.lead_hand.u - 5.0 + 9.0 * fling);
            q.rear_hand = p(q.rear_hand.f - 7.0 + 6.0 * fling, q.rear_hand.u - 9.0 + 12.0 * fling);
            // Rocked back onto flat feet, the hands knocked open.
            q.open = true;
            q.lead_heel = 0.0;
            q.rear_heel = 0.0;
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
            q.rear_heel *= 1.0 - settle;
            let ease = |from: f32, len: f32| {
                let k = ((t - from) / len).clamp(0.0, 1.0);
                k * k * (3.0 - 2.0 * k)
            };
            if STYLES[f.who].cheer {
                // Both arms up, the far one first so the near one comes up
                // outside it.
                q.rear_hand = q.rear_hand.to(p(20.0, 126.0 - bob), ease(0.0, 0.35));
                let lift = ease(0.12, 0.45);
                q.lead_hand = q.lead_hand.to(p(38.0, 118.0 + bob), lift);
                q.twist = lift;
            } else {
                // One fist straight up, the shoulder coming round under it;
                // the other waits for it to pass, then comes down to the hip.
                let lift = ease(0.0, 0.5);
                q.lead_hand = q.lead_hand.to(p(36.0, 120.0 + bob), lift);
                q.rear_hand = q.rear_hand.to(p(8.0, 42.0), ease(0.14, 0.30));
                q.twist = lift;
            }
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
                rear_hand: p(11.0 + 12.0 * bow, 44.0 - 12.0 * bow),
                lead_foot: p(6.0, 0.0),
                rear_foot: p(-6.0, 0.0),
                open: true,
                lead_heel: 0.0,
                rear_heel: 0.0,
                twist: 0.0,
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
            q.rear_heel = 0.9 * hips;
            q.lead_heel = 0.0;
            q.lead_hand = q.lead_hand.to(p(-2.0, 18.0 + heave * 0.5), slump);
            q.rear_hand = q.rear_hand.to(p(28.0, 28.0), slump);
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
        q.lead_heel *= 1.0 - dip.min(1.0);
        q.rear_heel *= 1.0 - dip.min(1.0);
    }

    if !f.airborne() && f.act != Act::Knockdown {
        q.lead_foot.u = q.lead_foot.u.max(0.0);
        q.rear_foot.u = q.rear_foot.u.max(0.0);
    }
    // The other foot forward: every pose that stands on the stance changes
    // legs, and a kick comes off whichever leg is in front. (A walk is
    // already on both; the bow and the end of the round stand square.)
    if f.far_leads && !f.airborne()
        && matches!(f.act, Act::Idle | Act::Crouch | Act::Block | Act::Hitstun | Act::Attack) {
        std::mem::swap(&mut q.lead_foot, &mut q.rear_foot);
        std::mem::swap(&mut q.lead_heel, &mut q.rear_heel);
    }
    q
}

/// Every joint of one fighter in screen coordinates, after the solver. The
/// fighter is drawn from it and the anatomy tests run against it.
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

/// Solve one fighter's pose into joints. `rig` maps pose space to the screen.
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
    let z = rig.scale;
    // An open stance, as every 2D fighter stands: the chest is turned to the
    // viewer, the near shoulder at the back edge of it and the far one at
    // the front. A punch turns the near shoulder through to the front.
    let t = q.twist.clamp(0.0, 1.0);
    let (sh_lead, _) = socket(neck, (-7.0 + 15.6 * t) * z, -8.5 * z);
    let (sh_rear, _) = socket(neck, (8.0 - 10.0 * t) * z, -8.5 * z);
    // Both legs hang from one pelvis: the far hip is just behind the near
    // one, not out at the back of the body.
    let (hip_lead, _) = socket(hip, 4.5 * z, 0.0);
    let (_, hip_rear) = socket(hip, 1.5 * z, 0.0);

    // Parallax on the far side: the far arm and leg sit slightly back, so a
    // guard shows two fists and a stance two feet. Applied to the target so
    // bone lengths stay. The camera is at chest height: a far foot appears
    // higher (toward the horizon), a far hand barely moves; shifting the foot
    // down puts it through the floor.
    let hand_back = |v: V| V(v.0 - rig.fwd * 3.0 * z, v.1 + z);
    let foot_back = |v: V| V(v.0 - rig.fwd * 3.5 * z, v.1 - 1.5 * z);

    let leg_at = |root: V, want: V| {
        // The knee's bend side turns with the thigh: a leg thrown out
        // horizontally has its anatomical forward pointing up, and a fixed
        // "forward" inverted the knee on the high kicks. So it comes from the
        // limb's own direction, a quarter turn from it.
        let (dx, dy) = unit(root, want);
        let anterior = V(dy * rig.fwd, -dx * rig.fwd);
        solve(root, want, THIGH * z, SHIN * z, KNEE_SHUT, anterior)
    };
    let arm_at = |root: V, want: V| {
        // The elbow breaks to the back of the arm and turns with it. A fixed
        // direction sat near the tie between the two mirrored solutions, so
        // the forearm flipped with a pixel of movement; squared to the arm it
        // is as far from the tie as possible. One rule at every angle (see
        // `no_elbow_sticks_out_behind_the_back`).
        // A hand posed too near its own shoulder is pushed out: folded
        // almost shut, the elbow's place is guesswork (see
        // `no_limb_is_folded_up_to_nothing`).
        let (dx, dy) = unit(root, want);
        let reach = dist(root, want).max(ARM_MIN * z);
        let want = V(root.0 + dx * reach, root.1 + dy * reach);
        let posterior = V(-dy * rig.fwd, dx * rig.fwd);
        solve(root, want, UARM * z, FARM * z, ELBOW_SHUT, posterior)
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

fn draw_fighter(blip: &Blip, g: &Game, i: usize) {
    let shake = if g.shake > 0.0 { (g.shake * 60.0).sin() * g.shake * 30.0 } else { 0.0 };

    // In an exchange the two fighters overlap four frames in five, so in
    // versus a mark in each player's colour on the boards under them shows
    // which is yours.
    if g.mode == Mode::Versus {
        let c = if i == 0 { BlipColor { r: 1.0, g: 0.35, b: 0.35, a: 0.55 } }
                else { BlipColor { r: 0.40, g: 0.72, b: 1.0, a: 0.55 } };
        let f = &g.p[i];
        let w = 21.0 * f.arch().bulk * f.size();
        stroke(blip, V(f.x - w, FLOOR_Y + shake + 2.0), V(f.x + w, FLOOR_Y + shake + 2.0),
            2.2, 2.2, c);
    }
    draw_body(blip, &g.p[i], g.now, shake, g.hitstop, i, true);
}

fn mood_of(f: &Fighter) -> Mood {
    match f.act {
        Act::Attack => Mood::Shout,
        Act::Hitstun | Act::Knockdown | Act::Defeat => Mood::Hurt,
        Act::Victory => Mood::Happy,
        _ => Mood::Calm,
    }
}

/// One fighter, posed and drawn. Drawing order is depth: far leg, far arm,
/// body, head, near leg, near arm, so an attack arrives in front of the body
/// that threw it.
/// `shift` moves the fighter down the screen without the floor: the hit
/// shake, and the select screen's row. Not part of `f.y`, which is where the
/// fighter is for the rules. `shadow` is off where there is no floor.
fn draw_body(blip: &Blip, f: &Fighter, now: f32, shift: f32, hitstop: f32, i: usize, shadow: bool) {
    let a = f.arch();
    let prone = down_time(f).is_some_and(|t| t < 0.9);
    let rig = Rig::upright(f.x, f.y + shift, f.facing, if prone { -f.facing } else { f.facing },
        a.size);

    let flash = if hitstop > 0.0 && f.act == Act::Hitstun { 0.5 }
                else if invulnerable(f) && ((now * 30.0) as i32) % 2 == 0 { 0.3 }
                else { 0.0 };
    // Loose ends trail: back when moving forward, down on the way up, up on
    // the way down, with a small flutter at rest.
    let drift = match f.act {
        Act::Walk => a.walk,
        _ => f.vx * f.facing,
    };
    let rise = if f.airborne() { f.vy } else { 0.0 };
    let wind = (-(drift / 260.0).clamp(-1.0, 1.0) * 0.5 + (now * 5.0 + i as f32).sin() * 0.05,
                -(rise / 600.0).clamp(-1.0, 1.0) * 0.7);
    let near = Look { wind, ..look_of(&a, flash) };
    let far = Look { far: true, tone: FAR, ..near };

    // Dust off the boards where a body lands. Knockdowns are the one
    // moment the floor is part of the fight.
    if let Some(t) = down_time(f).filter(|t| *t < 0.26) {
        let k = (1.0 - t / 0.26).max(0.0);
        for j in 0..5 {
            let dx = (j as f32 - 2.0) * 13.0 - f.facing * 14.0;
            let rise = (1.0 - k) * 16.0;
            blip.fill_circle(f.x + dx, FLOOR_Y + shift - 3.0 - rise, 3.0 + 7.0 * (1.0 - k),
                BlipColor { r: 0.62, g: 0.55, b: 0.46, a: 0.34 * k });
        }
    }

    // And a puff either side of the feet coming down from a jump, bigger
    // the harder the landing.
    if f.land > 0.0 && f.act != Act::Knockdown {
        let age = 1.0 - f.land / LAND_ABSORB;
        let size = f.size().max(0.6) * (0.5 + 0.5 * f.land_force);
        for side in [-1.0f32, 1.0] {
            let x = f.x + side * (12.0 + 18.0 * age) * size;
            blip.fill_circle(x, FLOOR_Y + shift - 2.0 - 5.0 * age, (3.0 + 5.0 * age) * size,
                BlipColor { r: 0.66, g: 0.60, b: 0.52, a: 0.42 * (1.0 - age) });
        }
    }

    // A flat oval on the boards, smaller under a jump: what glues the body
    // down, and how a player judges where a jump will land.
    if shadow {
        let lift = ((FLOOR_Y - f.y) / 120.0).clamp(0.0, 1.0);
        let (w, h) = (30.0 * a.bulk * a.size * (1.0 - 0.45 * lift), 5.0);
        let c = BlipColor { r: 0.0, g: 0.0, b: 0.0, a: 0.34 * (1.0 - 0.4 * lift) };
        for row in 0..h as i32 * 2 {
            let y = (row as f32 + 0.5) / h - 1.0;
            let half = w * (1.0 - y * y).sqrt();
            blip.fill_rect(f.x - half, FLOOR_Y + shift + 1.0 + row as f32 - h, half * 2.0, 1.0, c);
        }
    }

    // Rage: a red heat behind the fighter, beating.
    if f.enraged() && !matches!(f.act, Act::Defeat | Act::Victory | Act::Bow) {
        let beat = 0.5 + 0.5 * (now * 9.0).sin();
        blip.fill_glow_circle(f.x, f.y + shift - f.height() * 0.5, f.height() * (0.42 + 0.06 * beat),
            BlipColor { r: 1.0, g: 0.18, b: 0.10, a: 0.20 + 0.12 * beat });
    }

    // A turtle guarding low is drawn as what it is: a shell on the boards
    // with a pair of eyes under the rim.
    if f.shelled() {
        // As tall as the crouch it stands for, or blows land on thin air.
        let b = a.bulk * a.size * 1.7;
        let (gy, fw) = (f.y + shift, f.facing);
        shape(blip, &[(V(f.x - 13.0 * b, gy - 4.0 * b - LINE), V(f.x + 15.0 * b, gy - 4.0 * b - LINE), 3.2 * b, 3.2 * b)],
            near.c(near.skin), near.ink());
        shape(blip, &[(V(f.x - 8.0 * b, gy - 15.0 * b), V(f.x + 8.0 * b, gy - 15.0 * b), 12.0 * b, 12.0 * b)],
            near.c(near.cloth), near.ink());
        stroke(blip, V(f.x - 17.0 * b, gy - 8.0 * b), V(f.x + 17.0 * b, gy - 8.0 * b), 2.6 * b, 2.6 * b,
            near.c(PLASTRON));
        stroke(blip, V(f.x + fw * 9.0 * b, gy - 13.0 * b), V(f.x + fw * 17.0 * b, gy - 13.0 * b), 2.6 * b, 2.6 * b,
            near.c(near.trim));
        blip.fill_circle(f.x + fw * 15.0 * b, gy - 13.0 * b, 1.7 * b, BLIP_WHITE);
        return;
    }

    let q = pose_of(now, f, i);
    let k = skeleton(rig, &q);
    if a.build == Build::Caped { draw_cape(blip, &far, k.hip, k.neck, rig.fwd, f.soar, now); }
    draw_leg(blip, &far, k.hip_rear, k.knee_rear, k.ankle_rear, rig.fwd, rig.ground, q.rear_heel);
    // The far arm is beyond the chest: what shows of it is what clears it.
    draw_arm(blip, &far, k.sh_rear, k.elbow_rear, k.hand_rear, q.open, rig.fwd);
    draw_torso(blip, &near, k.hip, k.neck, rig.fwd);
    draw_head(blip, &near, rig.face, k.head, k.neck, mood_of(f));
    draw_leg(blip, &near, k.hip_lead, k.knee_lead, k.ankle_lead, rig.fwd, rig.ground, q.lead_heel);
    draw_belt(blip, &near, k.hip, k.neck, rig.fwd);
    draw_arm(blip, &near, k.sh_lead, k.elbow_lead, k.hand_lead, q.open, rig.fwd);

    // The laser's tell: the eyes light up through the whole of its startup.
    if f.act == Act::Attack && f.mv == MoveId::Special && a.special == Special::LaserVision {
        let charge = (f.t / (move_data(f.mv).startup * F)).min(1.0);
        if f.t < (move_data(f.mv).startup + 6.0) * F {
            let r = head_r(a.bulk) * a.size;
            let eye = V(k.head.0 + rig.face * 0.5 * r, k.head.1 - 0.08 * r);
            blip.fill_glow_circle(eye.0, eye.1, 6.0 + 12.0 * charge,
                BlipColor { r: 1.0, g: 0.16, b: 0.12, a: 0.5 });
            blip.fill_circle(eye.0, eye.1, 2.0 + 2.0 * charge, BlipColor { r: 1.0, g: 0.3, b: 0.2, a: 1.0 });
        }
    }

    // The epic punch splits the air: three lines fanning out ahead of the
    // fist while it travels and lands.
    if f.act == Act::Attack && f.epic_punch() {
        let m = move_data(f.mv);
        if f.t > m.startup * F * 0.6 && f.t < (m.startup + m.active + 3.0) * F {
            for up in [-0.7f32, 0.0, 0.7] {
                let (dx, dy) = (f.facing * up.cos(), up.sin());
                let at = |d: f32| V(k.hand_lead.0 + dx * d, k.hand_lead.1 + dy * d);
                stroke(blip, at(13.0), at(30.0), 2.0, 0.6, BLIP_WHITE);
            }
        }
    }
}

/// What each bolt-thrower throws: every one the same hitbox, each its own
/// picture.
fn draw_bolt(blip: &Blip, who: usize, at: V, dir: f32, now: f32, from: V) {
    let a = &FIGHTERS[who];
    let spin = now * 14.0 * dir;
    let arm = |ang: f32, r: f32| V(at.0 + ang.cos() * r, at.1 + ang.sin() * r);
    match a.special_name {
        // A slice-less pizza, spinning flat.
        "PIZZA TOSS" => {
            blip.fill_circle(at.0, at.1, 9.0, INK);
            blip.fill_circle(at.0, at.1, 7.5, BlipColor { r: 0.86, g: 0.62, b: 0.30, a: 1.0 });
            blip.fill_circle(at.0, at.1, 5.8, BlipColor { r: 0.98, g: 0.84, b: 0.36, a: 1.0 });
            for j in 0..3 {
                let v = arm(spin + j as f32 * 2.094, 3.2);
                blip.fill_circle(v.0, v.1, 1.5, BlipColor { r: 0.82, g: 0.18, b: 0.14, a: 1.0 });
            }
        }
        // A four-pointed star.
        "SHURIKEN" => {
            for (r, c) in [(10.0, INK), (8.0, BlipColor { r: 0.80, g: 0.84, b: 0.90, a: 1.0 })] {
                for j in 0..2 {
                    let ang = spin + j as f32 * 1.571;
                    stroke(blip, arm(ang, r), arm(ang + 3.1416, r), r * 0.14, r * 0.14, c);
                }
                blip.fill_circle(at.0, at.1, r * 0.4, c);
            }
            blip.fill_circle(at.0, at.1, 1.4, INK);
        }
        "LASER VISION" => {
            let red = BlipColor { r: 1.0, g: 0.16, b: 0.12, a: 1.0 };
            stroke(blip, from, at, 2.6, 3.4, red);
            stroke(blip, from, at, 1.0, 1.4, BLIP_WHITE);
            blip.fill_glow_circle(at.0, at.1, 10.0, BlipColor { a: 0.4, ..red });
        }
        // A ball of web on the end of its line.
        "WEB SHOT" => {
            stroke(blip, V(at.0 - dir * 30.0, at.1), at, 0.8, 1.6, BLIP_WHITE);
            blip.fill_circle(at.0, at.1, 8.0, INK);
            blip.fill_circle(at.0, at.1, 6.2, BLIP_WHITE);
        }
        _ => {
            blip.fill_glow_circle(at.0, at.1, 11.0, rgba(a.trim, 0.35));
            blip.fill_circle(at.0, at.1, 7.0, rgb(a.trim));
            blip.fill_circle(at.0 - dir * 5.0, at.1, 4.0, BLIP_WHITE);
        }
    }
}

/// One of the eight bonus fruits, sitting on the boards at `x`.
fn draw_fruit(blip: &Blip, kind: usize, x: f32, ground: f32) {
    let y = ground - 9.0;
    let c = |r: f32, g: f32, b: f32| BlipColor { r, g, b, a: 1.0 };
    let (red, green, gold) = (c(0.88, 0.14, 0.16), c(0.30, 0.70, 0.28), c(0.98, 0.84, 0.22));
    let ball = |dx: f32, dy: f32, r: f32, col: BlipColor| {
        blip.fill_circle(x + dx, y + dy, r + LINE, INK);
        blip.fill_circle(x + dx, y + dy, r, col);
    };
    let stalk = |x0: f32, y0: f32, x1: f32, y1: f32, col: BlipColor| {
        stroke(blip, V(x + x0, y + y0), V(x + x1, y + y1), 1.2, 1.0, col);
    };
    match kind {
        // Cherries: two on one stalk.
        0 => {
            stalk(-5.0, 0.0, 2.0, -12.0, green);
            stalk(5.0, 2.0, 2.0, -12.0, green);
            ball(-5.0, 2.0, 4.5, red);
            ball(5.0, 3.5, 4.5, red);
        }
        // Strawberry: seeded, with a leaf cap.
        1 => {
            ball(0.0, 0.0, 7.0, red);
            for (dx, dy) in [(-3.0f32, -1.0f32), (2.5, 1.0), (-0.5, 3.5), (3.0, -3.0)] {
                blip.fill_circle(x + dx, y + dy, 0.9, BLIP_WHITE);
            }
            stalk(-4.0, -7.0, 4.0, -7.0, green);
        }
        // Orange.
        2 => {
            ball(0.0, 0.0, 7.0, c(0.98, 0.58, 0.14));
            stalk(0.0, -7.0, 4.0, -10.0, green);
        }
        // Apple.
        3 => {
            ball(0.0, 0.0, 7.0, red);
            stalk(0.0, -6.0, 1.0, -11.0, c(0.45, 0.28, 0.14));
            blip.fill_circle(x - 2.5, y - 2.5, 1.6, BLIP_WHITE);
        }
        // Melon: netted green.
        4 => {
            ball(0.0, 0.0, 7.5, green);
            for dx in [-3.5f32, 0.0, 3.5] { stalk(dx, -6.0, dx, 6.0, c(0.60, 0.88, 0.50)); }
            stalk(0.0, -7.0, 0.0, -11.0, c(0.45, 0.28, 0.14));
        }
        // The flagship of another fleet: a yellow dart with red wings.
        5 => {
            ball(0.0, 0.0, 4.0, gold);
            stroke(blip, V(x - 8.0, y + 4.0), V(x - 2.0, y - 2.0), 2.4, 1.6, red);
            stroke(blip, V(x + 8.0, y + 4.0), V(x + 2.0, y - 2.0), 2.4, 1.6, red);
            stalk(0.0, -4.0, 0.0, -10.0, c(0.30, 0.45, 0.95));
        }
        // Bell.
        6 => {
            ball(0.0, -1.0, 6.5, gold);
            blip.fill_rect(x - 8.0, y + 3.0, 16.0, 3.0, gold);
            blip.fill_circle(x, y + 7.0, 1.8, c(0.75, 0.80, 0.90));
        }
        // Key.
        _ => {
            ball(0.0, -4.0, 4.5, c(0.45, 0.75, 0.95));
            blip.fill_rect(x - 1.5, y, 3.0, 9.0, c(0.80, 0.84, 0.90));
            blip.fill_rect(x + 1.5, y + 4.0, 3.0, 2.0, c(0.80, 0.84, 0.90));
        }
    }
}

fn draw_fight(blip: &Blip, g: &Game) {
    // The floor after a rage jump: dust thrown up along the whole of it,
    // rolling out from where he landed.
    if g.quake_t > 0.0 {
        let k = 1.0 - g.quake_t / QUAKE_SECS;
        for j in 0..16 {
            let x = (j as f32 + 0.5) * WIN_W as f32 / 16.0;
            // Each puff starts as the wave reaches it.
            let age = k - (x - g.quake_x).abs() / WIN_W as f32 * 0.5;
            if age <= 0.0 { continue; }
            blip.fill_circle(x, FLOOR_Y - 2.0 - age * 22.0, 4.0 + age * 12.0,
                BlipColor { r: 0.70, g: 0.62, b: 0.52, a: 0.5 * (1.0 - age).max(0.0) });
        }
    }
    // The fruit lies on the boards behind the fighters, and blinks before
    // it goes.
    if g.fruit.ttl > 0.0 && (g.fruit.ttl > 2.0 || (g.now * 8.0) as i32 % 2 == 0) {
        let shake = if g.shake > 0.0 { (g.shake * 60.0).sin() * g.shake * 30.0 } else { 0.0 };
        draw_fruit(blip, g.fruit.kind, g.fruit.x, FLOOR_Y + shake);
    }
    for i in 0..2 { draw_fighter(blip, g, i); }
    let shake = if g.shake > 0.0 { (g.shake * 60.0).sin() * g.shake * 30.0 } else { 0.0 };
    for (k, h) in g.helpers.iter().enumerate() {
        if let Some(h) = h.filter(|h| h.wait <= 0.0) { draw_body(blip, &h.f, g.now, shake, 0.0, 2 + k, true); }
    }
    for f in g.p.iter().filter(|f| f.webbed > 0.0) { draw_web(blip, f, shake, g.now); }

    for b in g.bolts.iter() {
        if !b.active { continue; }
        // While he is still staring, the laser runs unbroken from his eyes;
        // after that it is a pulse trailing the way it came.
        let f = &g.p[b.owner];
        let speed = b.vx.hypot(b.vy).max(1.0);
        let tail = V(b.x - b.vx / speed * 54.0, b.y - b.vy / speed * 54.0);
        let from = if f.act == Act::Attack && f.mv == MoveId::Special {
            V(f.x + f.facing * 14.0 * f.size(), f.y - 104.0 * f.size())
        } else {
            tail
        };
        draw_bolt(blip, f.who, V(b.x, b.y), b.vx.signum(), g.now, from);
    }

    for s in g.hitspark.iter().filter(|s| s.ttl > 0.0) { draw_splash(blip, s); }

    // Words for what just happened, popping up over it and rising.
    for w in g.pops.iter().filter(|w| w.ttl > 0.0) {
        let age = POP_SECS - w.ttl;
        let sz = if age < 0.08 { 3.0 } else { 2.0 };
        let (x, y) = (w.x - text_w(w.text, sz) / 2.0, w.y - age * 22.0);
        let x = x.clamp(4.0, WIN_W as f32 - text_w(w.text, sz) - 4.0);
        let a = (w.ttl * 4.0).min(1.0);
        for (dx, dy) in [(-1.0, 0.0), (1.0, 0.0), (0.0, -1.0), (0.0, 1.0)] {
            blip.draw_text(w.text, x + dx * sz, y + dy * sz, sz, BlipColor { a, ..INK });
        }
        blip.draw_text(w.text, x, y, sz, BlipColor { a, ..BLIP_WHITE });
    }

    // The combo count, over the shoulder of whoever earned it.
    if g.combo_t > 0.0 && g.combo_shown > 1 {
        let f = g.p[g.combo_side];
        let text = format!("{} HITS", g.combo_shown);
        let rise = (1.1 - g.combo_t) * 26.0;
        blip.draw_text(&text, f.x - text_w(&text, 2.0) / 2.0, f.y - STAND_H * f.size() - 26.0 - rise, 2.0,
            BlipColor { r: 1.0, g: 0.85, b: 0.25, a: g.combo_t.min(1.0) });
    }
}

/// A fighter wrapped in a web: rings and spokes round the whole body, tied
/// down to the boards. It thins with the first blow and flickers before it
/// lets go.
fn draw_web(blip: &Blip, f: &Fighter, shift: f32, now: f32) {
    if f.webbed < 1.0 && (now * 10.0) as i32 % 2 == 0 { return; }
    let h = f.height();
    let c = V(f.x, f.y + shift - h * 0.5);
    let (rx, ry) = (f.width() * 0.5 + 16.0, h * 0.5 + 9.0);
    let silk = BlipColor { r: 0.94, g: 0.96, b: 1.0, a: 1.0 };
    let line = |a: V, b: V| {
        stroke(blip, a, b, 1.9, 1.9, BlipColor { a: 0.5, ..INK });
        stroke(blip, a, b, 1.0, 1.0, silk);
    };
    let at = |ang: f32, k: f32| V(c.0 + ang.cos() * rx * k, c.1 + ang.sin() * ry * k);
    let torn = f.web_hits > 0;
    let spokes = 10;
    let step = std::f32::consts::TAU / spokes as f32;
    for j in 0..spokes {
        if torn && j % 3 == 1 { continue; }
        let ang = 0.3 + j as f32 * step;
        line(at(ang, 0.18), at(ang, 1.0));
        // Each ring sags between its spokes, as silk does.
        for (ring, k) in [0.45f32, 0.74, 1.0].into_iter().enumerate() {
            if torn && ring == 1 { continue; }
            let mid = at(ang + step / 2.0, k * 0.90);
            line(at(ang, k), mid);
            line(mid, at(ang + step, k));
        }
    }
    // Tied to the floor either side.
    let floor = FLOOR_Y + shift;
    for side in [-1.0f32, 1.0] {
        line(V(c.0 + side * rx * 0.8, c.1 + ry * 0.55), V(c.0 + side * (rx + 14.0), floor));
    }
}

/// Where a blow landed: a star of flat colour that snaps open and thins away.
/// Hot for damage, a cold four-point glint for a guard: whether the exchange
/// cost you reads from the shape as well as the colour.
fn draw_splash(blip: &Blip, s: &Spark) {
    let age = 1.0 - (s.ttl / s.life).clamp(0.0, 1.0);
    let big = s.life > 0.2;
    // Open in the first third, then hold the size and lose the weight.
    let open = (age / 0.3).min(1.0);
    let thin = 1.0 - ((age - 0.45) / 0.55).clamp(0.0, 1.0);
    let reach = if big { 34.0 } else { 22.0 } * (0.45 + 0.55 * open);
    let (rim, fill) = if s.blocked {
        (BlipColor { r: 0.20, g: 0.50, b: 1.0, a: 1.0 }, BlipColor { r: 0.78, g: 0.93, b: 1.0, a: 1.0 })
    } else {
        (BlipColor { r: 1.0, g: 0.48, b: 0.10, a: 1.0 }, BlipColor { r: 1.0, g: 0.93, b: 0.35, a: 1.0 })
    };
    let points = if s.blocked { 4 } else { 8 };
    let turn = s.x * 0.05;
    let c = V(s.x, s.y);
    let spike = |j: usize, len: f32, w: f32, col: BlipColor| {
        let ang = turn + j as f32 * std::f32::consts::TAU / points as f32;
        // Every other point is short, so it is a star and not a wheel.
        let len = len * if j % 2 == 1 && !s.blocked { 0.62 } else { 1.0 };
        stroke(blip, c, V(c.0 + ang.cos() * len, c.1 + ang.sin() * len), w, 0.6, col);
    };
    let w = reach * 0.30 * thin + 0.8;
    for j in 0..points { spike(j, reach + LINE, w + LINE, INK); }
    for j in 0..points { spike(j, reach, w, rim); }
    for j in 0..points { spike(j, reach * 0.66, w * 0.62, fill); }
    blip.fill_circle(c.0, c.1, w * 0.55, BLIP_WHITE);
    // Chips thrown clear of it, between the points.
    for j in 0..points {
        let ang = turn + (j as f32 + 0.5) * std::f32::consts::TAU / points as f32;
        let (dx, dy) = (ang.cos(), ang.sin());
        let (near, far) = (reach * (0.9 + 0.5 * age), reach * (1.05 + 0.75 * age));
        stroke(blip, V(c.0 + dx * near, c.1 + dy * near), V(c.0 + dx * far, c.1 + dy * far),
            1.6 * thin + 0.3, 0.3, fill);
    }
}

/// Lettering over the fight: inked all round and dropped onto its own
/// shadow, because yellow on a sunset is not text, it is a smudge.
fn stamp(blip: &Blip, text: &str, y: f32, sz: f32, c: BlipColor) {
    stamp_at(blip, text, WIN_W as f32 / 2.0, y, sz, c);
}

/// The same, centred on `cx`.
fn stamp_at(blip: &Blip, text: &str, cx: f32, y: f32, sz: f32, c: BlipColor) {
    let x = cx - text_w(text, sz) / 2.0;
    let d = (sz * 0.5).max(1.0);
    blip.draw_text(text, x + d, y + d * 2.5, sz, BlipColor { a: 0.55, ..INK });
    for (dx, dy) in [(-1.0, 0.0), (1.0, 0.0), (0.0, -1.0), (0.0, 1.0),
                     (-1.0, -1.0), (1.0, -1.0), (-1.0, 1.0), (1.0, 1.0)] {
        blip.draw_text(text, x + dx * d, y + dy * d, sz, INK);
    }
    blip.draw_text(text, x, y, sz, c);
}

// ---- HUD -----------------------------------------------------------------

/// Health, rounds won and the clock. Bars drain toward the centre, so who is
/// ahead reads without reading.
fn draw_hud(blip: &Blip, g: &Game) {
    let pad = 16.0;
    // A portrait at the outer end of each bar.
    let face = 30.0;
    let inset = pad + face + 4.0;
    let bar_w = WIN_W as f32 / 2.0 - 36.0 - inset;
    let bar_h = 16.0;
    let y = 22.0;

    for i in 0..2 {
        let f = g.p[i];
        let frac = (f.health as f32 / f.arch().health as f32).clamp(0.0, 1.0);
        let x = if i == 0 { inset } else { WIN_W as f32 - inset - bar_w };
        let a = f.arch();
        let px = if i == 0 { pad } else { WIN_W as f32 - pad - face };
        let py = y - 7.0;
        blip.fill_rect(px - 2.0, py - 2.0, face + 4.0, face + 4.0, INK);
        blip.fill_rect(px, py, face, face, shade(rgb(a.color), 0.45));
        let fw = if i == 0 { 1.0 } else { -1.0 };
        // Wincing while the last blow is still draining off the bar.
        let mood = if f.health == 0 || g.ghost[i] > f.health as f32 + 0.5 { Mood::Hurt } else { Mood::Calm };
        let look = Look { bulk: a.bulk * 0.82, size: 0.82, wind: (0.0, 0.5), ..look_of(&a, 0.0) };
        let head = V(px + face / 2.0 + fw * 2.0, py + 13.0);
        draw_head(blip, &look, fw, head, V(head.0 - fw * 2.0, head.1 + 13.0), mood);
        // The neck runs out of the frame; the name plate squares it off.
        blip.fill_rect(px - 2.0, py + face, face + 4.0, 3.0, INK);
        blip.draw_rect(px, py, face, face, rgb(a.trim));
        blip.fill_rect(x - 2.0, y - 2.0, bar_w + 4.0, bar_h + 4.0,
            BlipColor { r: 0.10, g: 0.10, b: 0.12, a: 1.0 });
        blip.fill_rect(x, y, bar_w, bar_h, BlipColor { r: 0.35, g: 0.06, b: 0.06, a: 1.0 });
        let w = bar_w * frac;
        // Both bars empty away from the centre of the screen.
        let fx = if i == 0 { x + bar_w - w } else { x };
        let c = if frac > 0.45 { BlipColor { r: 0.95, g: 0.85, b: 0.2, a: 1.0 } }
                else if frac > 0.2 { BlipColor { r: 0.95, g: 0.55, b: 0.15, a: 1.0 } }
                else { BlipColor { r: 0.95, g: 0.25, b: 0.2, a: 1.0 } };
        // What the last blow cost, draining after it.
        let lost = bar_w * ((g.ghost[i] / f.arch().health as f32).clamp(0.0, 1.0) - frac).max(0.0);
        let gx = if i == 0 { fx - lost } else { fx + w };
        blip.fill_rect(gx, y, lost, bar_h, BlipColor { r: 1.0, g: 0.92, b: 0.86, a: 1.0 });
        // Nearly out, the bar beats.
        let c = if frac <= 0.2 && (g.now * 6.0) as i32 % 2 == 0 { blend(c, BLIP_WHITE, 0.35) } else { c };
        blip.fill_rect(fx, y, w, bar_h, c);
        // A lit top edge and a shaded foot: a bar of enamel, not a rectangle.
        blip.fill_rect(fx, y, w, 3.0, blend(c, BLIP_WHITE, 0.45));
        blip.fill_rect(fx, y + bar_h - 4.0, w, 4.0, shade(c, 0.78));
        blip.draw_rect(x, y, bar_w, bar_h, BlipColor { r: 0.8, g: 0.8, b: 0.85, a: 0.7 });

        let name = f.arch().name;
        let nx = if i == 0 { inset } else { WIN_W as f32 - inset - text_w(name, 1.0) };
        blip.draw_text(name, nx, y + bar_h + 6.0, 1.0, BLIP_WHITE);

        // Round pips: a fighter needs two, so two lamps say everything.
        for r in 0..ROUNDS_TO_WIN {
            // Inboard of the name, hard against the clock.
            let px = if i == 0 { x + bar_w - 10.0 - r as f32 * 14.0 } else { x + r as f32 * 14.0 };
            let lit = f.rounds > r;
            blip.fill_circle(px + 5.0, y + bar_h + 10.0, 5.5, INK);
            blip.fill_circle(px + 5.0, y + bar_h + 10.0, 4.0,
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

    let stage = STAGE_NAMES[g.stage % STAGES];
    blip.fill_rect(cx - text_w(stage, 1.0) / 2.0 - 5.0, y + bar_h + 11.0,
        text_w(stage, 1.0) + 10.0, 12.0, BlipColor { r: 0.08, g: 0.08, b: 0.10, a: 0.75 });
    blip.draw_text(stage, cx - text_w(stage, 1.0) / 2.0, y + bar_h + 14.0, 1.0,
        BlipColor { r: 0.92, g: 0.92, b: 0.96, a: 1.0 });
}

fn draw_banner(blip: &Blip, g: &Game) {
    let cy = 150.0;
    match g.state {
        State::RoundIntro => {
            // One round each: this one decides it.
            let last = g.p.iter().all(|f| f.rounds == ROUNDS_TO_WIN - 1);
            let r = if last { "FINAL ROUND".to_string() } else { format!("ROUND {}", g.round) };
            stamp(blip, &r, cy, 4.0, BLIP_YELLOW);
            // How far up the ladder this is.
            if g.mode == Mode::Solo {
                let rung = format!("FIGHT {} OF {}", g.opponent_index + 1, RUNGS);
                stamp(blip, &rung, cy - 28.0, 2.0, BlipColor { r: 0.92, g: 0.92, b: 0.96, a: 1.0 });
            }
            let left = g.phase.remaining();
            if left < 0.7 {
                // It lands big and settles.
                let sz = if left > 0.6 { 7.0 } else { 5.0 };
                stamp(blip, "FIGHT!", cy + 46.0 - (sz - 5.0) * 4.0, sz, BLIP_WHITE);
            }
        }
        State::RoundEnd => {
            stamp(blip, g.banner, cy, 5.0, BLIP_YELLOW);
            // The verdict lands once the slow motion is over.
            if !g.call.is_empty() && g.slow <= 0.0 {
                stamp(blip, g.call, cy + 52.0, 4.0, BLIP_WHITE);
            }
        }
        State::MatchEnd => stamp(blip, g.banner, cy, 4.0, BLIP_YELLOW),
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
            stamp(blip, "GAME OVER", cy, 5.0, BlipColor { r: 0.95, g: 0.3, b: 0.3, a: 1.0 });
            let s = format!("SCORE {}", g.sess.score);
            stamp(blip, &s, cy + 50.0, 2.0, BLIP_WHITE);
            if !g.phase.active() { blip.draw_centered("PRESS FIRE", cy + 86.0, 2.0, BLIP_YELLOW); }
        }
        State::Won => {
            stamp(blip, "CHAMPION", cy, 5.0, BLIP_YELLOW);
            let s = format!("SCORE {}", g.sess.score);
            stamp(blip, &s, cy + 50.0, 2.0, BLIP_WHITE);
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

/// How long each pair of fighters holds the title screen.
const TITLE_TURN: f32 = 4.0;

/// One of the title's two fighters, `u` seconds into its turn: stands in,
/// shows a move, and takes the win before the next pair arrives.
fn title_fighter(who: usize, x: f32, facing: f32, u: f32, first: bool) -> Fighter {
    let mut f = Fighter::new(who, x, facing);
    // The left one kicks, then the right one answers with their special.
    let (mv, at) = if first { (MoveId::HighKick, 0.7) } else { (MoveId::Special, 1.7) };
    let m = move_data(mv);
    let len = (m.startup + m.active + m.recovery) * F;
    if u >= at && u < at + len {
        f.act = Act::Attack;
        f.mv = mv;
        f.t = u - at;
    } else if u >= 2.9 {
        f.act = Act::Victory;
        f.t = u - 2.9;
    }
    f
}

/// The title screen: the sign, the whole roster under it, and two fighters at
/// a time showing what they do. It still says only what is needed to start.
fn draw_title(blip: &Blip, g: &Game) {
    let now = g.now;
    // A night stage, knocked back so the sign is the brightest thing on it.
    draw_temple(blip, 0.0, now, 0.0);
    draw_floor(blip, 1, 0.0);
    blip.fill_rect(0.0, 0.0, WIN_W as f32, FLOOR_Y, BlipColor { r: 0.02, g: 0.01, b: 0.08, a: 0.70 });

    // Rays turning slowly behind the sign, the way a fight poster has them.
    let sign = BlipColor { r: 1.0, g: 0.16, b: 0.38, a: 1.0 };
    let (cx, cy) = (WIN_W as f32 / 2.0, 62.0);
    for j in 0..14 {
        let ang = j as f32 * std::f32::consts::TAU / 14.0 + now * 0.12;
        for k in -2..=2 {
            let a = ang + k as f32 * 0.035;
            blip.draw_line_ex(cx + a.cos() * 60.0, cy + a.sin() * 26.0,
                cx + a.cos() * 520.0, cy + a.sin() * 520.0, 9.0, BlipColor { a: 0.05, ..sign });
        }
    }

    // A new pair every few seconds, so the title shows the whole roster. They
    // arrive in a flash of white.
    let n = FIGHTERS.len();
    let turn = (now / TITLE_TURN) as usize;
    let u = now % TITLE_TURN;
    let pair = [turn % n, (turn + n / 2) % n];
    for (i, (x, face)) in [(66.0f32, 1.0f32), (574.0, -1.0)].into_iter().enumerate() {
        let f = title_fighter(pair[i], x, face, u, i == 0);
        draw_body(blip, &f, now, 0.0, 0.0, i, true);
        if u < 0.12 {
            blip.fill_glow_circle(x, FLOOR_Y - f.height() * 0.5, f.height() * 0.7,
                BlipColor { a: 0.5 * (1.0 - u / 0.12), ..BLIP_WHITE });
        }
    }

    let glow = tube(now, 0.0);
    for (r, a) in [(150.0f32, 0.045f32), (96.0, 0.05)] {
        blip.fill_glow_circle(cx, cy + 6.0, r, BlipColor { a: a * glow, ..sign });
    }
    neon(blip, "BRAWLER", 22.0, 9.0, sign, glow);
    neon(blip, "TEN FIGHTERS   ONE CHAMPION", 100.0, 2.0,
        BlipColor { r: 0.35, g: 0.95, b: 1.0, a: 1.0 }, tube(now, 2.1));

    // The roster, one face each, with the pair on stage lit.
    let gap = 38.0;
    let x0 = cx - gap * (n as f32 - 1.0) / 2.0;
    for (i, a) in FIGHTERS.iter().enumerate() {
        let head = V(x0 + i as f32 * gap - 2.0, 140.0);
        if pair.contains(&i) {
            blip.fill_glow_circle(head.0 + 2.0, head.1, 21.0, rgba(a.trim, 0.55));
        }
        let look = Look { bulk: a.bulk.min(1.15), size: 1.0, ..look_of(a, 0.0) };
        draw_head(blip, &look, 1.0, head, V(head.0 - 3.0, head.1 + 15.0), Mood::Calm);
    }

    // The mode, chosen on a stick and one button because that is all a
    // cabinet has. The line you are on is lit; the other is glass.
    let dark = BlipColor { r: 0.42, g: 0.30, b: 0.46, a: 1.0 };
    for (i, label) in ["1 PLAYER", "2 PLAYERS"].iter().enumerate() {
        let y = 176.0 + i as f32 * 32.0;
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
        let y = 250.0 + i as f32 * 22.0;
        // Player two greys out on the one-player line: the row is still
        // there to say the mode exists, without claiming those keys do
        // anything in the game you are about to start.
        let live = *one || g.menu == 1;
        let tint = if live { 1.0 } else { 0.45 };
        let col = if *one { p1c } else { p2c };
        blip.draw_text(tag, x, y, 2.0, BlipColor { a: tint, ..col });
        blip.draw_text(body, x + 48.0, y, 2.0, BlipColor { a: tint, ..dim });
    }
    blip.draw_centered("TOWARD AND BUTTON HITS HIGH   BOTH TOGETHER SPECIAL", 300.0, 1.0, dim);

    // The prompt, on the boards and blinking: the one line that has to be
    // found.
    if (now * 2.2).fract() < 0.7 {
        neon(blip, "PRESS PUNCH TO START", 348.0, 3.0,
            BlipColor { r: 1.0, g: 0.85, b: 0.25, a: 1.0 }, 1.0);
    }
    blip.draw_centered("W S CHOOSE     F OR SPACE START", 382.0, 1.0, dim);
}

/// The select screen, laid out like the cabinets': a strip of faces along
/// the bottom, and above it whoever each cursor is on, standing full height
/// beside their numbers.
fn draw_select(blip: &Blip, g: &Game) {
    let versus = g.mode == Mode::Versus;
    // Behind them, dimmed, the home of whoever the cursor is on (in versus,
    // of player two: where that match will be), raised to the preview's floor.
    let ground = 236.0;
    let home = home_of(if versus { g.pick2 } else { g.pick });
    draw_backdrop(blip, home, ground - FLOOR_Y, g.now, 0.0);
    draw_floor(blip, home, ground - FLOOR_Y);
    blip.fill_rect(0.0, 0.0, WIN_W as f32, WIN_H as f32, BlipColor { r: 0.02, g: 0.02, b: 0.05, a: 0.48 });
    blip.fill_rect(0.0, 250.0, WIN_W as f32, WIN_H as f32 - 250.0, BlipColor { r: 0.03, g: 0.03, b: 0.06, a: 1.0 });
    stamp(blip, if versus { "CHOOSE YOUR FIGHTERS" } else { "CHOOSE YOUR FIGHTER" },
        20.0, 3.0, BLIP_YELLOW);
    let p1c = BlipColor { r: 1.0, g: 0.35, b: 0.35, a: 1.0 };
    let p2c = BlipColor { r: 0.40, g: 0.72, b: 1.0, a: 1.0 };
    let grey = BlipColor { r: 0.74, g: 0.77, b: 0.84, a: 1.0 };

    // The chosen fighters, drawn by the same code as in a match, so the
    // screen shows what you will control: player one on the left facing
    // right, player two mirrored.
    let sides: &[(usize, f32, f32, f32, BlipColor)] = if versus {
        &[(0, 84.0, 1.0, 164.0, p1c), (1, 556.0, -1.0, 356.0, p2c)]
    } else {
        &[(0, 214.0, 1.0, 306.0, BLIP_WHITE)]
    };
    for &(side, x, facing, tx, col) in sides {
        let who = g.picked(side);
        let a = FIGHTERS[who];
        let mut f = Fighter::new(who, x, facing);
        // Locked in, they celebrate; until then they stand in their guard.
        if g.locked[side] { f.act = Act::Victory; f.t = 1.0; }
        draw_body(blip, &f, g.now, ground - FLOOR_Y, 0.0, side, true);

        if versus {
            let lit = if g.locked[side] { 1.0 } else { 0.45 + 0.55 * (g.now * 5.0).sin().max(0.0) };
            let tag = if g.locked[side] { ["P1 READY", "P2 READY"][side] } else { ["P1", "P2"][side] };
            blip.draw_text(tag, tx, 112.0, 2.0, BlipColor { a: lit, ..col });
        }
        // A plate under the words: the stage behind them is not black.
        blip.fill_rect(tx - 8.0, 128.0, 142.0, 108.0, BlipColor { r: 0.02, g: 0.02, b: 0.05, a: 0.72 });
        blip.draw_text(a.name, tx, 136.0, 3.0, rgb(a.trim));
        let special = if a.build == Build::Turtle { format!("TURTLE CALL, {}", a.special_name) }
                      else { a.special_name.to_string() };
        blip.draw_text(&special, tx, 164.0, 1.0, grey);
        blip.draw_text(STAGE_NAMES[home_of(who)], tx, 174.0, 1.0, BlipColor { r: 0.55, g: 0.58, b: 0.66, a: 1.0 });
        // The three numbers that actually differ, as bars: a player choosing
        // between fighters needs the trade, not a biography. Full is the best
        // of the fair fighters; the invincible one runs off the end of all
        // three.
        let stats = [("PWR", a.power / 1.45), ("SPD", a.walk / 250.0),
                     ("HP ", if a.invincible { 2.0 } else { a.health as f32 / 130.0 })];
        for (r, (label, v)) in stats.iter().enumerate() {
            let sy = 190.0 + r as f32 * 14.0;
            blip.draw_text(label, tx, sy, 1.0, grey);
            blip.fill_rect(tx + 28.0, sy, 90.0 * v.clamp(0.0, 1.25), 8.0, rgb(a.trim));
            blip.draw_rect(tx + 28.0, sy, 90.0, 8.0, BlipColor { r: 0.3, g: 0.3, b: 0.36, a: 1.0 });
        }
    }

    // The roster: one face each.
    let n = FIGHTERS.len() as f32;
    let (cw, ch, gap) = (58.0, 58.0, 4.0);
    let x0 = (WIN_W as f32 - n * cw - (n - 1.0) * gap) / 2.0;
    let y0 = 258.0;
    for (i, a) in FIGHTERS.iter().enumerate() {
        let x = x0 + i as f32 * (cw + gap);
        let on1 = g.pick == i;
        let on2 = versus && g.pick2 == i;
        blip.fill_rect(x, y0, cw, ch, BlipColor { r: 0.12, g: 0.12, b: 0.17, a: 1.0 });
        let cx = x + cw / 2.0;
        // Faces all one size here, whatever the fighter's.
        let look = Look { bulk: a.bulk, size: 1.0, ..look_of(a, 0.0) };
        let head = V(cx - 2.0, y0 + 24.0);
        draw_head(blip, &look, 1.0, head, V(head.0 - 3.0, head.1 + 16.0), Mood::Calm);
        blip.fill_rect(x, y0 + ch - 14.0, cw, 14.0, BlipColor { r: 0.07, g: 0.07, b: 0.10, a: 1.0 });
        blip.draw_text(a.name, cx - text_w(a.name, 1.0) / 2.0, y0 + ch - 10.0, 1.0,
            if on1 || on2 { BLIP_WHITE } else { BlipColor { r: 0.6, g: 0.6, b: 0.66, a: 1.0 } });
        let frame = if on1 && on2 { BLIP_WHITE }
            else if on1 { if versus { p1c } else { rgb(a.trim) } }
            else if on2 { p2c }
            else { BlipColor { r: 0.3, g: 0.3, b: 0.35, a: 1.0 } };
        blip.draw_rect(x, y0, cw, ch, frame);
        if on1 || on2 {
            blip.draw_rect(x - 1.0, y0 - 1.0, cw + 2.0, ch + 2.0, frame);
            blip.draw_rect(x - 2.0, y0 - 2.0, cw + 4.0, ch + 4.0, frame);
        }
    }

    // The rules a player cannot find by pressing buttons, put where there is
    // nothing else to do but read them.
    blip.draw_centered("PUNCH AND KICK TOGETHER IS THE SPECIAL", 326.0, 1.0, grey);
    blip.draw_centered("HOLD AWAY TO GUARD   WALK IN CLOSE AND PUNCH TO THROW", 340.0, 1.0, grey);
    blip.draw_centered("TAP TOWARD AS A BLOW LANDS TO TURN IT ASIDE", 354.0, 1.0, grey);
    let prompt = if !versus { "PRESS FIRE TO START" }
        else if !g.locked[0] || !g.locked[1] { "PRESS TO LOCK IN" }
        else { "FIGHT" };
    blip.draw_centered(prompt, 374.0, 2.0, BLIP_WHITE);
}

/// The kiosk card's picture (see brawler_card.sh): the first fighter of the
/// roster in a flying kick, out of a burst in his own red. Drawn small and
/// enlarged by the script, so it keeps the game's pixels.
fn draw_poster(blip: &Blip, g: &Game) {
    let c = |r: f32, g: f32, b: f32| BlipColor { r, g, b, a: 1.0 };
    let (w, h) = (WIN_W as f32, WIN_H as f32);
    blip.fill_rect(0.0, 0.0, w, h, c(0.30, 0.05, 0.07));
    // The burst comes from behind his back foot, where the kick started.
    let from = V(250.0, 250.0);
    let rays = 18;
    for j in 0..rays {
        let ang = 0.12 + j as f32 * std::f32::consts::TAU / rays as f32;
        let col = if j % 2 == 0 { c(0.80, 0.15, 0.12) } else { c(0.56, 0.08, 0.10) };
        let mut d = 18.0;
        while d < 620.0 {
            let r = d * 0.085;
            blip.fill_circle(from.0 + ang.cos() * d, from.1 + ang.sin() * d, r, col);
            d += r * 0.7;
        }
    }
    blip.fill_circle(from.0, from.1, 30.0, c(1.0, 0.86, 0.40));
    blip.fill_circle(from.0, from.1, 18.0, BLIP_WHITE);

    let mut f = Fighter::new(0, 300.0, 1.0);
    f.y = FLOOR_Y - 96.0;
    f.act = Act::Attack;
    f.shown = Act::Attack;
    f.mv = MoveId::FlyingKick;
    let m = move_data(MoveId::FlyingKick);
    f.t = (m.startup + m.active * 0.5) * F;
    draw_body(blip, &f, g.now, 0.0, 0.0, 0, false);
    // Where the heel is going.
    let hit = Spark { x: f.x + 96.0, y: f.y - 44.0, ttl: 0.12, life: 0.26, blocked: false };
    draw_splash(blip, &hit);
}

/// The billing before a match: the two fighters, where they meet, and in a
/// solo run the whole ladder with the ones already beaten struck out.
fn draw_vs(blip: &Blip, g: &Game) {
    let (w, h) = (WIN_W as f32, WIN_H as f32);
    let age = VS_SECS - g.phase.remaining();
    let who = [g.p[0].arch(), g.p[1].arch()];
    let grey = BlipColor { r: 0.92, g: 0.92, b: 0.96, a: 1.0 };

    // Each side leans to its fighter's colour, the left warm and the right
    // cold, so two of a kind still stand on different ground.
    let warm = BlipColor { r: 0.75, g: 0.16, b: 0.12, a: 1.0 };
    let cold = BlipColor { r: 0.12, g: 0.26, b: 0.70, a: 1.0 };
    let tint = [blend(rgb(who[0].trim), warm, 0.6), blend(rgb(who[1].trim), cold, 0.6)];
    let slant = 0.24;
    let edge = |y: f32| w / 2.0 + (h / 2.0 - y) * slant;
    for row in 0..(h as i32 / 2) {
        let y = row as f32 * 2.0;
        let k = 0.22 + 0.26 * y / h;
        blip.fill_rect(0.0, y, edge(y), 2.0, shade(tint[0], k));
        blip.fill_rect(edge(y), y, w - edge(y), 2.0, shade(tint[1], k));
    }
    stroke(blip, V(edge(0.0), 0.0), V(edge(h), h), 5.0, 5.0, INK);
    stroke(blip, V(edge(0.0), 0.0), V(edge(h), h), 1.5, 1.5, BLIP_WHITE);

    // A portrait each, sliding in from the wings, and the fighters themselves
    // squaring up under the letters.
    let off = (1.0 - (age / 0.35).min(1.0)).powi(2) * 260.0;
    let (pw, py) = (128.0, 44.0);
    let ground = 302.0;
    for i in 0..2 {
        let side = if i == 0 { -1.0 } else { 1.0 };
        let a = &who[i];
        let cx = w / 2.0 + side * (218.0 + off);
        blip.fill_rect(cx - pw / 2.0 - 4.0, py - 4.0, pw + 8.0, pw + 8.0, INK);
        blip.fill_rect(cx - pw / 2.0, py, pw, pw, shade(tint[i], 0.75));
        // No wind in the frame: a headband's tails would fly out of it.
        let look = Look { bulk: a.bulk * 3.0, size: 3.0, wind: (0.0, 1.0), ..look_of(a, 0.0) };
        let head = V(cx - side * 8.0, py + 50.0);
        let neck = V(head.0 + side * 8.0, py + pw - 22.0);
        // A bust: shoulders across the foot of the frame, under the head.
        let (l, r) = (V(cx - 34.0, py + pw + 4.0), V(cx + 34.0, py + pw + 4.0));
        let bare = matches!(a.build, Build::Bare | Build::Giant | Build::Turtle);
        stroke(blip, l, r, 30.0 + LINE, 30.0 + LINE, INK);
        stroke(blip, l, r, 30.0, 30.0, look.c(if bare { look.skin } else { look.cloth }));
        draw_head(blip, &look, -side, head, neck, Mood::Calm);
        // The shoulders run out of the frame; the sill squares them off.
        blip.fill_rect(cx - pw / 2.0 - 4.0, py + pw, pw + 8.0, 34.0, INK);
        blip.draw_rect(cx - pw / 2.0, py, pw, pw, rgb(a.trim));
        blip.draw_rect(cx - pw / 2.0 - 1.0, py - 1.0, pw + 2.0, pw + 2.0, rgb(a.trim));
        if g.mode == Mode::Versus {
            let label = if i == 0 { "PLAYER 1" } else { "PLAYER 2" };
            blip.draw_text(label, cx - text_w(label, 1.0) / 2.0, py - 16.0, 1.0, grey);
        }
        blip.draw_text(a.name, cx - text_w(a.name, 3.0) / 2.0, py + pw + 6.0, 3.0, BLIP_WHITE);

        let f = Fighter::new(g.p[i].who, w / 2.0 + side * (64.0 + off), -side);
        draw_body(blip, &f, g.now, ground - FLOOR_Y, 0.0, i, true);
    }

    // It lands big and settles.
    if age > 0.35 {
        let sz = if age < 0.45 { 11.0 } else { 8.0 };
        stamp_at(blip, "VS", w / 2.0, 62.0 - (sz - 8.0) * 4.0, sz, BLIP_YELLOW);
    }
    stamp(blip, STAGE_NAMES[g.stage % STAGES], 14.0, 2.0, grey);
    if g.mode != Mode::Solo { return; }

    // The ladder: everyone still to come, and a cross through the beaten.
    let (cw, gap) = (40.0, 6.0);
    let x0 = (w - RUNGS as f32 * cw - (RUNGS as f32 - 1.0) * gap) / 2.0;
    let y0 = 342.0;
    for (rung, foe) in g.ladder().into_iter().enumerate() {
        let a = &FIGHTERS[foe];
        let x = x0 + rung as f32 * (cw + gap);
        let (beaten, next) = (rung < g.opponent_index, rung == g.opponent_index);
        blip.fill_rect(x - 2.0, y0 - 2.0, cw + 4.0, cw + 4.0, INK);
        blip.fill_rect(x, y0, cw, cw, shade(rgb(a.color), 0.4));
        // No wind in the frame: a headband's tails would fly out of it.
        let look = Look { bulk: a.bulk * 0.9, size: 0.9, wind: (0.0, 1.0), ..look_of(a, 0.0) };
        let head = V(x + cw / 2.0 - 1.0, y0 + 18.0);
        draw_head(blip, &look, -1.0, head, V(head.0 + 3.0, head.1 + 15.0),
            if beaten { Mood::Hurt } else { Mood::Calm });
        blip.fill_rect(x - 2.0, y0 + cw, cw + 4.0, 4.0, INK);
        if beaten {
            blip.fill_rect(x, y0, cw, cw, BlipColor { a: 0.55, ..INK });
            let red = BlipColor { r: 0.95, g: 0.25, b: 0.2, a: 1.0 };
            stroke(blip, V(x + 7.0, y0 + 7.0), V(x + cw - 7.0, y0 + cw - 7.0), 2.2, 2.2, red);
            stroke(blip, V(x + cw - 7.0, y0 + 7.0), V(x + 7.0, y0 + cw - 7.0), 2.2, 2.2, red);
        }
        let lit = next && (g.now * 5.0) as i32 % 2 == 0;
        let frame = if lit { BLIP_YELLOW } else if next { BLIP_WHITE }
            else { BlipColor { r: 0.3, g: 0.3, b: 0.35, a: 1.0 } };
        blip.draw_rect(x, y0, cw, cw, frame);
        if next { blip.draw_rect(x - 1.0, y0 - 1.0, cw + 2.0, cw + 2.0, frame); }
    }
    let rung = format!("FIGHT {} OF {}", g.opponent_index + 1, RUNGS);
    stamp(blip, &rung, y0 - 24.0, 2.0, BLIP_YELLOW);
}

// ---- pose gallery (development only) -------------------------------------
// `cargo build -p brawler --features gallery` replaces the game with a
// looping contact sheet of every pose, since reaching a sweep in play takes a
// dozen inputs and shows it for four frames.
#[cfg(feature = "gallery")]
pub fn draw_gallery(blip: &Blip, now: f32) {
    blip.clear(BlipColor { r: 0.15, g: 0.16, b: 0.21, a: 1.0 });
    let acts: [(&str, Act, MoveId); 22] = [
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
        ("FLYING", Act::Air, MoveId::LowPunch),
        ("LOW GUARD", Act::Block, MoveId::LowPunch),
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
    let who = WHO.load(Ordering::Relaxed) % FIGHTERS.len();
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
        if label == "LOW GUARD" { f.crouch_block = true; }
        if act == Act::Air {
            f.y = FLOOR_Y - 70.0;
            f.soar = k as f32 / 4.0;
        }
        blip.draw_line(x - 60.0, FLOOR_Y, x + 74.0, FLOOR_Y,
            BlipColor { r: 0.34, g: 0.34, b: 0.44, a: 0.8 });
        // The hitbox this frame, so the picture can be checked against
        // the thing it claims to be a picture of.
        if let Some((hx, hy, hw, hh)) = f.hit_box() {
            blip.fill_rect(hx, hy, hw, hh, BlipColor { r: 1.0, g: 0.3, b: 0.3, a: 0.30 });
        }
        draw_body(blip, &f, now, 0.0, 0.0, k, false);
    }
    blip.draw_centered(label, 22.0, 3.0, BLIP_YELLOW);
    blip.draw_centered(FIGHTERS[who].name, 54.0, 2.0, BLIP_WHITE);
    // The small things, so they can be looked at too: the fruits and the
    // two cameos.
    for kind in 0..FRUIT_POINTS.len() { draw_fruit(blip, kind, 30.0 + kind as f32 * 26.0, 110.0); }
    draw_flyby(blip, 100.0, 5.0 + (now * 0.2).fract());
    draw_chase(blip, 104.0, 0.5 + (now * 0.2).fract(), BlipColor { r: 0.15, g: 0.16, b: 0.21, a: 1.0 });
}
