//! Height: the slight lean that lifts walls, roofs and flags off the street toward the vanishing point, and the
//! buildings drawn with it.

use super::*;

// ---- height ---------------------------------------------------------------
// The street is seen from above the bike: anything with height leans away from the middle of the street and from
// the bike, as it would through a camera. Terraces show their fronts; towers stand up as they pass.

/// The point straight under the camera, where nothing leans: under the bike.
pub(super) fn vanish() -> (f32, f32) { (VIEW.with(|v| v.get().1), BIKE_Y + 30.0) }
/// How far a point leans per pixel of height, across the street and (a long lens: less) along it.
pub(super) const LEAN: f32 = 0.0068;
pub(super) const LEAN_ALONG: f32 = 0.0022;

/// Where a point `h` up from (`x`, `y`) is drawn.
pub fn lift(x: f32, y: f32, h: f32) -> (f32, f32) {
    // The lean is the camera's: more across the screen than up it, whichever way the street has been turned.
    let v = vanish();
    let (dx, dy) = (x - v.0, y - v.1);
    let (s, c) = TURNED.with(|a| a.get()).sin_cos();
    let (sx, sy_) = ((dx * c - dy * s) * h * LEAN, (dx * s + dy * c) * h * LEAN_ALONG);
    (x + sx * c + sy_ * s, y - sx * s + sy_ * c)
}

pub(super) fn tri(a: (f32, f32), b: (f32, f32), c: (f32, f32), col: BlipColor) {
    use blip::macroquad::math::vec2;
    blip::macroquad::shapes::draw_triangle(vec2(a.0, a.1), vec2(b.0, b.1), vec2(c.0, c.1), col);
}

pub(super) fn quad(a: (f32, f32), b: (f32, f32), c: (f32, f32), d: (f32, f32), col: BlipColor) {
    tri(a, b, c, col);
    tri(a, c, d, col);
}

pub(super) fn mix2(a: (f32, f32), b: (f32, f32), k: f32) -> (f32, f32) { (a.0 + (b.0 - a.0) * k, a.1 + (b.1 - a.1) * k) }

/// A box standing on the ground, `hw` x `hh` either side of (`x`, `y`), `lo` to `hi` up: its walls that face the
/// camera, then its top.
#[allow(clippy::too_many_arguments)]
pub fn block(x: f32, y: f32, hw: f32, hh: f32, lo: f32, hi: f32, wall: BlipColor, top: BlipColor) {
    let c = [(x - hw, y - hh), (x + hw, y - hh), (x + hw, y + hh), (x - hw, y + hh)];
    let b: Vec<_> = c.iter().map(|p| lift(p.0, p.1, lo)).collect();
    let t: Vec<_> = c.iter().map(|p| lift(p.0, p.1, hi)).collect();
    // the faces toward the vanishing point show
    if x - hw > vanish().0 { quad(b[0], b[3], t[3], t[0], BlipColor { r: wall.r * 0.85, g: wall.g * 0.85, b: wall.b * 0.85, ..wall }); }
    if x + hw < vanish().0 { quad(b[1], b[2], t[2], t[1], BlipColor { r: wall.r * 0.85, g: wall.g * 0.85, b: wall.b * 0.85, ..wall }); }
    if y + hh < vanish().1 { quad(b[3], b[2], t[2], t[3], wall); }
    if y - hh > vanish().1 { quad(b[0], b[1], t[1], t[0], wall); }
    quad(t[0], t[1], t[2], t[3], top);
}

/// The height of a terrace: a whole row the same, as on a real street; rows change at side streets and parks.
pub(super) fn terrace_height(g: &Game, s: usize, r: usize) -> f32 {
    let block = (0..r).filter(|&i| !g.houses[s][i].lot.is_house()).count();
    let pick = |hs: &[f32]| hs[(block + s) % hs.len()];
    match district(if s == 0 { Side::Left } else { Side::Right }, r) {
        District::Norrebro => pick(&[66.0, 58.0, 72.0]),
        District::IndreBy => pick(&[46.0, 52.0]),
        District::Nyboder => pick(&[28.0, 32.0]),
        District::Frederiksstaden => pick(&[56.0, 62.0]),
    }
}

