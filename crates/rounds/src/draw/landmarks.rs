//! The landmarks along the round: forecourts and plazas, Rundetaarn and the Marble Church from below, Amalienborg's
//! palace and its guards, Frederik on his horse.

use super::*;

// ---- landmarks ------------------------------------------------------------------
// Drawn as people know them: standing up, from the front.

/// Granite setts in front of a landmark.
pub(super) fn forecourt(blip: &Blip, s: usize, y: f32, r: usize) {
    side_rect(blip, s, 0.0, y, FENCE[0], LOT, rgb([150, 148, 146]));
    for row in 0..14 {
        let off = if row % 2 == 0 { 0.0 } else { 5.0 };
        let mut x = off;
        while x < FENCE[0] - 8.0 {
            side_rect(blip, s, x, y + row as f32 * 8.0, 9.0, 6.0, rgb(shade_u([132, 130, 132], 0.9 + scatter(r * 300 + row * 20 + x as usize, 5) * 0.2)));
            x += 10.0;
        }
    }
}

/// Amalienborg's square: setts, laid in an octagon of pale granite strips.
pub(super) fn plaza(blip: &Blip, r: usize, y: f32) {
    let w = WIN_W as f32;
    blip.fill_rect(0.0, y, w, LOT, rgb([138, 136, 136]));
    for row in 0..14 {
        let off = if (r * 14 + row).is_multiple_of(2) { 0.0 } else { 5.0 };
        let mut x = off;
        let mut k = 0;
        while x < w {
            blip.fill_rect(x, y + row as f32 * 8.0, 9.0, 6.0, rgb(shade_u([124, 122, 124], 0.9 + scatter(r * 900 + row * 50 + k, 88) * 0.2)));
            x += 10.0;
            k += 1;
        }
    }
    // pale granite strips: an octagon round the statue
    let cy = sy(statue_d());
    let pale = rgba([214, 208, 196], 0.9);
    for i in 0..8 {
        let a0 = (i as f32 + 0.5) / 8.0 * TAU;
        let a1 = (i as f32 + 1.5) / 8.0 * TAU;
        let rr = 150.0;
        blip.draw_line_ex(240.0 + a0.cos() * rr, cy + a0.sin() * rr * 0.75, 240.0 + a1.cos() * rr, cy + a1.sin() * rr * 0.75, 3.0, pale);
        blip.draw_line_ex(240.0 + a0.cos() * 40.0, cy + a0.sin() * 30.0, 240.0 + a0.cos() * rr, cy + a0.sin() * rr * 0.75, 2.0, pale);
    }
}

/// A point `h` up from (`x`, `y`) on the ground: the same perspective as the houses (`lift`).
pub(super) fn up(p: (f32, f32), h: f32) -> (f32, f32) { lift(p.0, p.1, h) }

/// How a wall facing `n` (a unit vector on the ground) is lit by the low sun ahead and to the right: 0.62 to 1.08.
pub(super) fn sunlit(n: (f32, f32)) -> f32 {
    let s = (0.33f32, -0.94f32);
    0.85 + 0.23 * (n.0 * s.0 + n.1 * s.1)
}

/// A round wall from `h0` to `h1` on a circle of radius `rad` round `c`, seen from above as the houses are: only the
/// side facing the viewer (toward the vanishing point), each strip lit by the sun; `strip` may draw on each strip.
pub(super) fn round_wall(c: (f32, f32), rad: f32, h0: f32, h1: f32, col: [u8; 3], strip: &mut dyn FnMut(f32, f32, f32)) {
    let v = vanish();
    let n = 36;
    for i in 0..n {
        let (a0, a1) = (i as f32 / n as f32 * TAU, (i + 1) as f32 / n as f32 * TAU);
        let am = (a0 + a1) / 2.0;
        let nrm = (am.cos(), am.sin());
        if nrm.0 * (v.0 - c.0) + nrm.1 * (v.1 - c.1) < 0.0 { continue; }
        let (p0, p1) = ((c.0 + rad * a0.cos(), c.1 + rad * a0.sin()), (c.0 + rad * a1.cos(), c.1 + rad * a1.sin()));
        quad(up(p0, h0), up(p1, h0), up(p1, h1), up(p0, h1), rgb(shade_u(col, sunlit(nrm))));
        strip(am, a0, a1);
    }
}

