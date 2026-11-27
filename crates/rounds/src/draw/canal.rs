//! The Canal Run at the end of each day: Nyhavn's quay, its tables, ships and boats, and Vor Frelsers spire.

use super::*;

// ---- the Canal Run ------------------------------------------------------------

pub fn draw_canal(blip: &Blip, g: &Game, t: f32) {
    set_sun(1.0);
    let w = WIN_W as f32;
    let water = [52, 104, 120];
    blip.clear(rgb(water));
    let start = sy(0.0);
    let harbour = sy(HARBOUR_D);
    // ripples on the canal and the harbour
    for k in 0..48 {
        let y = (scatter(k, 71) * 700.0 - g.bike_d * 0.0 + g.bike_d).rem_euclid(700.0);
        let y = HUD_H + (y - g.bike_d).rem_euclid(620.0);
        let x = 256.0 + scatter(k, 72) * 190.0;
        blip.fill_rect(x, y + (t * 2.0 + k as f32).sin() * 2.0, 14.0, 2.0, rgba([190, 220, 230], 0.28));
    }
    // the quay, the café terraces and Kongens Nytorv's end below the canal's head
    let (top, bot) = (harbour.max(HUD_H - 40.0), start.min(WIN_H as f32 + 40.0));
    if bot > top {
        blip.fill_rect(0.0, top, QUAY[1] + 4.0, bot - top, rgb([150, 146, 142]));
        let mut y = top - (top - start).rem_euclid(8.0);
        let mut row = 0;
        while y < bot {
            let off = if row % 2 == 0 { 0.0 } else { 5.0 };
            let mut x = off;
            while x < QUAY[1] {
                blip.fill_rect(x, y, 9.0, 6.0, rgb(shade_u([134, 132, 134], 0.9 + scatter(((y - start) as i32).unsigned_abs() as usize * 7 + x as usize, 5) * 0.2)));
                x += 10.0;
            }
            y += 8.0;
            row += 1;
        }
        blip.fill_rect(QUAY[1], top, 6.0, bot - top, rgb([206, 200, 190]));
        blip.fill_rect(436.0, top, w - 436.0, bot - top, rgb([150, 146, 142]));
        blip.fill_rect(436.0, top, 5.0, bot - top, rgb([206, 200, 190]));
        // bollards along the edge
        let mut d = (g.bike_d - 200.0).div_euclid(70.0) * 70.0;
        while d < g.bike_d + 500.0 {
            if d > 0.0 && d < HARBOUR_D {
                let y = sy(d);
                blip.fill_circle(QUAY[1] + 2.0, y, 3.5, rgb([30, 30, 34]));
            }
            d += 70.0;
        }
    }
    if start < WIN_H as f32 { blip.fill_rect(0.0, start, w, WIN_H as f32 - start, rgb([150, 146, 142])); }
    // terrace tables that are only scenery, back by the houses, and their umbrellas
    let mut d = (g.bike_d - 300.0).div_euclid(64.0) * 64.0;
    while d < g.bike_d + 500.0 {
        if d > 60.0 && d < HARBOUR_D - 40.0 {
            let y = sy(d);
            let k = (d / 64.0) as usize;
            let c = [[200, 40, 44], [240, 232, 214], [44, 110, 80], [230, 160, 40]][k % 4];
            blip.fill_circle(118.0 + 3.0, y + 3.0, 15.0, rgba([20, 20, 30], 0.25));
            blip.fill_circle(118.0, y, 15.0, rgb(c));
            blip.fill_circle(118.0, y, 2.0, rgb(shade_u(c, 0.7)));
        }
        d += 64.0;
    }
    // the targets on the terrace and the water
    for tg in &g.targets { draw_mark(blip, tg, t); }
    // Nyhavn's houses, both quays: narrow, tall and every colour, gables to the canal
    nyhavn_row(g, 0);
    nyhavn_row(g, 1);
    // the memorial anchor at the canal's head
    if start > HUD_H && start < WIN_H as f32 + 80.0 { anchor(blip, 330.0, start - 10.0); }
    // across the harbour: Christianshavn and the spire of Vor Frelsers Kirke
    if harbour > HUD_H - 300.0 {
        let far = sy(HARBOUR_D + 420.0);
        blip.fill_rect(0.0, far - 60.0, w, 60.0, rgb([120, 116, 112]));
        for i in 0..10 {
            let c = [[200, 120, 70], [230, 200, 140], [170, 80, 60], [210, 190, 160]][i % 4];
            blip.fill_rect(i as f32 * 48.0, far - 56.0, 46.0, 30.0, rgb(c));
            blip.fill_rect(i as f32 * 48.0, far - 66.0, 46.0, 12.0, rgb([150, 70, 50]));
        }
        vor_frelsers(blip, 330.0, far - 30.0, t);
    }
    for p in &g.papers { draw_paper(blip, p); }
    // gulls wheeling over the canal
    for k in 0..5 {
        let a = t * (0.4 + scatter(k, 91) * 0.3) + k as f32 * 1.7;
        let gx = 330.0 + a.cos() * (60.0 + scatter(k, 92) * 60.0);
        let gy = 160.0 + k as f32 * 70.0 + a.sin() * 40.0;
        let flap = (t * 9.0 + k as f32).sin() * 4.0;
        blip.draw_line_ex(gx - 9.0, gy - flap, gx, gy, 2.0, rgb([246, 246, 244]));
        blip.draw_line_ex(gx, gy, gx + 9.0, gy - flap, 2.0, rgb([246, 246, 244]));
        blip.fill_circle(gx, gy, 2.0, rgb([230, 230, 226]));
    }
}