/// A house's walls and roof, by the part of the city it stands in.
pub(super) fn house_colours(s: usize, r: usize, k: usize) -> ([u8; 3], [u8; 3]) {
    let pick = |c: &[[u8; 3]], salt: u32| c[(scatter(k, salt) * c.len() as f32) as usize % c.len()];
    match district(side_of(s), r) {
        // brick and plaster blocks of flats, slate and dark tile
        District::Norrebro => (pick(&[[168, 74, 54], [212, 168, 96], [192, 188, 178], [230, 206, 150], [142, 60, 48], [200, 140, 96]], 1),
            pick(&[[84, 86, 96], [120, 56, 42], [96, 98, 108]], 2)),
        District::IndreBy => (pick(&WALLS, 1), pick(&ROOFS, 2)),
        // Nyboder's ochre-orange (it reads orange in the street, not lemon), red tile
        District::Nyboder => (pick(&[[226, 150, 62], [220, 142, 58]], 1), pick(&[[163, 51, 30], [172, 60, 38]], 2)),
        // sandstone and grey under dark slate (OpenStreetMap: #21213a, #3f4155), not copper
        District::Frederiksstaden => (pick(&[[220, 206, 180], [200, 196, 188], [228, 214, 190], [212, 190, 160]], 1),
            pick(&[[52, 52, 68], [63, 65, 85], [72, 74, 88]], 2)),
    }
}