/// The flat top of a round wall at height `h`: a disc in the houses' perspective.
pub(super) fn round_top(c: (f32, f32), rad: f32, h: f32, col: BlipColor) {
    let n = 36;
    let mid = up(c, h);
    for i in 0..n {
        let (a0, a1) = (i as f32 / n as f32 * TAU, (i + 1) as f32 / n as f32 * TAU);
        tri(mid, up((c.0 + rad * a0.cos(), c.1 + rad * a0.sin()), h), up((c.0 + rad * a1.cos(), c.1 + rad * a1.sin()), h), col);
    }
}

/// Rundetaarn (1642) seen from above like the houses: the weathered brick drum, its arched windows climbing with the
/// ramp on the side facing you, the sandstone cornice, the iron railing round the platform and the observatory with
/// its dark dome in the middle.
pub(super) fn rundetaarn_up(blip: &Blip, g: &Game, s: usize, r: usize) {
    let c = (mx(s, 62.0), sy(door_d(r)));
    // 46 high in the houses' units (the palace is 54): taller leans so far toward the edge it reads as lying down
    let (rad, hh) = (32.0, 46.0);
    let (ox, oy) = sun(hh * 0.18);
    blip.fill_circle(c.0 + ox, c.1 + oy, rad + 6.0, rgba([20, 20, 30], 0.28));
    let glass = if daylight(g) < 0.4 { [255, 214, 130] } else { [56, 64, 84] };
    round_wall(c, rad, 0.0, hh, [150, 88, 66], &mut |am, _, _| {
        let _ = &glass;
        // windows on the ramp's spiral: one strip in three has one, higher the further round
        let turn = (am / TAU * 9.0).round() as i32;
        if turn % 2 != 0 { return; }
        let wh = 10.0 + ((am / TAU * 7.0).fract()) * (hh - 26.0);
        let p = (c.0 + (rad + 0.2) * am.cos(), c.1 + (rad + 0.2) * am.sin());
        let (b, t) = (up(p, wh), up(p, wh + 6.0));
        blip.draw_line_ex(b.0, b.1, t.0, t.1, 3.5, rgb([226, 214, 196]));
        blip.draw_line_ex(b.0, b.1, t.0, t.1, 2.0, rgb(glass));
    });
    // brick courses and the cornice ring
    round_wall(c, rad + 2.0, hh - 3.0, hh, [216, 206, 188], &mut |_, _, _| {});
    round_top(c, rad + 2.0, hh, rgb([196, 186, 168]));
    round_top(c, rad - 2.0, hh, rgb([160, 150, 134]));
    // the railing: black posts round the rim
    for i in 0..24 {
        let a = i as f32 / 24.0 * TAU;
        let p = (c.0 + (rad + 1.0) * a.cos(), c.1 + (rad + 1.0) * a.sin());
        let (b, t) = (up(p, hh), up(p, hh + 5.0));
        blip.draw_line(b.0, b.1, t.0, t.1, rgb([30, 30, 36]));
    }
    // the observatory: a pale drum and its dark dome
    round_wall(c, 11.0, hh, hh + 8.0, [214, 206, 190], &mut |_, _, _| {});
    // the dome's grey-green (OpenStreetMap's roof colour, #74847f)
    round_top(c, 11.0, hh + 8.0, rgb([100, 116, 110]));
    round_top(c, 6.0, hh + 11.0, rgb([128, 144, 138]));
    let slit = (up((c.0 - 1.0, c.1 - 10.0), hh + 9.0), up((c.0 - 1.0, c.1 + 10.0), hh + 9.0));
    blip.draw_line_ex(slit.0 .0, slit.0 .1, slit.1 .0, slit.1 .1, 1.5, rgb([30, 34, 38]));
}

