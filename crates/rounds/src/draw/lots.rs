//! The ground and what stands on each lot: shops, the street furniture, parks, the cemetery, Amaliehaven, the bakery
//! and the houses.

use super::*;

/// The bakery standing up: a cream shop front with its big window and BAGERI over it, under a slate roof.
pub(super) fn draw_shop(blip: &Blip, g: &Game, s: usize, r: usize) {
    let (y0, y1) = (sy((r + 1) as f32 * LOT), sy(r as f32 * LOT));
    let (w, back, ridge) = (mx(s, WALL[0]), mx(s, -30.0), mx(s, 30.0));
    let hh = terrace_height(g, s, r);
    let face = |uy: f32, v: f32| lift(w, uy, v);
    quad((w, y0), (w, y1), face(y1, hh), face(y0, hh), rgb([244, 232, 206]));
    quad(face(y0 + 14.0, 3.0), face(y1 - 14.0, 3.0), face(y1 - 14.0, 18.0), face(y0 + 14.0, 18.0), rgb([255, 214, 140]));
    for i in 0..3 {
        let u = y0 + 24.0 + i as f32 * 26.0;
        quad(face(u, 6.0), face(u + 14.0, 6.0), face(u + 14.0, 10.0), face(u, 10.0), rgb([200, 140, 70]));
    }
    // the golden pretzel, the Danish baker's sign, hung out over the pavement
    let (px, py) = face((y0 + y1) / 2.0, hh - 8.0);
    let px = px + (1.0 - s as f32 * 2.0) * 14.0;
    blip.draw_line_ex(px - (1.0 - s as f32 * 2.0) * 14.0, py, px, py, 2.0, rgb([40, 36, 34]));
    blip.draw_circle(px - 4.0, py + 8.0, 5.0, 3.0, rgb(GOLD));
    blip.draw_circle(px + 4.0, py + 8.0, 5.0, 3.0, rgb(GOLD));
    blip.draw_circle(px, py + 14.0, 4.0, 3.0, rgb(GOLD));
    let (e0, e1, r0, r1) = (face(y0, hh), face(y1, hh), lift(ridge, y0, hh + 18.0), lift(ridge, y1, hh + 18.0));
    quad(lift(back, y0, hh), lift(back, y1, hh), r1, r0, rgb([70, 72, 82]));
    quad(e0, e1, r1, r0, rgb([96, 98, 110]));
}