/// The house itself, standing up: its front wall with windows, door and letterbox, gable ends where the row stops,
/// the red tile roof with its chimney and dormers, and the goodwill hearts above.
pub(super) fn draw_building(blip: &Blip, g: &Game, s: usize, r: usize, t: f32) {
    let h = g.houses[s][r];
    let k = r * 2 + s;
    let (wall, roof) = house_colours(s, r, k);
    let nyboder = district(side_of(s), r) == District::Nyboder;
    // Nyboder's doors are all bottle green
    let door = if nyboder { [44, 96, 64] } else { DOORS[(scatter(k, 3) * DOORS.len() as f32) as usize % DOORS.len()] };
    let (y0, y1) = (sy((r + 1) as f32 * LOT), sy(r as f32 * LOT));
    let dy = sy(door_d(r));
    let (w, back, ridge) = (mx(s, WALL[0]), mx(s, -30.0), mx(s, 30.0));
    let hh = terrace_height(g, s, r);
    let rise = 20.0;
    let shade = |c: [u8; 3], f: f32| rgb(shade_u(c, f));
    let face = |uy: f32, v: f32| lift(w, uy, v);
    // gable ends where the terrace stops
    let open = |rr: i64| rr < 0 || rr >= ROWS as i64 || !g.houses[s][rr as usize].lot.is_house() || terrace_height(g, s, rr as usize) < hh;
    for (yy, nb) in [(y0, r as i64 + 1), (y1, r as i64 - 1)] {
        if !open(nb) { continue; }
        quad((back, yy), (w, yy), lift(w, yy, hh), lift(back, yy, hh), shade(wall, 0.78));
        tri(lift(back, yy, hh), lift(w, yy, hh), lift(ridge, yy, hh + rise), shade(wall, 0.78));
    }
    // the front wall
    quad((w, y0), (w, y1), face(y1, hh), face(y0, hh), rgb(wall));
    // the plinth: Nyboder's is tarred black, elsewhere the wall darkened
    if nyboder { quad((w, y0), (w, y1), face(y1, 4.0), face(y0, 4.0), rgb([38, 34, 32])); }
    else { quad((w, y0), (w, y1), face(y1, 3.0), face(y0, 3.0), shade(wall, 0.7)); }
    // windows: two floors either side of the door, white frames (Nyboder's green), shutters upstairs (Nyboder's on
    // every floor, a dusty raspberry)
    let frame = rgb(if nyboder { [58, 106, 70] } else { [246, 240, 228] });
    let shutter = if nyboder { [168, 70, 78] } else { SHUTTER };
    let lit_now = |i: usize| lit(g, s, (r * 4 + i) % (ROWS * 4) / 4) && scatter(k * 8 + i, 9) > 0.3;
    let floors: Vec<(f32, f32)> = if hh < 30.0 { vec![(7.0, 17.0)] } else {
        (0..((hh - 4.0) / 13.0) as i32).map(|f| (5.0 + f as f32 * 13.0, 13.0 + f as f32 * 13.0)).collect()
    };
    let shutters = matches!(district(side_of(s), r), District::Nyboder | District::IndreBy);
    for (fi, &(v0, v1)) in floors.iter().enumerate() {
        for (i, off) in [-40.0f32, -18.0, 18.0, 40.0].into_iter().enumerate() {
            if fi == 0 && off.abs() < 20.0 { continue; }
            let (a, b) = (dy + off - 7.0, dy + off + 7.0);
            if (fi > 0 || nyboder) && shutters {
                quad(face(a - 4.0, v0), face(a, v0), face(a, v1), face(a - 4.0, v1), rgb(shutter));
                quad(face(b, v0), face(b + 4.0, v0), face(b + 4.0, v1), face(b, v1), rgb(shutter));
            }
            quad(face(a - 1.0, v0 - 1.0), face(b + 1.0, v0 - 1.0), face(b + 1.0, v1 + 1.0), face(a - 1.0, v1 + 1.0), frame);
            let glass = if lit_now(i + fi * 4) { [255, 214, 120] } else { [70, 90, 116] };
            quad(face(a, v0), face(b, v0), face(b, v1), face(a, v1), rgb(glass));
            if fi > 0 && h.done && h.wave < 1.5 {
                let c = (1.0 - h.wave / 1.5).max(0.0);
                quad(face(a, v0), face(a + (b - a) * c * 0.5, v0), face(a + (b - a) * c * 0.5, v1), face(a, v1), rgb([236, 226, 206]));
            }
        }
    }
    // the door, open while the neighbour is out, and its brass letterbox
    let open_door = h.wave < 1.6 && h.done;
    quad(face(dy - 9.0, 0.0), face(dy + 9.0, 0.0), face(dy + 9.0, 20.0), face(dy - 9.0, 20.0), frame);
    quad(face(dy - 8.0, 0.0), face(dy + 8.0, 0.0), face(dy + 8.0, 19.0), face(dy - 8.0, 19.0),
        rgb(if open_door { [255, 230, 160] } else { door }));
    let flap = if h.flap < 0.35 { 1.0 - h.flap / 0.35 } else { 0.0 };
    let slot = if h.sub { GOLD } else { [120, 110, 96] };
    quad(face(dy - SLOT_HALF, 9.0 - flap * 2.0), face(dy + SLOT_HALF, 9.0 - flap * 2.0), face(dy + SLOT_HALF, 11.0), face(dy - SLOT_HALF, 11.0), rgb(slot));
    if h.sub && lit(g, s, r) {
        let (lx, ly) = face(dy - 13.0, 16.0);
        blip.fill_glow_circle(lx, ly, 4.0, rgba(LAMP, 0.9));
    }
    // the roof: red tiles in rows, the ridge, a chimney and a dormer or two
    let (e0, e1, r0, r1) = (face(y0, hh), face(y1, hh), lift(ridge, y0, hh + rise), lift(ridge, y1, hh + rise));
    quad(lift(back, y0, hh), lift(back, y1, hh), r1, r0, shade(roof, 0.72));
    quad(e0, e1, r1, r0, rgb(roof));
    for i in 1..7 {
        let f = i as f32 / 7.0;
        let (a, b) = (mix2(e0, r0, f), mix2(e1, r1, f));
        blip.draw_line(a.0, a.1, b.0, b.1, shade(roof, 0.82));
    }
    blip.draw_line(r0.0, r0.1, r1.0, r1.1, shade(roof, 0.6));
    blip.draw_line_ex(e0.0, e0.1, e1.0, e1.1, 2.0, shade(roof, 0.55));
    if scatter(k, 6) > 0.75 {
        // a dormer: a little white-framed window standing on the slope
        let u = y0 + LOT * (0.3 + scatter(k, 8) * 0.4);
        let (bx0, by0) = (mx(s, 46.0), hh + 8.0);
        let p = |uy: f32, v: f32| lift(bx0, uy, v);
        quad(p(u - 8.0, by0), p(u + 8.0, by0), p(u + 8.0, by0 + 12.0), p(u - 8.0, by0 + 12.0), frame);
        quad(p(u - 6.0, by0 + 2.0), p(u + 6.0, by0 + 2.0), p(u + 6.0, by0 + 10.0), p(u - 6.0, by0 + 10.0), rgb([80, 100, 126]));
        tri(lift(mx(s, 46.0), u - 9.0, by0 + 12.0), lift(mx(s, 46.0), u + 9.0, by0 + 12.0), lift(mx(s, 38.0), u, by0 + 18.0), shade(roof, 0.9));
    }
    let cu = y0 + 12.0 + scatter(k, 5) * (LOT - 30.0);
    block(mx(s, 24.0), cu, 5.0, 6.0, hh + rise - 6.0, hh + rise + 8.0, rgb([150, 70, 52]), rgb([60, 40, 38]));
    let cold = 1.0 - daylight(g);
    if cold > 0.2 {
        let (cx, cy) = lift(mx(s, 24.0), cu, hh + rise + 8.0);
        for i in 0..3 {
            let a = (t * 0.4 + i as f32 / 3.0 + scatter(k, 10)).fract();
            blip.fill_circle(cx + (a * 7.0 + r as f32).sin() * 3.0 - (s as f32 * 2.0 - 1.0) * a * 10.0, cy - a * 16.0, 3.0 + a * 5.0,
                rgba([236, 232, 240], 0.45 * cold * (1.0 - a)));
        }
    }
    // goodwill: two hearts over a subscriber's roof, one breaking as a paper is missed
    if h.sub || h.cancelled < 2.0 {
        let (hx, hy) = lift(w, dy, 27.0);
        for i in 0..2u8 {
            let full = i < h.hearts;
            let broke = !full && i == h.hearts && h.sad < 1.0;
            let c = if full { rgb(HEART) } else { rgba([60, 40, 50], 0.55) };
            let lift_y = if broke { h.sad * 8.0 } else { 0.0 };
            heart(blip, hx - 7.0 + i as f32 * 14.0, hy - lift_y, 5.0, if broke { rgba(HEART, 1.0 - h.sad) } else { c });
        }
        if h.joined < 1.6 && h.joined >= 0.0 { blip.draw_circle(hx, hy, 10.0 + h.joined * 14.0, 2.0, rgba(GOLD, 1.0 - h.joined / 1.6)); }
    }
}