/// Marmorkirken seen from above like the houses: the round marble church, its portico toward the street, the green
/// copper dome on top, ribbed, with the ring of round dormers, the lantern and the gilded cross.
pub(super) fn marmorkirken_up(blip: &Blip, s: usize, r: usize) {
    let c = (mx(s, 56.0), sy(door_d(r)));
    // kept low in the houses' units (body 22, dome 20) for the same reason as Rundetaarn
    let (rad, hh) = (42.0, 22.0);
    let toward = if s == 0 { 1.0 } else { -1.0 };
    let (ox, oy) = sun(hh * 0.3);
    blip.fill_circle(c.0 + ox, c.1 + oy, rad + 8.0, rgba([20, 20, 30], 0.28));
    // grey Norwegian marble and pale verdigris copper (OpenStreetMap: #c6c7ce walls, #b5dbce roof)
    let marble = [204, 205, 210];
    // the portico: a box toward the street, its columns, its pediment roof
    let (px0, px1, py0, py1) = (c.0 + toward * (rad - 8.0), c.0 + toward * (rad + 16.0), c.1 - 26.0, c.1 + 26.0);
    let (pl, pr) = (px0.min(px1), px0.max(px1));
    let front = if toward > 0.0 { pr } else { pl };
    let ph = 18.0;
    quad(up((front, py0), 0.0), up((front, py1), 0.0), up((front, py1), ph), up((front, py0), ph), rgb(shade_u(marble, 1.02)));
    for i in 0..6 {
        let yy = py0 + 5.0 + i as f32 * 8.4;
        let (b, t) = (up((front, yy), 0.0), up((front, yy), ph - 2.0));
        blip.draw_line_ex(b.0, b.1, t.0, t.1, 3.0, rgb([246, 242, 232]));
    }
    quad(up((pl, py1), 0.0), up((pr, py1), 0.0), up((pr, py1), ph), up((pl, py1), ph), rgb(shade_u(marble, 0.8)));
    quad(up((pl, py0), ph), up((pr, py0), ph), up((pr, py1), ph), up((pl, py1), ph), rgb(shade_u(marble, 0.9)));
    let (a, b2) = (up((front, py0), ph), up((front, py1), ph));
    tri(a, b2, up((front, c.1), ph + 7.0), rgb(shade_u(marble, 1.05)));
    // the body: columns all round, tall windows between
    round_wall(c, rad, 0.0, hh, marble, &mut |am, _, _| {
        let p = (c.0 + (rad + 0.3) * am.cos(), c.1 + (rad + 0.3) * am.sin());
        let (b, t) = (up(p, 3.0), up(p, hh - 3.0));
        if ((am / TAU * 36.0).round() as i32) % 3 == 0 {
            blip.draw_line_ex(b.0, b.1, t.0, t.1, 2.0, rgb([246, 242, 232]));
        } else if ((am / TAU * 36.0).round() as i32) % 3 == 1 {
            let (w0, w1) = (up(p, 6.0), up(p, 16.0));
            blip.draw_line_ex(w0.0, w0.1, w1.0, w1.1, 2.0, rgb([70, 80, 96]));
        }
    });
    round_top(c, rad, hh, rgb(shade_u(marble, 0.9)));
    // the dome: copper rings rising and narrowing, lit toward the sun, the ribs over them
    let copper = [150, 198, 178];
    let (dr_, dh) = (38.0, 20.0);
    for k in 0..10 {
        let f = k as f32 / 10.0;
        let rr = dr_ * (1.0 - f * f).sqrt();
        round_top(c, rr, hh + 2.0 + f * dh, rgb(shade_u(copper, 0.8 + 0.3 * f)));
    }
    for i in 0..16 {
        let a = i as f32 / 16.0 * TAU;
        let mut prev = up((c.0 + dr_ * a.cos(), c.1 + dr_ * a.sin()), hh + 2.0);
        for k in 1..10 {
            let f = k as f32 / 10.0;
            let rr = dr_ * (1.0 - f * f).sqrt();
            let p = up((c.0 + rr * a.cos(), c.1 + rr * a.sin()), hh + 2.0 + f * dh);
            blip.draw_line(prev.0, prev.1, p.0, p.1, rgb(shade_u(copper, 0.66)));
            prev = p;
        }
    }
    // round dormers low on the dome, on the side you see
    let v = vanish();
    for i in 0..12 {
        let a = i as f32 / 12.0 * TAU + 0.26;
        if a.cos() * (v.0 - c.0) + a.sin() * (v.1 - c.1) < 0.0 { continue; }
        let p = up((c.0 + dr_ * 0.92 * a.cos(), c.1 + dr_ * 0.92 * a.sin()), hh + 8.0);
        blip.fill_circle(p.0, p.1, 2.6, rgb([226, 220, 206]));
        blip.fill_circle(p.0, p.1, 1.6, rgb([50, 60, 74]));
    }
    // the lantern, its little copper cap, the gilded ball and cross
    round_wall(c, 7.0, hh + dh, hh + dh + 10.0, [214, 208, 194], &mut |_, _, _| {});
    round_top(c, 7.5, hh + dh + 10.0, rgb(copper));
    let (b, t) = (up(c, hh + dh + 10.0), up(c, hh + dh + 20.0));
    blip.draw_line_ex(b.0, b.1, t.0, t.1, 1.5, rgb(GOLD));
    blip.fill_circle(t.0, t.1, 2.0, rgb(GOLD));
}