/// Pavements, bike lanes and the road for one row; a side street or a bridge where the route has one.
pub(super) fn draw_ground(blip: &Blip, g: &Game, r: usize) {
    let y = sy((r + 1) as f32 * LOT);
    let lot = g.houses[0][r].lot;
    let w = WIN_W as f32;
    if lot == Lot::Bridge {
        // the canal under the bridge, the deck and its railings
        blip.fill_rect(0.0, y, w, LOT, rgb(WATER));
        for i in 0..8 {
            let wy = y + scatter(r * 8 + i, 3) * LOT;
            let wx = scatter(r * 8 + i, 4) * w;
            blip.fill_rect(wx, wy, 18.0, 2.0, rgba([200, 230, 240], 0.35));
        }
        // the Lakes' swans, gliding
        let t = blip::now();
        for k in 0..3 {
            let sx = if k == 1 { 30.0 + scatter(r, 61) * 90.0 } else { 350.0 + scatter(r * 3 + k, 62) * 100.0 };
            let syy = y + 20.0 + scatter(r * 3 + k, 63) * 70.0 + (t * 0.5 + k as f32).sin() * 4.0;
            blip.draw_circle(sx, syy + 2.0, 9.0, 1.0, rgba([200, 230, 240], 0.4));
            blip.fill_circle(sx, syy, 6.0, rgb([250, 250, 248]));
            blip.fill_rect(sx - 1.0, syy - 11.0, 3.0, 8.0, rgb([250, 250, 248]));
            blip.fill_circle(sx, syy - 11.0, 2.5, rgb([250, 250, 248]));
            blip.fill_rect(sx + 1.0, syy - 12.0, 3.0, 2.0, rgb([240, 130, 40]));
        }
        // the bridge: granite parapets, wide pavements and the road between
        blip.fill_rect(RAIL[0] - 30.0, y, RAIL[1] - RAIL[0] + 60.0, LOT, rgb([176, 172, 166]));
        blip.fill_rect(RAIL[0], y, RAIL[1] - RAIL[0], LOT, rgb([84, 84, 92]));
        for x in [RAIL[0] - 36.0, RAIL[1] + 30.0] { blip.fill_rect(x, y, 6.0, LOT, rgb([206, 200, 190])); }
        for x in [RAIL[0] - 4.0, RAIL[1]] {
            blip.fill_rect(x, y, 4.0, LOT, rgb(KERB));
        }
        // the bridge's ornate cast-iron lamp posts on the parapets, dark green-black, a cluster of globes on top
        let dark = daylight(g) < 0.45;
        for x in [RAIL[0] - 33.0, RAIL[1] + 33.0] {
            for k in [0.25, 0.75] {
                let yc = y + LOT * k;
                // 20 high: taller, the perspective tips a thin post toward the water as if knocked over
                let top = lift(x, yc, 20.0);
                blip.fill_circle(x, yc, 3.5, rgb([30, 44, 40]));
                blip.draw_line_ex(x, yc, top.0, top.1, 2.5, rgb([34, 50, 46]));
                let mid = lift(x, yc, 8.0);
                blip.fill_circle(mid.0, mid.1, 2.0, rgb([40, 58, 52]));
                for (dx, dy) in [(-4.0, 0.0), (4.0, 0.0), (0.0, -3.0)] {
                    let (gx, gy) = (top.0 + dx, top.1 + dy);
                    if dark { blip.fill_glow_circle(gx, gy, 2.6, rgb([255, 236, 190])); }
                    else { blip.fill_circle(gx, gy, 2.3, rgb([226, 226, 220])); }
                }
                blip.fill_rect(top.0 - 1.0, top.1 - 7.0, 2.0, 4.0, rgb([34, 50, 46]));
            }
        }
        return;
    }
    // pavements: a band of big granite slabs between strips of small cobbles, the Copenhagen way
    for s in 0..2 {
        side_rect(blip, s, FENCE[0], y, CURB[0] - FENCE[0], LOT, rgb([150, 146, 140]));
        side_rect(blip, s, FENCE[0] + 5.0, y, 14.0, LOT, rgb([200, 196, 188]));
        for k in 0..5 { side_rect(blip, s, FENCE[0] + 5.0, y + k as f32 * 22.0 + 4.0, 14.0, 1.0, rgb([160, 156, 150])); }
        for k in 0..18 {
            let cy = y + k as f32 * 6.0 + 1.0;
            side_rect(blip, s, FENCE[0] + 1.0, cy, 3.0, 4.0, rgb([128, 124, 120]));
            side_rect(blip, s, FENCE[0] + 20.0, cy, 3.0, 4.0, rgb([128, 124, 120]));
        }
    }
    // the road: asphalt on Noerrebro's busy street, granite setts in the old town
    blip.fill_rect(CURB[0], y, CURB[1] - CURB[0], LOT, rgb([84, 84, 92]));
    let setts = district(Side::Left, r) != District::Norrebro;
    for row in 0..(if setts { 14 } else { 0 }) {
        let cy = y + row as f32 * 8.0;
        let off = if (r * 14 + row).is_multiple_of(2) { 0.0 } else { 5.0 };
        let mut cx = CURB[0] + 26.0 + off;
        let mut k = 0;
        while cx < CURB[1] - 28.0 {
            let shade = 0.88 + scatter(r * 400 + row * 30 + k, 87) * 0.2;
            blip.fill_rect(cx, cy, 9.0, 6.0, rgb(shade_u(COBBLE, shade)));
            cx += 10.0;
            k += 1;
        }
    }
    // cycle tracks: smooth asphalt a step up from the road behind a granite kerb
    for s in 0..2 {
        side_rect(blip, s, CURB[0], y, 26.0, LOT, rgb(TRACK));
        side_rect(blip, s, CURB[0] + 24.0, y, 4.0, LOT, rgb(KERB));
        side_rect(blip, s, CURB[0] + 28.0, y, 1.0, LOT, rgba([30, 30, 40], 0.5));
        side_rect(blip, s, CURB[0] - 2.0, y, 2.0, LOT, rgb(KERB));
    }
    if lot == Lot::Palace {
        plaza(blip, r, y);
        return;
    }
    if lot == Lot::Cross {
        // a side street across, with zebra crossings either side of it
        blip.fill_rect(0.0, y, CURB[0], LOT, rgb(PAVE));
        blip.fill_rect(CURB[1], y, w - CURB[1], LOT, rgb(PAVE));
        blip.fill_rect(0.0, y + 16.0, w, LOT - 32.0, rgb(ROAD));
        for (yy, hh) in [(y + 16.0, 3.0), (y + LOT - 19.0, 3.0)] { blip.fill_rect(0.0, yy, CURB[0], hh, rgb(STONE)); blip.fill_rect(CURB[1], yy, w - CURB[1], hh, rgb(STONE)); }
        for k in 0..9 { blip.fill_rect(CURB[0] + 4.0 + k as f32 * 20.0, y + 20.0, 10.0, 14.0, rgba([240, 240, 230], 0.85)); }
        for k in 0..9 { blip.fill_rect(CURB[0] + 4.0 + k as f32 * 20.0, y + LOT - 34.0, 10.0, 14.0, rgba([240, 240, 230], 0.85)); }
        for x in [40.0, 100.0, 380.0, 440.0] { blip.fill_rect(x, y + LOT / 2.0 - 1.0, 22.0, 3.0, rgba([240, 220, 120], 0.7)); }
        // a hot dog wagon on the corner, as on every Copenhagen square
        if r == JUNCTIONS[0] { stand(56.0, y + 14.0, Stand::Wagon); }
        // the blue cycle crossings carry the tracks over the side street
        for s in 0..2 {
            side_rect(blip, s, CURB[0], y + 16.0, 26.0, LOT - 32.0, rgb(CROSSING_BLUE));
            for k in 0..4 { side_rect(blip, s, CURB[0] + 2.0, y + 22.0 + k as f32 * 18.0, 22.0, 6.0, rgba([255, 255, 255], 0.85)); }
        }
        // stop lines: the bike's at the near side, the oncoming side's at the far
        blip.fill_rect(CURB[0], y + LOT - 12.0, 240.0 - CURB[0], 3.0, rgba([250, 250, 250], 0.9));
        blip.fill_rect(240.0, y + 9.0, CURB[1] - 240.0, 3.0, rgba([250, 250, 250], 0.9));
        return;
    }
    // the centre line, painted over the setts
    for k in 0..3 { blip.fill_rect(239.0, y + k as f32 * 38.0 + 6.0, 3.0, 20.0, rgba([240, 236, 220], 0.7)); }
    // a bike painted in each lane now and then
    if r % 3 == 1 {
        for cx in BIKE_LANE {
            let cy = y + LOT / 2.0;
            let c = rgba([236, 232, 220], 0.55);
            blip.draw_circle(cx, cy - 6.0, 3.5, 1.0, c);
            blip.draw_circle(cx, cy + 6.0, 3.5, 1.0, c);
            blip.draw_line(cx, cy - 6.0, cx, cy + 6.0, c);
        }
    }
}