pub(super) fn side_of(s: usize) -> Side { if s == 0 { Side::Left } else { Side::Right } }

pub(super) fn shade_u(c: [u8; 3], k: f32) -> [u8; 3] { [(c[0] as f32 * k) as u8, (c[1] as f32 * k) as u8, (c[2] as f32 * k) as u8] }

/// A Danish pennant on a garden flagpole, streaming with the wind: the wind's tell.
pub(super) fn draw_flag(blip: &Blip, g: &Game, s: usize, y: f32, t: f32) {
    let x = mx(s, WALL[0] + 26.0);
    blip.fill_circle(x, y, 2.5, rgb([240, 240, 240]));
    let dir = if g.wind() == 0.0 { 1.0 } else { -g.wind().signum() }; // screen y grows backward along the street
    let len = if g.wind() == 0.0 { 14.0 } else { 30.0 * (g.wind().abs() / 60.0) };
    for i in 0..10 {
        let k = i as f32 / 10.0;
        let wob = (t * 9.0 - k * 7.0).sin() * 3.0 * k;
        let w = 7.0 * (1.0 - k * 0.7);
        // red, with the white cross near the pole and the white tips of the tail
        let white = i == 1 || i == 9;
        blip.fill_rect(x - w / 2.0 + wob, y + dir * k * len, w, len / 10.0 + 1.0, rgb(if white { [250, 250, 250] } else { [200, 22, 40] }));
    }
}

/// Copenhagen's street lamps hang on wires strung across the street; lit while it is still dark.
pub fn draw_overhead(blip: &Blip, g: &Game, rows: &[usize]) {
    let glow = (0.75 - daylight(g)).max(0.0);
    for &r in rows {
        if r % 3 != 0 || matches!(g.houses[0][r].lot, Lot::Bridge | Lot::Cross | Lot::Palace) { continue; }
        let y = sy(r as f32 * LOT + 4.0);
        if !(HUD_H - 10.0..WIN_H as f32).contains(&y) { continue; }
        // the wires from the walls either side, sagging to the lamp, and the lamp's white globe under its cap
        let (lx, ly) = lift(240.0, y, 40.0);
        let (a, b) = (lift(WALL[0], y, 44.0), lift(WALL[1], y, 44.0));
        blip.draw_line_ex(a.0, a.1, lx, ly - 2.0, 1.5, rgba([26, 26, 32], 0.85));
        blip.draw_line_ex(lx, ly - 2.0, b.0, b.1, 1.5, rgba([26, 26, 32], 0.85));
        blip.fill_circle(240.0 + 4.0, y + 6.0, 7.0, rgba([20, 20, 30], 0.18));
        blip.fill_circle(lx, ly, 7.0, rgb(if glow > 0.0 { [255, 236, 190] } else { [236, 236, 232] }));
        blip.fill_circle(lx, ly - 2.0, 4.5, rgb([60, 62, 70]));
        if glow > 0.0 {
            blip.fill_glow_circle(lx, ly, 9.0, rgba([255, 224, 160], glow));
            blip.fill_circle(240.0, y + 2.0, 26.0, rgba([255, 220, 150], glow * 0.12));
        }
    }
}

/// Market day: bunting strung across the street.
pub(super) fn draw_bunting(blip: &Blip, g: &Game, r: usize, t: f32) {
    if !r.is_multiple_of(3) || g.houses[0][r].lot == Lot::Bridge { return; }
    let y = sy(r as f32 * LOT + 10.0);
    blip.draw_line(WALL[0], y, WALL[1], y, rgb([90, 80, 70]));
    let mut x = WALL[0] + 6.0;
    let mut i = 0;
    while x < WALL[1] - 6.0 {
        let c = [[230, 70, 70], [250, 210, 80], [90, 160, 220], [120, 200, 120]][i % 4];
        let sway = (t * 2.0 + i as f32).sin();
        for k in 0..6 { blip.fill_rect(x - 5.0 + k as f32, y + 1.0, 10.0 - k as f32 * 2.0 + 1.0, 1.5, rgb(c)); }
        blip.fill_rect(x - 4.0, y + 1.0, 8.0, 6.0 + sway, rgb(c));
        x += 18.0;
        i += 1;
    }
}