/// One of Amalienborg's four palaces: sandstone, three floors of tall windows, a copper roof.
pub(super) fn palace(blip: &Blip, s: usize, r: usize) {
    let (y0, y1) = (sy((r + 1) as f32 * LOT), sy(r as f32 * LOT));
    let wall_x = mx(s, 96.0);
    let (back, ridge) = (mx(s, -20.0), mx(s, 40.0));
    let hh = 54.0;
    let face = |uy: f32, v: f32| lift(wall_x, uy, v);
    // OpenStreetMap: walls #deccb7 under near-black hipped roofs #353440
    let stone = [222, 204, 183];
    quad((back, y0), (wall_x, y0), face(y0, hh), lift(back, y0, hh), rgb(shade_u(stone, 0.8)));
    quad((back, y1), (wall_x, y1), face(y1, hh), lift(back, y1, hh), rgb(shade_u(stone, 0.8)));
    quad((wall_x, y0), (wall_x, y1), face(y1, hh), face(y0, hh), rgb(stone));
    quad((wall_x, y0), (wall_x, y1), face(y1, 12.0), face(y0, 12.0), rgb(shade_u(stone, 0.86)));
    for floor in 0..3 {
        let (v0, v1) = (4.0 + floor as f32 * 17.0, 14.0 + floor as f32 * 17.0);
        let mut u = y0 + 8.0;
        while u < y1 - 12.0 {
            quad(face(u, v0), face(u + 8.0, v0), face(u + 8.0, v1), face(u, v1), rgb([250, 246, 236]));
            quad(face(u + 1.0, v0 + 1.0), face(u + 7.0, v0 + 1.0), face(u + 7.0, v1 - 1.0), face(u + 1.0, v1 - 1.0), rgb([70, 86, 104]));
            u += 15.0;
        }
    }
    // the roof: dark slate, as the four palaces have it, with a row of white dormers
    let slate = [58, 57, 70];
    let (e0, e1, r0, r1) = (face(y0, hh), face(y1, hh), lift(ridge, y0, hh + 24.0), lift(ridge, y1, hh + 24.0));
    quad(lift(back, y0, hh), lift(back, y1, hh), r1, r0, rgb(shade_u(slate, 0.75)));
    quad(e0, e1, r1, r0, rgb(slate));
    for i in 1..4 {
        let f = i as f32 / 4.0;
        let (a, b) = (mix2(e0, r0, f), mix2(e1, r1, f));
        blip.draw_line(a.0, a.1, b.0, b.1, rgb(shade_u(slate, 0.85)));
    }
    let mut u = y0 + 12.0;
    while u < y1 - 14.0 {
        let (p0, p1) = (mix2(face(u, hh), lift(ridge, u, hh + 24.0), 0.35), mix2(face(u + 7.0, hh), lift(ridge, u + 7.0, hh + 24.0), 0.35));
        blip.fill_rect(p0.0.min(p1.0) - 1.0, p0.1.min(p1.1) - 6.0, (p1.0 - p0.0).abs() + 3.0, 7.0, rgb([238, 234, 224]));
        blip.fill_rect(p0.0.min(p1.0), p0.1.min(p1.1) - 5.0, (p1.0 - p0.0).abs() + 1.0, 5.0, rgb([60, 70, 86]));
        u += 22.0;
    }
    // the centre: a pediment over the middle bay, in paler stone
    let mid = (y0 + y1) / 2.0;
    tri(face(mid - 20.0, hh), face(mid + 20.0, hh), face(mid, hh + 12.0), rgb(shade_u(stone, 1.06)));
    let (c0, c1) = (face(mid - 20.0, hh), face(mid + 20.0, hh));
    blip.draw_line(c0.0, c0.1, c1.0, c1.1, rgb(shade_u(stone, 0.8)));
    // a Royal Life Guard on watch by the palace: bearskin, dark blue tunic, white belts, light blue trousers
    if r == STATUE_ROW {
        stand(mx(s, 128.0), sy(door_d(r)), Stand::Guard);
    }
}