pub(super) const NYHAVN: [[u8; 3]; 8] = [
    [206, 62, 48], [240, 184, 60], [64, 116, 186], [232, 122, 48], [238, 228, 204], [118, 168, 120], [204, 92, 112],
    [246, 210, 120],
];

/// One quay's houses: narrow and tall, each its own colour, white windows, a gable or a cornice on top.
pub(super) fn nyhavn_row(g: &Game, s: usize) {
    let wall_x = if s == 0 { 22.0 } else { 441.0 };
    let len = 46.0;
    let first = ((g.bike_d - 260.0) / len).floor().max(0.0) as i32;
    let last = ((g.bike_d + 560.0).min(HARBOUR_D) / len).ceil() as i32;
    for i in first..last {
        let (d0, d1) = (i as f32 * len, (i + 1) as f32 * len);
        let (y0, y1) = (sy(d1), sy(d0));
        let k = i as usize * 2 + s;
        let col = NYHAVN[(scatter(k, 41) * NYHAVN.len() as f32) as usize % NYHAVN.len()];
        let hh = 54.0 + (scatter(k, 42) * 3.0).floor() * 10.0;
        let face = |uy: f32, v: f32| lift(wall_x, uy, v);
        quad((wall_x, y0), (wall_x, y1), face(y1, hh), face(y0, hh), rgb(col));
        quad(face(y0, hh), face(y1, hh), face(y1, hh + 4.0), face(y0, hh + 4.0), rgb([246, 240, 228]));
        // the gable, stepped up to a point
        let mid = (y0 + y1) / 2.0;
        if scatter(k, 43) > 0.4 { tri(face(y0 + 4.0, hh + 4.0), face(y1 - 4.0, hh + 4.0), face(mid, hh + 20.0), rgb(col)); }
        let frame = rgb([246, 240, 228]);
        for floor in 0..((hh / 14.0) as i32 - 1) {
            let v0 = 6.0 + floor as f32 * 14.0;
            for wy in [y0 + 8.0, mid - 4.0, y1 - 16.0] {
                quad(face(wy - 1.0, v0 - 1.0), face(wy + 9.0, v0 - 1.0), face(wy + 9.0, v0 + 10.0), face(wy - 1.0, v0 + 10.0), frame);
                quad(face(wy, v0), face(wy + 8.0, v0), face(wy + 8.0, v0 + 9.0), face(wy, v0 + 9.0), rgb([60, 80, 104]));
            }
        }
        let roof = [176, 70, 46];
        let back = if s == 0 { -40.0 } else { 520.0 };
        quad(face(y0, hh + 4.0), face(y1, hh + 4.0), lift(back, y1, hh + 20.0), lift(back, y0, hh + 20.0), rgb(roof));
    }
}