/// What stands on the pavement where two lots meet: a street tree or a rack of parked bikes.
pub(super) fn draw_furniture(blip: &Blip, g: &Game, r: usize, y: f32) {
    for s in 0..2 {
        let pick = scatter(r * 2 + s, 91);
        let x = mx(s, FENCE[0] + 12.0);
        if g.houses[s][r].lot == Lot::Bakery { continue; }
        if pick < 0.24 {
            blip.fill_circle(x + 3.0, y + 4.0, 15.0, rgba([20, 40, 20], 0.3));
            blip.fill_circle(x, y, 15.0, rgb([64, 128, 66]));
            blip.fill_circle(x - 4.0, y - 4.0, 8.0, rgb([94, 160, 86]));
            blip.fill_circle(x + 5.0, y + 3.0, 4.0, rgb([120, 176, 96]));
        } else if pick < 0.42 {
            for b in 0..4 {
                let by = y - 20.0 + b as f32 * 12.0;
                let c = [[220, 70, 70], [60, 120, 200], [60, 60, 70]][(r + b) % 3];
                blip.fill_rect(x - 9.0, by, 18.0, 3.0, rgb(c));
                blip.fill_circle(x - 9.0, by + 1.5, 2.5, rgb([40, 40, 46]));
                blip.fill_circle(x + 9.0, by + 1.5, 2.5, rgb([40, 40, 46]));
            }
        }
    }
}