/// A Royal Life Guard, standing (`step` 0) or marching.
pub fn guard(blip: &Blip, x: f32, y: f32, step: f32) {
    blip.fill_rect(x - 4.0, y - 2.0, 10.0, 4.0, rgba([20, 20, 30], 0.3));
    blip.fill_rect(x - 3.0, y - 12.0 + step.max(0.0), 2.0, 12.0 - step.max(0.0), rgb([120, 150, 200]));
    blip.fill_rect(x + 1.0, y - 12.0 - step.min(0.0), 2.0, 12.0 + step.min(0.0), rgb([120, 150, 200]));
    blip.fill_rect(x - 4.0, y - 24.0, 8.0, 13.0, rgb([28, 36, 70]));
    blip.draw_line(x - 4.0, y - 24.0, x + 4.0, y - 13.0, rgb([240, 240, 240]));
    blip.draw_line(x + 4.0, y - 24.0, x - 4.0, y - 13.0, rgb([240, 240, 240]));
    blip.fill_rect(x - 1.5, y - 27.0, 3.0, 3.0, rgb([236, 196, 170]));
    blip.fill_circle(x, y - 33.0, 5.0, rgb([18, 16, 20]));
    blip.fill_rect(x - 4.0, y - 36.0, 8.0, 8.0, rgb([18, 16, 20]));
    blip.draw_line(x + 6.0, y - 34.0, x + 6.0, y - 14.0, rgb([60, 46, 36]));
}

/// Frederik V on his horse, high on the pedestal in the middle of Amalienborg's square.
pub(super) fn frederik(blip: &Blip, cx: f32, base: f32) {
    // Saly's statue as it stands: dark green-grey bronze, a tall veined marble plinth with bronze medallions on its
    // faces, and the black iron railing round it
    let bronze = [62, 104, 92];
    let lit = shade_u(bronze, 1.35);
    blip.fill_rect(cx - 26.0, base - 6.0, 58.0, 14.0, rgba([20, 20, 30], 0.3));
    let marble = [222, 216, 204];
    blip.fill_rect(cx - 15.0, base - 46.0, 30.0, 46.0, rgb(marble));
    blip.fill_rect(cx - 15.0, base - 46.0, 4.0, 46.0, rgb(shade_u(marble, 0.86)));
    blip.fill_rect(cx - 18.0, base - 50.0, 36.0, 5.0, rgb(shade_u(marble, 0.9)));
    blip.fill_rect(cx - 19.0, base - 6.0, 38.0, 6.0, rgb(shade_u(marble, 0.88)));
    for k in [-6.0, 3.0, 9.0] { blip.draw_line(cx + k, base - 44.0, cx + k + 3.0, base - 8.0, rgba([170, 160, 150], 0.5)); }
    blip.fill_circle(cx, base - 27.0, 7.0, rgb(shade_u(bronze, 0.8)));
    blip.fill_circle(cx, base - 27.0, 5.0, rgb(bronze));
    blip.fill_circle(cx - 1.5, base - 28.5, 2.0, rgb(lit));
    // the horse, stepping out, and the king in his Roman armour with the baton
    let y = base - 50.0;
    blip.fill_rect(cx - 16.0, y - 22.0, 28.0, 12.0, rgb(bronze));
    blip.fill_rect(cx - 14.0, y - 22.0, 24.0, 3.0, rgb(lit));
    blip.fill_circle(cx - 16.0, y - 16.0, 6.0, rgb(bronze));
    blip.fill_circle(cx + 12.0, y - 16.0, 6.0, rgb(bronze));
    for (lx, lift_) in [(-14.0, 0.0), (-8.0, 0.0), (6.0, 0.0), (11.0, 6.0)] {
        blip.fill_rect(cx + lx, y - 12.0 - lift_, 3.0, 12.0, rgb(shade_u(bronze, 0.85)));
    }
    blip.fill_rect(cx + 12.0, y - 32.0, 7.0, 14.0, rgb(bronze));
    blip.fill_rect(cx + 15.0, y - 34.0, 9.0, 6.0, rgb(bronze));
    blip.fill_rect(cx + 15.0, y - 34.0, 9.0, 2.0, rgb(lit));
    blip.draw_line_ex(cx - 20.0, y - 20.0, cx - 24.0, y - 6.0, 2.0, rgb(bronze));
    blip.fill_rect(cx - 4.0, y - 38.0, 8.0, 16.0, rgb(bronze));
    blip.fill_rect(cx - 4.0, y - 38.0, 3.0, 16.0, rgb(lit));
    blip.fill_circle(cx, y - 41.0, 4.0, rgb(bronze));
    blip.draw_line_ex(cx + 3.0, y - 34.0, cx + 12.0, y - 40.0, 2.0, rgb(bronze));
    // the railing: black posts and two rails, around the foot
    let iron = rgb([28, 28, 32]);
    for i in 0..9 {
        let x = cx - 24.0 + i as f32 * 6.0;
        blip.fill_rect(x, base - 9.0, 1.5, 10.0, iron);
    }
    blip.fill_rect(cx - 24.0, base - 9.0, 50.0, 1.5, iron);
    blip.fill_rect(cx - 24.0, base - 4.0, 50.0, 1.0, iron);
}