/// A Nyhavn target: a café table with someone waiting for the paper, an old wooden ship with a basket on deck, or
/// a canal tour boat.
pub(super) fn draw_mark(blip: &Blip, tg: &Target, t: f32) {
    let y = sy(tg.d);
    if !(-120.0..WIN_H as f32 + 120.0).contains(&y) { return; }
    let bob = (t * 2.0 + tg.d * 0.05).sin() * 1.5;
    match tg.mark {
        Mark::Table => {
            blip.fill_circle(tg.x + 2.0, y + 3.0, 10.0, rgba([20, 20, 30], 0.25));
            blip.fill_circle(tg.x, y, 10.0, rgb([250, 248, 240]));
            blip.fill_circle(tg.x + 3.0, y - 3.0, 2.5, rgb([120, 80, 50]));
            person(blip, tg.x - 15.0, y + 2.0, [[60, 90, 160], [200, 80, 70], [80, 140, 90]][(tg.d as usize / 7) % 3], [90, 60, 40], 0.0);
        }
        Mark::Ship => {
            let (cx, cy) = (308.0, y + bob * 0.3);
            // one of them is the old red lightship
            let lightship = (tg.d as usize / 150) % 4 == 2;
            // the hull, the deck and its deckhouse
            blip.fill_rect(cx - 18.0 + 4.0, cy - 46.0 + 4.0, 36.0, 92.0, rgba([10, 20, 30], 0.3));
            let hull = if lightship { [196, 36, 36] } else { [86, 52, 34] };
            blip.fill_circle(cx, cy - 46.0, 18.0, rgb(hull));
            blip.fill_rect(cx - 18.0, cy - 46.0, 36.0, 92.0, rgb(hull));
            blip.fill_circle(cx, cy - 44.0, 14.0, rgb([196, 160, 112]));
            blip.fill_rect(cx - 14.0, cy - 44.0, 28.0, 86.0, rgb([196, 160, 112]));
            for i in 0..8 { blip.fill_rect(cx - 14.0, cy - 40.0 + i as f32 * 11.0, 28.0, 1.0, rgb([170, 134, 92])); }
            blip.fill_rect(cx - 8.0, cy + 18.0, 16.0, 18.0, rgb([120, 80, 52]));
            // masts, yards and furled sails, leaning with the height
            for md in [-26.0, 8.0] {
                let foot = (cx, cy + md);
                let tip = lift(cx, cy + md, 90.0);
                blip.draw_line_ex(foot.0, foot.1, tip.0, tip.1, 2.0, rgb([110, 76, 48]));
                for (v, half) in [(40.0, 16.0), (64.0, 12.0)] {
                    let a = lift(cx, cy + md - half, v);
                    let b = lift(cx, cy + md + half, v);
                    blip.draw_line_ex(a.0, a.1, b.0, b.1, 3.0, rgb([236, 228, 206]));
                }
                blip.draw_line(tip.0, tip.1, cx, cy - 60.0, rgba([40, 30, 26], 0.5));
                blip.draw_line(tip.0, tip.1, cx, cy + 44.0, rgba([40, 30, 26], 0.5));
                blip.fill_rect(tip.0, tip.1, 8.0, 5.0, rgb([200, 22, 40]));
                blip.fill_rect(tip.0 + 2.0, tip.1, 1.5, 5.0, rgb([250, 250, 250]));
            }
            // the basket on deck, where the paper goes
            blip.fill_circle(tg.x, y, 9.0, rgb([196, 150, 80]));
            blip.draw_circle(tg.x, y, 9.0, 2.0, rgb([150, 104, 50]));
            blip.fill_circle(tg.x, y, 5.0, rgb([120, 80, 40]));
        }
        Mark::TourBoat => {
            let (cx, cy) = (tg.x, y);
            blip.fill_rect(cx - 15.0 + 4.0, cy - 50.0 + 4.0, 30.0, 100.0, rgba([10, 20, 30], 0.3));
            blip.fill_circle(cx, cy - 48.0, 15.0, rgb([240, 240, 236]));
            blip.fill_rect(cx - 15.0, cy - 48.0, 30.0, 96.0, rgb([240, 240, 236]));
            blip.fill_rect(cx - 15.0, cy - 48.0, 30.0, 4.0, rgb([40, 70, 130]));
            blip.fill_rect(cx - 11.0, cy - 36.0, 22.0, 74.0, rgba([150, 200, 230], 0.85));
            for i in 0..6 {
                for j in 0..2 {
                    let fruit = [[230, 190, 150], [120, 80, 50], [240, 220, 160]][(i + j) % 3];
                    blip.fill_circle(cx - 5.0 + j as f32 * 10.0, cy - 30.0 + i as f32 * 12.0, 3.0, rgb(fruit));
                }
            }
            blip.fill_circle(cx, cy - 4.0, 8.0, rgb([250, 250, 250]));
            blip.draw_circle(cx, cy - 4.0, 8.0, 2.0, rgb([40, 70, 130]));
        }
    }
    if tg.hit >= 0.0 {
        let (px, py) = if tg.mark == Mark::TourBoat { (tg.x, y - 4.0) } else { (tg.x, y) };
        blip.fill_rect(px - 5.0, py - 2.0, 10.0, 4.0, rgb([250, 248, 238]));
        blip.fill_rect(px - 1.0, py - 2.0, 2.0, 4.0, rgb([200, 60, 60]));
        if tg.hit < 0.6 { blip.draw_circle(px, py, 12.0 + tg.hit * 30.0, 2.0, rgba(GOLD, 1.0 - tg.hit / 0.6)); }
    } else {
        let pts = Game::target_points(tg.x);
        let c = if pts >= 400 { GOLD } else { [255, 255, 255] };
        label(blip, &pts.to_string(), tg.x, y - 26.0 + bob, 2.0, rgba(c, 0.85));
    }
}