/// How much of the morning's light a house's windows still need: lit early, dark once the sun is up.
pub(super) fn lit(g: &Game, s: usize, r: usize) -> bool {
    daylight(g) < 0.25 + 0.65 * scatter(r * 2 + s, 7) || g.weather() == Weather::Fog || g.rain()
}

pub(super) fn draw_lot(blip: &Blip, g: &Game, s: usize, r: usize, t: f32) {
    let h = g.houses[s][r];
    let y = sy((r + 1) as f32 * LOT);
    let dy = sy(door_d(r));
    match h.lot {
        Lot::Bridge | Lot::Cross => {}
        // the cemetery's wall runs on along the street for two lots
        Lot::Park if (18..=19).contains(&r) => cemetery(blip, s, r, y),
        Lot::Park if r + 2 >= ROWS => amaliehaven(blip, g, s, r, y),
        Lot::Park => draw_park(blip, s, r, y),
        Lot::Bakery => draw_bakery(blip, g, s, r, y, t),
        Lot::Tower | Lot::Church => forecourt(blip, s, y, r),
        Lot::Palace | Lot::Mouth | Lot::Corner => {}
        Lot::House { drive } => draw_house(blip, g, s, r, y, dy, drive, t),
    }
}

pub(super) fn draw_park(blip: &Blip, s: usize, r: usize, y: f32) {
    side_rect(blip, s, 0.0, y, FENCE[0], LOT, rgb([92, 150, 76]));
    // a gravel path, a bench and trees
    side_rect(blip, s, 30.0, y, 12.0, LOT, rgb([206, 190, 150]));
    side_rect(blip, s, 48.0, y + 40.0, 6.0, 26.0, rgb([130, 90, 60]));
    for k in 0..3 {
        let tx = mx(s, 70.0 + scatter(r * 3 + k, 11) * 40.0);
        let ty = y + 18.0 + k as f32 * 36.0;
        blip.fill_circle(tx + 3.0, ty + 4.0, 17.0, rgba([30, 60, 30], 0.35));
        blip.fill_circle(tx, ty, 17.0, rgb([58, 120, 62]));
        blip.fill_circle(tx - 5.0, ty - 5.0, 9.0, rgb([84, 150, 80]));
    }
    for k in 0..6 {
        let fx = mx(s, 8.0 + scatter(r * 6 + k, 13) * 18.0);
        blip.fill_circle(fx, y + scatter(r * 6 + k, 14) * LOT, 2.0, rgb([[250, 220, 90], [240, 120, 150], [250, 250, 250]][k % 3]));
    }
}

/// Assistens Kirkegaard as it meets Nørrebrogade: the long ochre wall with its red tile coping and pillars, and over
/// it the cemetery's rows of tall poplars, broad lindens, box-hedged plots and old stones in the grass.
pub(super) fn cemetery(blip: &Blip, s: usize, r: usize, y: f32) {
    side_rect(blip, s, 0.0, y, FENCE[0], LOT, rgb([78, 128, 68]));
    // a gravel path between the plots
    side_rect(blip, s, 44.0, y, 10.0, LOT, rgb([196, 182, 146]));
    for k in 0..12 {
        let i = r * 12 + k;
        let gx = mx(s, 8.0 + scatter(i, 11) * 34.0 + if scatter(i, 15) > 0.5 { 52.0 } else { 0.0 });
        let gy = y + 8.0 + scatter(i, 12) * (LOT - 16.0);
        match (scatter(i, 16) * 4.0) as i32 {
            // a plot edged in clipped box, its stone at the head
            0 => {
                blip.fill_rect(gx - 6.0, gy - 5.0, 12.0, 10.0, rgb([44, 96, 52]));
                blip.fill_rect(gx - 4.0, gy - 3.0, 8.0, 6.0, rgb([110, 90, 70]));
                blip.fill_rect(gx - 3.0, gy - 6.0, 6.0, 3.0, rgb([176, 174, 166]));
            }
            // a white cross
            1 => {
                blip.fill_rect(gx - 0.5, gy - 5.0, 2.0, 9.0, rgb([232, 230, 222]));
                blip.fill_rect(gx - 2.5, gy - 3.0, 6.0, 2.0, rgb([232, 230, 222]));
            }
            // a dark polished headstone
            2 => {
                blip.fill_rect(gx - 3.0, gy - 2.0, 6.0, 4.0, rgb([46, 46, 52]));
                blip.fill_rect(gx - 3.0, gy - 2.0, 6.0, 1.0, rgb([96, 96, 104]));
            }
            // an old grey slab, weathered
            _ => {
                blip.fill_rect(gx - 3.0, gy - 2.0, 7.0, 5.0, rgb([164, 162, 152]));
                blip.fill_rect(gx - 3.0, gy - 2.0, 7.0, 1.0, rgb([196, 194, 186]));
            }
        }
    }
    // tall poplars in a row along the path, and a broad linden leaning over the wall
    for k in 0..3 {
        let (tx, ty) = (mx(s, 60.0), y + 18.0 + k as f32 * 37.0);
        blip.fill_rect(tx - 4.0 + 6.0, ty - 14.0 + 10.0, 9.0, 30.0, rgba([20, 40, 20], 0.30));
        for (dy, rad, c) in [(10.0, 6.0, [40, 88, 46]), (2.0, 7.0, [48, 100, 52]), (-6.0, 6.5, [56, 112, 58]), (-13.0, 5.0, [70, 128, 66])] {
            blip.fill_circle(tx, ty + dy, rad, rgb(c));
        }
    }
    let (lx, ly) = (mx(s, 96.0), y + 30.0 + scatter(r, 17) * 50.0);
    blip.fill_circle(lx + 4.0, ly + 5.0, 21.0, rgba([20, 40, 20], 0.32));
    blip.fill_circle(lx, ly, 21.0, rgb([50, 104, 56]));
    blip.fill_circle(lx - 6.0, ly - 6.0, 12.0, rgb([72, 132, 70]));
    blip.fill_circle(lx + 7.0, ly + 5.0, 8.0, rgb([42, 92, 48]));
    // the wall: an ochre face with its pillars, the coping of red tiles, the shade it throws inside
    side_rect(blip, s, FENCE[0] - 22.0, y, 10.0, LOT, rgba([20, 40, 20], 0.25));
    side_rect(blip, s, FENCE[0] - 12.0, y, 12.0, LOT, rgb([214, 166, 82]));
    let mut py = y + 6.0;
    while py < y + LOT {
        side_rect(blip, s, FENCE[0] - 12.0, py, 12.0, 3.0, rgb([196, 148, 70]));
        py += 22.0;
    }
    side_rect(blip, s, FENCE[0] - 14.0, y, 5.0, LOT, rgb([166, 66, 44]));
    let mut ty2 = y;
    while ty2 < y + LOT {
        side_rect(blip, s, FENCE[0] - 14.0, ty2, 5.0, 1.0, rgb([120, 44, 30]));
        ty2 += 3.0;
    }
}