/// The memorial anchor at the head of Nyhavn, on its granite plinth.
pub(super) fn anchor(blip: &Blip, cx: f32, base: f32) {
    blip.fill_rect(cx - 22.0, base - 10.0, 44.0, 14.0, rgb([170, 166, 160]));
    blip.fill_rect(cx - 18.0, base - 16.0, 36.0, 8.0, rgb([190, 186, 180]));
    let ink = rgb([26, 26, 30]);
    blip.fill_rect(cx - 3.0, base - 86.0, 6.0, 72.0, ink);
    blip.draw_line_ex(cx - 18.0, base - 76.0, cx + 18.0, base - 76.0, 5.0, ink);
    blip.draw_circle(cx, base - 92.0, 6.0, 3.0, ink);
    for i in 0..10 {
        let a = PI * (0.1 + 0.08 * i as f32);
        let r = 26.0;
        blip.fill_circle(cx + a.cos() * r * 1.0, base - 34.0 + a.sin() * r * 0.8 - 0.0, 3.5, ink);
        blip.fill_circle(cx - a.cos() * r * 1.0, base - 34.0 + a.sin() * r * 0.8, 3.5, ink);
    }
    blip.fill_circle(cx + 26.0, base - 32.0, 5.0, ink);
    blip.fill_circle(cx - 26.0, base - 32.0, 5.0, ink);
}

/// Vor Frelsers Kirke across the harbour: the brick tower and its dark spire, the gilded stair winding round it to
/// the globe at the top.
pub(super) fn vor_frelsers(blip: &Blip, cx: f32, base: f32, t: f32) {
    blip.fill_rect(cx - 22.0, base - 70.0, 44.0, 70.0, rgb([150, 70, 52]));
    for i in 0..4 { blip.fill_rect(cx - 16.0 + i as f32 * 10.0, base - 60.0, 5.0, 14.0, rgb([226, 214, 196])); }
    blip.fill_rect(cx - 24.0, base - 74.0, 48.0, 6.0, rgb([200, 190, 170]));
    // the spire, narrowing in steps, with the golden stair round it
    let h = 150.0;
    for i in 0..(h as i32) {
        let f = i as f32 / h;
        let half = 16.0 * (1.0 - f) + 1.5;
        blip.fill_rect(cx - half, base - 74.0 - i as f32, half * 2.0, 1.2, rgb([46, 44, 48]));
    }
    for i in 0..(h as i32 * 2) {
        let f = i as f32 / (h * 2.0);
        let half = 16.0 * (1.0 - f) + 3.0;
        let a = f * TAU * 4.0 + t * 0.0;
        if a.sin() < -0.2 { continue; }
        blip.fill_rect(cx + a.cos() * half - 1.0, base - 74.0 - f * h, 2.0, 2.0, rgb(GOLD));
    }
    blip.fill_circle(cx, base - 74.0 - h - 4.0, 5.0, rgb(GOLD));
    blip.fill_rect(cx - 1.0, base - 74.0 - h - 18.0, 2.0, 12.0, rgb(GOLD));
}