/// Amaliehaven at the end of the route: lawns and box hedges down to the harbour, and across the water the Opera
/// under its flat roof.
pub(super) fn amaliehaven(blip: &Blip, g: &Game, s: usize, r: usize, y: f32) {
    side_rect(blip, s, 0.0, y, FENCE[0], LOT, rgb([104, 160, 86]));
    for k in 0..3 { side_rect(blip, s, 14.0 + k as f32 * 36.0, y + 10.0, 24.0, LOT - 20.0, rgb([84, 140, 70])); }
    if s == 1 && r + 1 == ROWS {
        // the harbour, and the Opera across it on Holmen
        let top = y - 30.0;
        blip.fill_rect(0.0, top - 200.0, WIN_W as f32, 200.0, rgb([70, 112, 134]));
        let base = top - 60.0;
        blip.fill_rect(70.0, base - 40.0, 340.0, 40.0, rgb([40, 46, 56]));
        blip.fill_rect(150.0, base - 36.0, 180.0, 32.0, rgba([255, 214, 140], if daylight(g) < 0.5 { 0.8 } else { 0.35 }));
        for i in 0..9 { blip.fill_rect(150.0 + i as f32 * 20.0, base - 36.0, 2.0, 32.0, rgb([60, 66, 76])); }
        blip.fill_rect(40.0, base - 52.0, 400.0, 10.0, rgb([214, 214, 210]));
        blip.fill_rect(40.0, base - 44.0, 400.0, 3.0, rgb([170, 170, 168]));
        blip.fill_rect(60.0, base, 360.0, 6.0, rgb([150, 146, 140]));
    }
}

pub(super) fn draw_bakery(blip: &Blip, g: &Game, s: usize, r: usize, y: f32, t: f32) {
    // the bakery's forecourt, a striped awning over it and the golden pretzel sign
    side_rect(blip, s, WALL[0], y, FENCE[0] - WALL[0] + 4.0, LOT, rgb(PAVE));
    for k in 0..7 {
        let c = if k % 2 == 0 { [214, 64, 70] } else { [250, 244, 236] };
        side_rect(blip, s, WALL[0], y + 6.0 + k as f32 * 14.0, 30.0, 14.0, rgb(c));
    }
    let (px, py) = (mx(s, WALL[0] + 38.0), y + LOT / 2.0);
    blip.draw_circle(px - 4.0, py, 6.0, 3.0, rgb([196, 128, 60]));
    blip.draw_circle(px + 4.0, py, 6.0, 3.0, rgb([196, 128, 60]));
    // warm air from the ovens
    for k in 0..3 {
        let a = (t * 0.6 + k as f32 / 3.0).fract();
        blip.fill_circle(mx(s, 30.0 + k as f32 * 8.0) + (a * 9.0).sin() * 3.0, y + 30.0 - a * 26.0, 4.0 + a * 4.0,
            rgba([255, 255, 255], 0.35 * (1.0 - a)));
    }
    let _ = (g, r);
}

#[allow(clippy::too_many_arguments)]
pub(super) fn draw_house(blip: &Blip, g: &Game, s: usize, r: usize, y: f32, dy: f32, drive: bool, t: f32) {
    let h = g.houses[s][r];
    let k = r * 2 + s;
    if district(side_of(s), r) == District::Nyboder {
        // a little front garden behind a low hedge, the gate gap at the path
        side_rect(blip, s, WALL[0], y, FENCE[0] - WALL[0], LOT, rgb([116, 172, 92]));
        for f in 0..2 {
            let fx = mx(s, WALL[0] + 6.0 + scatter(k * 5 + f, 21) * 30.0);
            let fy = y + 6.0 + scatter(k * 5 + f, 22) * (LOT - 12.0);
            if (fy - dy).abs() < 12.0 { continue; }
            blip.fill_circle(fx, fy, 2.5, rgb([[250, 200, 80], [236, 110, 130], [250, 250, 250], [190, 120, 220]][f % 4]));
        }
        side_rect(blip, s, WALL[0], dy - 7.0, FENCE[0] - WALL[0], 14.0, rgb([214, 200, 172]));
        let hedge = rgb([62, 118, 64]);
        side_rect(blip, s, FENCE[0] - 4.0, y, 6.0, dy - 10.0 - y, hedge);
        side_rect(blip, s, FENCE[0] - 4.0, dy + 10.0, 6.0, y + LOT - dy - 10.0, hedge);
    } else {
        // a paved forecourt behind a black iron railing: bikes against the wall, the bins
        side_rect(blip, s, WALL[0], y, FENCE[0] - WALL[0], LOT, rgb([178, 174, 168]));

        let iron = rgb([36, 36, 42]);
        side_rect(blip, s, FENCE[0] - 2.0, y, 2.0, dy - 10.0 - y, iron);
        side_rect(blip, s, FENCE[0] - 2.0, dy + 10.0, 2.0, y + LOT - dy - 10.0, iron);
        let mut py = y + 4.0;
        while py < y + LOT { if (py - dy).abs() > 10.0 { side_rect(blip, s, FENCE[0] - 4.0, py, 6.0, 2.0, iron); } py += 8.0; }
    }
    if drive {
        side_rect(blip, s, WALL[0], dy + 12.0, FENCE[0] - WALL[0] + 2.0, 40.0, rgb([168, 162, 156]));
        side_rect(blip, s, FENCE[0] - 4.0, dy + 12.0, 6.0, 40.0, rgb([168, 162, 156]));
    }
    // the gate, rattling when a dog is behind it
    let rattle = g.hazards.iter().any(|z| z.kind == Hz::Dog && z.phase == TELL && (z.d - door_d(r) + 30.0).abs() < 1.0 && z.side.index() == s);
    let jig = if rattle { (t * 60.0).sin() * 2.0 } else { 0.0 };
    side_rect(blip, s, FENCE[0] - 3.0, dy - 10.0 + jig, 3.0, 20.0, rgb([236, 230, 214]));
    // the doorstep
    side_rect(blip, s, WALL[0], dy - DOOR_HALF, STEP, DOOR_HALF * 2.0, rgb(STONE));
    side_rect(blip, s, WALL[0] + STEP - 2.0, dy - DOOR_HALF, 2.0, DOOR_HALF * 2.0, rgb([186, 176, 160]));
    // the neighbour on the step, waving
    if h.done && h.wave < 1.6 {
        let wave = (h.wave * 10.0).sin() * 4.0;
        let (nx, ny) = (mx(s, WALL[0] + 8.0), dy);
        blip.fill_circle(nx, ny, 6.0, rgb([[200, 90, 90], [90, 120, 200], [230, 200, 90]][r % 3]));
        blip.fill_circle(nx, ny - 1.0, 4.0, rgb([240, 200, 170]));
        blip.draw_line_ex(nx, ny, nx + (s as f32 * 2.0 - 1.0) * -4.0, ny - 10.0 + wave, 2.0, rgb([240, 200, 170]));
    }
    if !h.sub {
        // the sample box by the gate: blue, with a star, until one is in it
        let (bx, by) = (mx(s, BOX_X[0]), sy(door_d(r) + BOX_D));
        blip.fill_rect(bx - 6.0, by - 6.0, 12.0, 12.0, rgb(if h.sampled { [120, 200, 140] } else { [70, 140, 220] }));
        blip.draw_rect(bx - 6.0, by - 6.0, 12.0, 12.0, rgb([30, 50, 90]));
        if h.sampled {
            blip.fill_rect(bx - 4.0, by - 2.0, 8.0, 4.0, rgb([250, 248, 240]));
        } else {
            let pulse = 0.6 + 0.4 * (t * 4.0 + r as f32).sin();
            blip.fill_circle(bx, by, 2.5, rgba([255, 255, 220], pulse));
        }
    }
    if r % 4 == 2 && (g.wind() != 0.0 || s == r / 4 % 2) { draw_flag(blip, g, s, dy - 34.0, t); }
}
