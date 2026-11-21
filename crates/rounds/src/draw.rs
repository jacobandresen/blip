//! The town at dawn, drawn at runtime: terraced houses on both sides, the light warming as the round goes on,
//! the weather, the neighbours waking and the cargo bike.

use crate::game::*;
use crate::levels::*;
use blip::font::text_width;
use blip::{scatter, Blip, BlipColor, BLIP_BLACK, BLIP_WHITE};
use std::f32::consts::{PI, TAU};

pub fn rgb(c: [u8; 3]) -> BlipColor { BlipColor::from_rgba(c[0], c[1], c[2], 255) }
pub fn rgba(c: [u8; 3], a: f32) -> BlipColor { BlipColor { a, ..rgb(c) } }

const INK: [u8; 3] = [34, 26, 40];
const GRASS: [u8; 3] = [104, 160, 82];
const PAVE: [u8; 3] = [196, 186, 170];
const ROAD: [u8; 3] = [86, 90, 104];
const WATER: [u8; 3] = [64, 128, 150];
const STONE: [u8; 3] = [214, 204, 188];
const GOLD: [u8; 3] = [255, 214, 90];
const LAMP: [u8; 3] = [255, 214, 120];
const HEART: [u8; 3] = [236, 72, 96];

/// Copenhagen terraces: mostly Nyboder yellow, with the oxblood, pale yellow, burnt orange, pale blue and cream of the
/// streets around it.
const WALLS: [[u8; 3]; 9] = [
    [238, 184, 70], [232, 176, 64], [242, 192, 84], [186, 74, 56], [244, 216, 150], [218, 124, 74], [158, 186, 204],
    [238, 228, 204], [236, 180, 72],
];
/// Red tile, mostly; now and then slate.
const ROOFS: [[u8; 3]; 4] = [[186, 72, 46], [170, 62, 40], [198, 88, 58], [94, 96, 106]];
/// Bottle green, oxblood, near black and harbour blue.
const DOORS: [[u8; 3]; 4] = [[40, 92, 70], [124, 38, 36], [52, 44, 40], [36, 62, 98]];
/// Shutters on a Nyboder row are oxblood.
const SHUTTER: [u8; 3] = [140, 46, 40];
const COBBLE: [u8; 3] = [104, 104, 112];
const TRACK: [u8; 3] = [72, 74, 84];
const KERB: [u8; 3] = [190, 190, 194];
const CROSSING_BLUE: [u8; 3] = [52, 120, 200];
const CARS: [[u8; 3]; 5] = [[220, 70, 60], [70, 120, 200], [240, 200, 80], [90, 160, 110], [230, 230, 230]];

thread_local! {
    /// The street being drawn: the distance along it at the camera's anchor, and the anchor's x (where height leans
    /// away from).
    static VIEW: std::cell::Cell<(f32, f32)> = const { std::cell::Cell::new((0.0, 240.0)) };
    /// The sun's shadow, canal-pixels per unit of height (x, y): long toward the viewer at dawn, short by morning.
    static SUN: std::cell::Cell<(f32, f32)> = const { std::cell::Cell::new((-0.3, 0.8)) };
    /// How far the camera has turned the street being drawn.
    static TURNED: std::cell::Cell<f32> = const { std::cell::Cell::new(0.0) };
    /// What stands upright whatever the camera does (towers, statues, posts), put off until the streets are drawn.
    static STANDING: std::cell::RefCell<Vec<(f32, f32, Stand)>> = const { std::cell::RefCell::new(Vec::new()) };
}

/// Something that stands up off the street, drawn upright on the screen.
#[derive(Clone, Copy)]
enum Stand { Statue, Guard, Post(bool, Light), Wagon, Sign }

/// The low sun is ahead and a little to the right: shadows fall back toward the viewer and left, 2.4 px per unit of
/// height at dawn, 0.8 by full morning.
fn set_sun(day: f32) {
    let l = blip::lerp(2.4, 0.8, day.clamp(0.0, 1.0));
    SUN.with(|s| s.set((-0.35 * l, l)));
}

/// Where the shadow of a thing `h` units tall falls, from under it.
fn sun(h: f32) -> (f32, f32) {
    let (x, y) = SUN.with(|s| s.get());
    (x * h, y * h)
}

/// Draw the street from (`x`, `d`) in its own frame.
fn set_view(d: f32, x: f32) { VIEW.with(|v| v.set((d, x))); }

/// Put `what`, standing at (`x`, `y`) in the street's picture, aside to draw upright on the screen.
fn stand(x: f32, y: f32, what: Stand) {
    let (_, ax) = VIEW.with(|v| v.get());
    let ang = TURNED.with(|a| a.get());
    let (qx, qy) = (x - ax, y - BIKE_Y);
    let (s, c) = ang.sin_cos();
    let at = (240.0 + qx * c - qy * s, BIKE_Y + qx * s + qy * c);
    STANDING.with(|l| l.borrow_mut().push((at.0, at.1, what)));
}

/// Everything put aside, upright: farthest up the screen first.
fn draw_standing(blip: &Blip) {
    let mut list = STANDING.with(|l| std::mem::take(&mut *l.borrow_mut()));
    list.sort_by(|a, b| a.1.total_cmp(&b.1));
    for (x, y, what) in list {
        match what {
            Stand::Statue => frederik(blip, x, y),
            Stand::Guard => guard(blip, x, y, 0.0),
            Stand::Post(big, light) => light_post(blip, x, y, big, light),
            Stand::Wagon => polsevogn(blip, x, y),
            Stand::Sign => route_sign_at(blip, x, y),
        }
    }
}

/// The plain view: the bike's street, upright.
pub fn plain_view(blip: &Blip, g: &Game) {
    set_view(g.bike_d, 240.0);
    blip.set_offset(0.0, 0.0);
}

/// The street's screen y for a distance along it.
pub fn sy(d: f32) -> f32 { BIKE_Y - (d - VIEW.with(|v| v.get().0)) }

/// `x` given for the left side, mirrored onto `side`: the street is symmetric about its middle.
fn mx(side: usize, x: f32) -> f32 { if side == 0 { x } else { WIN_W as f32 - x } }

/// A rectangle given in left-side coordinates, drawn on `side`.
fn side_rect(blip: &Blip, side: usize, x: f32, y: f32, w: f32, h: f32, c: BlipColor) {
    let x = if side == 0 { x } else { WIN_W as f32 - x - w };
    blip.fill_rect(x, y, w, h, c);
}

fn heart(blip: &Blip, x: f32, y: f32, r: f32, c: BlipColor) {
    blip.fill_circle(x - r * 0.5, y, r * 0.6, c);
    blip.fill_circle(x + r * 0.5, y, r * 0.6, c);
    for i in 0..(r as i32 + 1) {
        let k = i as f32 / r;
        let w = r * 1.1 * (1.0 - k);
        blip.fill_rect(x - w, y + i as f32 * 0.9, w * 2.0, 1.2, c);
    }
}

/// Text centred on `cx`, with an outline so it reads over the street.
pub fn label(blip: &Blip, text: &str, cx: f32, y: f32, sz: f32, c: BlipColor) {
    blip.draw_text_outlined(text, (cx - text_width(text, sz) / 2.0).round(), y.round(), sz, c, rgb(INK));
}

// ---- the street -----------------------------------------------------------

/// How bright the morning is: dawn at the start of the round, full morning by its end.
pub fn daylight(g: &Game) -> f32 {
    match g.state {
        State::Title | State::Over => 0.35,
        State::Canal => 1.0,
        _ => g.progress(),
    }
}

/// The whole street scene: the old street and, near the turn, the new one, each through its own camera turned as
/// the bike's heading turns; the bike upright on top.
pub fn draw_world(blip: &Blip, g: &Game, t: f32) {
    set_sun(daylight(g));
    blip.clear(rgb(GRASS));
    let (cx, cd, ca) = g.camera();
    let near_turn = (g.bike_d - JD).abs() < 700.0;
    let mut bike_at = (g.bike_x, BIKE_Y);
    for seg in [0u8, 1] {
        if seg == 0 && g.turned && !near_turn || seg == 1 && !g.turned && !near_turn { continue; }
        let (ax, ad, ang) = if seg == 0 { (cx, cd, ca) } else {
            let (x, d) = world_to_new_street(cx, cd);
            (x, d, ca - std::f32::consts::FRAC_PI_2)
        };
        set_view(ad, ax);
        TURNED.with(|a| a.set(ang));
        blip.set_view_pivot((ax, BIKE_Y), (240.0, BIKE_Y), ang);
        draw_street(blip, g, t, seg);
        if seg == g.seg() {
            for p in g.papers.iter().filter(|p| p.landed.is_none()) { draw_paper(blip, p); }
            draw_bits(blip, g);
            // where the bike is on the screen, through this camera
            let q = (g.bike_x - ax, BIKE_Y - (g.bike_d - ad) - BIKE_Y);
            let (s, c) = ang.sin_cos();
            bike_at = (240.0 + q.0 * c - q.1 * s, BIKE_Y + q.0 * s + q.1 * c);
        }
    }
    blip.set_offset(0.0, 0.0);
    draw_standing(blip);
    draw_bike(blip, g, t, bike_at.0, bike_at.1);
    // back to the bike's own street for what is drawn over the picture
    set_view(g.bike_d, 240.0);
}

/// One street's ground, houses, traffic and lamps, in its own frame (0 the old street, 1 the new).
pub fn draw_street(blip: &Blip, g: &Game, t: f32, seg: u8) {
    let at = VIEW.with(|v| v.get().0);
    let (lo, hi) = (at - 520.0, at + 620.0);
    let first_new = NEW_STREET_ROW;
    let mine = |r: usize| if seg == 0 { r < first_new } else { r >= first_new };
    let visible: Vec<usize> = (0..ROWS).filter(|&r| { let d = r as f32 * LOT; mine(r) && d + LOT > lo && d < hi }).collect();
    // the old street carries on past the turn (Store Kongensgade), closed for roadworks
    let ghost: Vec<usize> = if seg == 0 { (first_new..first_new + 5).filter(|&r| { let d = r as f32 * LOT; d + LOT > lo && d < hi }).collect() } else { Vec::new() };
    for &r in &ghost { draw_ground(blip, g, r); }
    for &r in &visible { draw_ground(blip, g, r); }
    for &r in &visible { for s in 0..2 { draw_lot(blip, g, s, r, t); } }
    for &r in &visible {
        if !matches!(g.houses[0][r].lot, Lot::Bridge | Lot::Cross | Lot::Palace)
            && !(r > 0 && g.houses[0][r - 1].lot == Lot::Palace) { draw_furniture(blip, g, r, sy((r + 1) as f32 * LOT)); }
    }
    if seg == 0 && (visible.contains(&TURN_ROW) || visible.contains(&(TURN_ROW + 1))) { mouth(blip); }
    let flat = |h: &&Hazard| matches!(h.kind, Hz::Puddle(_) | Hz::Sprinkler);
    let shown = |h: &&Hazard| h.seg == seg && (seg == 0 || g.turned);
    for h in g.hazards.iter().filter(flat).filter(shown) { draw_hazard(blip, g, h, t); }
    if seg == g.seg() { for p in g.papers.iter().filter(|p| p.landed.is_some()) { draw_paper(blip, p); } }
    for h in g.hazards.iter().filter(|h| !flat(h)).filter(shown) { draw_hazard(blip, g, h, t); }
    for &r in &ghost {
        for s in 0..2 {
            if s == 1 || r >= first_new + 2 { ghost_building(blip, s, r); }
        }
    }
    if seg == 0 { roadworks(blip); }
    let mut order: Vec<usize> = visible.clone();
    order.sort_by(|a, b| {
        let far = |r: usize| (sy(door_d(r)) - vanish().1).abs();
        far(*b).total_cmp(&far(*a))
    });
    for &r in &order {
        for s in 0..2 {
            if g.houses[s][r].lot == Lot::Corner { continue; }
            match g.houses[s][r].lot {
                Lot::House { .. } => draw_building(blip, g, s, r, t),
                Lot::Bakery => draw_shop(blip, g, s, r),
                // drawn in the houses' own perspective, so they stand up off the street as the houses do
                Lot::Tower => rundetaarn_up(blip, g, s, r),
                Lot::Church => marmorkirken_up(blip, s, r),
                Lot::Palace => palace(blip, s, r),
                _ => {}
            }
        }
    }
    for &(r, side) in STOPS {
        if visible.contains(&r) { bus_stop(blip, g, r, side, t); }
    }
    for &r in JUNCTIONS.iter() {
        if visible.contains(&r) { traffic_lights(g, r); }
    }
    if visible.contains(&STATUE_ROW) || visible.contains(&(STATUE_ROW - 1)) { stand(240.0, sy(statue_d()) + STATUE_HALF, Stand::Statue); }
    for &r in &visible { pedestrians(blip, g, r, t); }
    if seg == 0 && (visible.contains(&(TURN_ROW - 1)) || visible.contains(&TURN_ROW)) { route_sign(); }
    draw_overhead(blip, g, &visible);
    if g.weather() == Weather::Market { for &r in &visible { draw_bunting(blip, g, r, t); } }
}

/// People on the pavements in the early morning: joggers, someone walking a dog, a bag of bread from the bakery,
/// a commuter with a coffee. Only scenery: they keep to the pavement.
fn pedestrians(blip: &Blip, g: &Game, r: usize, t: f32) {
    if matches!(g.houses[0][r].lot, Lot::Bridge | Lot::Cross | Lot::Palace) { return; }
    for s in 0..2 {
        let k = r * 2 + s;
        if scatter(k, 61) > 0.25 { continue; }
        let kind = (scatter(k, 62) * 4.0) as usize;
        let speed = [70.0, 26.0, 30.0, 34.0][kind] * if scatter(k, 63) > 0.5 { 1.0 } else { -1.0 };
        let d = r as f32 * LOT + (scatter(k, 64) * LOT + t * speed).rem_euclid(LOT);
        let x = mx(s, 137.0 + (scatter(k, 65) - 0.5) * 10.0);
        let y = sy(d);
        let coat = if g.rain() { [[250, 210, 40], [40, 60, 110], [60, 140, 90]][k % 3] } else { [[200, 70, 70], [60, 90, 150], [230, 220, 200], [70, 120, 90], [40, 40, 50]][k % 5] };
        let swing = (t * if kind == 0 { 12.0 } else { 6.0 } + k as f32).sin() * 2.0;
        person(blip, x, y, coat, [[80, 60, 40], [230, 200, 120], [30, 24, 20]][k % 3], swing);
        match kind {
            1 => {
                // a dog on a lead, trotting ahead
                let dy = y - speed.signum() * 14.0;
                blip.draw_line(x, y, x + 3.0, dy, rgb([60, 40, 30]));
                blip.fill_rect(x + 1.0, dy - 4.0, 5.0, 8.0, rgb([[150, 100, 60], [40, 36, 34], [220, 210, 190]][k % 3]));
            }
            2 => blip.fill_rect(x + 5.0, y + 1.0, 5.0, 7.0, rgb([230, 214, 170])),
            3 => blip.fill_rect(x - 9.0, y - 1.0, 3.0, 4.0, rgb([250, 250, 250])),
            _ => {}
        }
        if g.rain() { blip.fill_circle(x, y - 3.0, 9.0, rgba([[40, 40, 50], [200, 40, 50], [40, 90, 160]][k % 3], 0.95)); }
    }
}

/// A Movia city bus in height: yellow sides with red ends and a black band of windows, the roof's grey units, the
/// windscreen and orange destination board at the front, the line on it.
#[allow(clippy::too_many_arguments)]
fn bus(blip: &Blip, x: f32, y: f32, up: bool, long: bool, line: &str, doors: bool, blink: bool, t: f32) {
    let (hw, hl, hh) = (13.0, if long { 60.0 } else { 44.0 }, 28.0);
    let yellow = [246, 194, 34];
    let red = [206, 40, 40];
    let (ox, oy) = sun(6.0);
    blip.fill_rect(x - hw + ox, y - hl + oy, hw * 2.0, hl * 2.0, rgba([20, 20, 30], 0.3));
    let (front, rear) = if up { (y - hl, y + hl) } else { (y + hl, y - hl) };
    // the side toward the middle of the street
    let sx = if x < vanish().0 { x + hw } else { x - hw };
    let at = |yy: f32, v: f32| lift(sx, yy, v);
    quad(at(y - hl, 0.0), at(y + hl, 0.0), at(y + hl, hh), at(y - hl, hh), rgb(yellow));
    let fdir = (front - rear).signum();
    for (a, b) in [(front, front - fdir * 16.0), (rear, rear + fdir * 10.0)] {
        quad(at(a, 0.0), at(b, 0.0), at(b, 11.0), at(a, 11.0), rgb(red));
    }
    quad(at(front - fdir * 6.0, 13.0), at(rear + fdir * 4.0, 13.0), at(rear + fdir * 4.0, 25.0), at(front - fdir * 6.0, 25.0), rgb([30, 32, 38]));
    for k in [0.18, 0.55] {
        let dy = front - fdir * (2.0 * hl) * k;
        let c = if doors { [250, 236, 180] } else { [60, 64, 72] };
        quad(at(dy - 5.0, 1.0), at(dy + 5.0, 1.0), at(dy + 5.0, 24.0), at(dy - 5.0, 24.0), rgb(c));
    }
    if long {
        let j = y;
        quad(at(j - 4.0, 0.0), at(j + 4.0, 0.0), at(j + 4.0, hh), at(j - 4.0, hh), rgb([40, 40, 44]));
    }
    // the end toward the camera: the front with its windscreen and destination board, or the back
    let end = if y + hl < vanish().1 { Some(y + hl) } else if y - hl > vanish().1 { Some(y - hl) } else { None };
    if let Some(e) = end {
        let ea = |xx: f32, v: f32| lift(xx, e, v);
        let is_front = (e - front).abs() < 1.0;
        quad(ea(x - hw, 0.0), ea(x + hw, 0.0), ea(x + hw, hh), ea(x - hw, hh), rgb(if is_front { red } else { yellow }));
        if is_front {
            quad(ea(x - hw + 2.0, 9.0), ea(x + hw - 2.0, 9.0), ea(x + hw - 2.0, 23.0), ea(x - hw + 2.0, 23.0), rgb([24, 26, 32]));
            quad(ea(x - hw + 3.0, 23.5), ea(x + hw - 3.0, 23.5), ea(x + hw - 3.0, 27.0), ea(x - hw + 3.0, 27.0), rgb([20, 16, 12]));
            let (lx, ly) = ea(x, 26.5);
            blip.draw_text(line, lx - 5.0, ly - 3.0, 1.0, rgb([255, 160, 40]));
            for xx in [x - hw + 3.0, x + hw - 3.0] { let (a, b) = ea(xx, 4.0); blip.fill_circle(a, b, 2.0, rgb([255, 250, 220])); }
        } else {
            quad(ea(x - hw + 4.0, 14.0), ea(x + hw - 4.0, 14.0), ea(x + hw - 4.0, 22.0), ea(x - hw + 4.0, 22.0), rgb([30, 32, 38]));
            for xx in [x - hw + 3.0, x + hw - 3.0] { let (a, b) = ea(xx, 6.0); blip.fill_circle(a, b, 2.0, rgb([230, 40, 40])); }
        }
    }
    // the roof: pale grey, its cooling units, a stripe of the livery's red
    let r = [(x - hw, y - hl), (x + hw, y - hl), (x + hw, y + hl), (x - hw, y + hl)].map(|p| lift(p.0, p.1, hh));
    quad(r[0], r[1], r[2], r[3], rgb(yellow));
    let inset = [(x - hw + 3.0, y - hl + 3.0), (x + hw - 3.0, y - hl + 3.0), (x + hw - 3.0, y + hl - 3.0), (x - hw + 3.0, y + hl - 3.0)].map(|p| lift(p.0, p.1, hh));
    quad(inset[0], inset[1], inset[2], inset[3], rgb([214, 214, 210]));
    let cap = [(x - hw, front), (x + hw, front), (x + hw, front - fdir * 8.0), (x - hw, front - fdir * 8.0)].map(|p| lift(p.0, p.1, hh));
    quad(cap[0], cap[1], cap[2], cap[3], rgb(red));
    block(x, y - fdir * hl * 0.45, hw * 0.6, 7.0, hh, hh + 4.0, rgb([150, 152, 156]), rgb([176, 178, 182]));
    block(x, y + fdir * hl * 0.25, hw * 0.6, 9.0, hh, hh + 4.0, rgb([150, 152, 156]), rgb([176, 178, 182]));
    // pulling out: the indicator on the road side, blinking
    if blink && (t * 3.0).fract() < 0.5 {
        let side = if x > vanish().0 { x - hw } else { x + hw };
        let (a, b) = lift(side, front, 8.0);
        blip.fill_glow_circle(a, b, 3.0, rgb([255, 170, 40]));
    }
}

/// A bus stop: the glass shelter on the pavement, its yellow sign, and people waiting (fewer once the bus has been).
fn bus_stop(blip: &Blip, g: &Game, row: usize, side: Side, t: f32) {
    let s = side.index();
    let d = door_d(row) + 30.0;
    let y = sy(d);
    if !(-80.0..WIN_H as f32 + 60.0).contains(&y) { return; }
    let x = mx(s, 136.0);
    let (ox, oy) = sun(3.5);
    blip.fill_rect(x - 9.0 + ox, y - 24.0 + oy, 18.0, 48.0, rgba([20, 20, 30], 0.25));
    block(x, y, 8.0, 24.0, 0.0, 22.0, rgba([170, 210, 230], 0.55), rgb([60, 64, 72]));
    let (px, py) = lift(mx(s, 146.0), y - 34.0, 26.0);
    blip.draw_line_ex(mx(s, 146.0), y - 34.0, px, py, 2.0, rgb([60, 60, 66]));
    blip.fill_rect(px - 5.0, py - 7.0, 10.0, 8.0, rgb([250, 206, 40]));
    blip.fill_rect(px - 3.0, py - 5.0, 6.0, 4.0, rgb([30, 30, 34]));
    let gone = g.hazards.iter().any(|h| h.kind == Hz::Bus && h.side == side && (h.d - door_d(row)).abs() < 200.0 && h.phase == AWAY);
    let n = if gone { 1 } else { 4 };
    for i in 0..n {
        // in a loose queue along the pavement, either side of the shelter
        let ox = mx(s, if i % 2 == 0 { 129.0 } else { 145.0 });
        let oy = y - 40.0 + i as f32 * 12.0 + scatter(row * 9 + i, 52) * 4.0;
        let coat = [[60, 80, 140], [200, 90, 60], [90, 140, 90], [230, 220, 200], [40, 40, 50], [170, 60, 120]][i % 6];
        person(blip, ox, oy, coat, [[80, 60, 40], [230, 200, 120], [30, 24, 20]][i % 3], ((t * 1.2 + i as f32).sin() * 0.8).round());
    }
}

/// A poelsevogn: the hot dog wagon, white with a red roof, its hatch open and the sausages on the grill.
fn polsevogn(blip: &Blip, cx: f32, base: f32) {
    blip.fill_rect(cx - 22.0 + 3.0, base - 4.0, 44.0, 8.0, rgba([20, 20, 30], 0.3));
    blip.fill_rect(cx - 22.0, base - 30.0, 44.0, 30.0, rgb([244, 240, 232]));
    blip.fill_rect(cx - 24.0, base - 38.0, 48.0, 9.0, rgb([200, 30, 40]));
    blip.fill_rect(cx - 16.0, base - 26.0, 32.0, 12.0, rgb([60, 50, 44]));
    for i in 0..4 { blip.fill_rect(cx - 13.0 + i as f32 * 7.0, base - 21.0, 5.0, 2.0, rgb([200, 90, 60])); }
    blip.fill_rect(cx - 22.0, base - 14.0, 44.0, 3.0, rgb([200, 30, 40]));
    blip.fill_circle(cx - 14.0, base, 4.0, rgb([40, 40, 44]));
    blip.fill_circle(cx + 14.0, base, 4.0, rgb([40, 40, 44]));
}

/// The lights at a junction, on posts at the corners: for the bike's street and its own small cycle lights.
fn traffic_lights(g: &Game, r: usize) {
    let (light, _) = g.light(r);
    let near = sy(r as f32 * LOT + 8.0);
    let far = sy((r + 1) as f32 * LOT - 8.0);
    for (x, y, big) in [(CURB[1] + 8.0, near, true), (CURB[0] - 8.0, near, false), (CURB[0] - 8.0, far, true), (CURB[1] + 8.0, far, false)] {
        stand(x, y, Stand::Post(big, light));
    }
}

/// One post of a junction's lights, its three lamps.
fn light_post(blip: &Blip, x: f32, y: f32, big: bool, light: Light) {
    {
        blip.fill_rect(x - 1.0, y - 30.0, 3.0, 30.0, rgb([40, 40, 46]));
        let (w, hgt) = if big { (10.0, 26.0) } else { (8.0, 20.0) };
        blip.fill_rect(x - w / 2.0, y - 30.0 - hgt, w, hgt, rgb([28, 28, 32]));
        let lamp = hgt / 3.0;
        for (i, (on, c)) in [(light == Light::Red, [240, 40, 40]), (light == Light::Amber, [250, 180, 30]), (light == Light::Green, [60, 220, 90])].into_iter().enumerate() {
            let ly = y - 30.0 - hgt + lamp * (i as f32 + 0.5);
            if on { blip.fill_glow_circle(x, ly, lamp * 0.42, rgb(c)); } else { blip.fill_circle(x, ly, lamp * 0.35, rgb([60, 56, 56])); }
        }
    }
}

/// Frederiksgade's mouth on the left of Store Kongensgade: its road and pavements running off to the left, the
/// blue cycle crossing over the old street's track, the corners.
fn mouth(blip: &Blip) {
    let (top, bot) = (sy(JD + MOUTH_HALF + 20.0), sy(JD - MOUTH_HALF - 20.0));
    blip.fill_rect(-400.0, top, CURB[0] + 400.0, bot - top, rgb(PAVE));
    let (rt, rb) = (sy(JD + MOUTH_HALF), sy(JD - MOUTH_HALF));
    blip.fill_rect(-400.0, rt, CURB[0] + 400.0, rb - rt, rgb([84, 84, 92]));
    // its cycle tracks, its centre line, the kerbs
    for (a, b) in [(rt, rt + 26.0), (rb - 26.0, rb)] { blip.fill_rect(-400.0, a, CURB[0] + 400.0, b - a, rgb(TRACK)); }
    for k in 0..12 { blip.fill_rect(CURB[0] - 40.0 - k as f32 * 40.0, (rt + rb) / 2.0 - 1.5, 20.0, 3.0, rgba([240, 236, 220], 0.7)); }
    // the old street's left cycle track crosses the mouth on blue
    blip.fill_rect(CURB[0], rt, 26.0, rb - rt, rgb(CROSSING_BLUE));
    for k in 0..8 { blip.fill_rect(CURB[0] + 2.0, rt + 6.0 + k as f32 * 22.0, 22.0, 8.0, rgba([255, 255, 255], 0.85)); }
    blip.fill_rect(CURB[0] - 3.0, rt, 3.0, rb - rt, rgb(KERB));
}

/// A house of the old street past the turn: only scenery, Store Kongensgade carrying on.
fn ghost_building(blip: &Blip, s: usize, r: usize) {
    let y = sy((r + 1) as f32 * LOT);
    side_rect(blip, s, WALL[0], y, FENCE[0] - WALL[0], LOT, rgb([178, 174, 168]));
    side_rect(blip, s, FENCE[0] - 2.0, y, 2.0, LOT, rgb([36, 36, 42]));
    let (y0, y1) = (y, sy(r as f32 * LOT));
    let (w, back, ridge) = (mx(s, WALL[0]), mx(s, -30.0), mx(s, 30.0));
    let (wall, roof) = house_colours(s, 27, r * 2 + s);
    let hh = 46.0;
    let face = |uy: f32, v: f32| lift(w, uy, v);
    quad((w, y0), (w, y1), face(y1, hh), face(y0, hh), rgb(wall));
    let frame = rgb([246, 240, 228]);
    for f in 0..3 {
        let (v0, v1) = (5.0 + f as f32 * 13.0, 13.0 + f as f32 * 13.0);
        for off in [-40.0f32, -14.0, 14.0, 40.0] {
            let dy = (y0 + y1) / 2.0 + off;
            quad(face(dy - 7.0, v0), face(dy + 7.0, v0), face(dy + 7.0, v1), face(dy - 7.0, v1), frame);
            quad(face(dy - 6.0, v0 + 1.0), face(dy + 6.0, v0 + 1.0), face(dy + 6.0, v1 - 1.0), face(dy - 6.0, v1 - 1.0), rgb([70, 90, 116]));
        }
    }
    let (e0, e1, r0, r1) = (face(y0, hh), face(y1, hh), lift(ridge, y0, hh + 20.0), lift(ridge, y1, hh + 20.0));
    quad(lift(back, y0, hh), lift(back, y1, hh), r1, r0, rgb(shade_u(roof, 0.72)));
    quad(e0, e1, r1, r0, rgb(roof));
}

/// Store Kongensgade closed past the turn: red and white barriers on their feet, amber lamps, the sign, the dug road
/// and a digger.
fn roadworks(blip: &Blip) {
    let y = sy(WORKS_D);
    if !(-60.0..WIN_H as f32 + 60.0).contains(&y) { return; }
    let t = blip::now();
    blip.fill_rect(CURB[0] + 4.0, y - 90.0, CURB[1] - CURB[0] - 8.0, 80.0, rgb([120, 96, 70]));
    for k in 0..6 { blip.fill_circle(CURB[0] + 20.0 + k as f32 * 28.0, y - 40.0 - (k % 2) as f32 * 20.0, 8.0, rgb([100, 80, 58])); }
    block(270.0, y - 50.0, 14.0, 20.0, 0.0, 18.0, rgb([230, 180, 30]), rgb([246, 200, 50]));
    block(270.0, y - 50.0, 8.0, 8.0, 18.0, 30.0, rgb([110, 140, 170]), rgb([240, 196, 40]));
    let (a0, a1) = (lift(258.0, y - 60.0, 26.0), lift(220.0, y - 80.0, 34.0));
    blip.draw_line_ex(a0.0, a0.1, a1.0, a1.1, 4.0, rgb([230, 180, 30]));
    // the barriers
    let mut x = CURB[0] + 2.0;
    while x < CURB[1] - 2.0 {
        let (p0, p1) = (lift(x, y, 0.0), lift(x, y, 12.0));
        blip.draw_line_ex(p0.0, p0.1, p1.0, p1.1, 2.0, rgb([60, 60, 66]));
        for i in 0..4 {
            let (a, b) = (lift(x + i as f32 * 8.0, y, 12.0), lift(x + i as f32 * 8.0 + 8.0, y, 12.0));
            blip.draw_line_ex(a.0, a.1, b.0, b.1, 5.0, rgb(if i % 2 == 0 { [214, 40, 40] } else { [250, 250, 250] }));
        }
        let lamp = lift(x + 16.0, y, 16.0);
        if (t * 2.0 + x * 0.05).fract() < 0.5 { blip.fill_glow_circle(lamp.0, lamp.1, 3.0, rgb([255, 170, 40])); }
        x += 32.0;
    }
    let s = lift(300.0, y + 6.0, 30.0);
    blip.fill_rect(s.0 - 34.0, s.1 - 10.0, 68.0, 16.0, rgb([250, 250, 250]));
    blip.draw_rect(s.0 - 34.0, s.1 - 10.0, 68.0, 16.0, rgb([200, 30, 30]));
    blip.draw_text("VEJARBEJDE", s.0 - 30.0, s.1 - 6.0, 1.0, rgb([20, 20, 24]));
}

/// Ahead of the left turn: Copenhagen's blue direction sign on its posts at the corner, the arrow to the left.
fn route_sign() {
    stand(WIN_W as f32 - 14.0 - 98.0, sy(JD - MOUTH_HALF - 40.0), Stand::Sign);
}

/// The sign itself, its posts standing at (`cx`, `base`).
fn route_sign_at(blip: &Blip, cx: f32, base: f32) {
    let (w, h) = (196.0, 26.0);
    let x0 = cx - w / 2.0;
    let top = base - 44.0;
    for px in [x0 + 20.0, x0 + w - 20.0] { blip.fill_rect(px - 1.5, top + h, 3.0, base - top - h, rgb([70, 70, 76])); }
    blip.fill_rect(x0 + 3.0, top + 3.0, w, h, rgba([20, 20, 30], 0.3));
    blip.fill_rect(x0, top, w, h, rgb([30, 80, 170]));
    blip.draw_rect(x0 + 2.0, top + 2.0, w - 4.0, h - 4.0, rgb([240, 244, 250]));
    let (ax, ay) = (x0 + 16.0, top + h / 2.0);
    blip.draw_line_ex(ax, ay, ax + 14.0, ay, 3.0, rgb([255, 255, 255]));
    blip.draw_line_ex(ax, ay, ax + 6.0, ay - 6.0, 3.0, rgb([255, 255, 255]));
    blip.draw_line_ex(ax, ay, ax + 6.0, ay + 6.0, 3.0, rgb([255, 255, 255]));
    blip.draw_text(TURN_INTO, x0 + 36.0, top + 7.0, 2.0, rgb([255, 255, 255]));
}

// ---- landmarks ------------------------------------------------------------------
// Drawn as people know them: standing up, from the front.

/// Granite setts in front of a landmark.
fn forecourt(blip: &Blip, s: usize, y: f32, r: usize) {
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
fn plaza(blip: &Blip, r: usize, y: f32) {
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
fn up(p: (f32, f32), h: f32) -> (f32, f32) { lift(p.0, p.1, h) }

/// How a wall facing `n` (a unit vector on the ground) is lit by the low sun ahead and to the right: 0.62 to 1.08.
fn sunlit(n: (f32, f32)) -> f32 {
    let s = (0.33f32, -0.94f32);
    0.85 + 0.23 * (n.0 * s.0 + n.1 * s.1)
}

/// A round wall from `h0` to `h1` on a circle of radius `rad` round `c`, seen from above as the houses are: only the
/// side facing the viewer (toward the vanishing point), each strip lit by the sun; `strip` may draw on each strip.
fn round_wall(c: (f32, f32), rad: f32, h0: f32, h1: f32, col: [u8; 3], strip: &mut dyn FnMut(f32, f32, f32)) {
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
fn round_top(c: (f32, f32), rad: f32, h: f32, col: BlipColor) {
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
fn rundetaarn_up(blip: &Blip, g: &Game, s: usize, r: usize) {
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
fn marmorkirken_up(blip: &Blip, s: usize, r: usize) {
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
fn palace(blip: &Blip, s: usize, r: usize) {
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
fn frederik(blip: &Blip, cx: f32, base: f32) {
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

/// The bakery standing up: a cream shop front with its big window and BAGERI over it, under a slate roof.
fn draw_shop(blip: &Blip, g: &Game, s: usize, r: usize) {
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
fn draw_ground(blip: &Blip, g: &Game, r: usize) {
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
    // a manhole cover, patches in the asphalt
    if false {
        let mx_ = 196.0 + scatter(r, 82) * 90.0;
        let my = y + 20.0 + scatter(r, 83) * 70.0;
        blip.fill_circle(mx_, my, 7.0, rgb([70, 72, 84]));
        blip.draw_circle(mx_, my, 7.0, 1.0, rgb([110, 112, 124]));
    }
    // the centre line, painted over the setts
    for k in 0..3 { blip.fill_rect(239.0, y + k as f32 * 38.0 + 6.0, 3.0, 20.0, rgba([240, 236, 220], 0.7)); }
    // a bike painted in each lane now and then
    if r % 3 == 1 {
        for s in 0..2 {
            let cx = BIKE_LANE[s];
            let cy = y + LOT / 2.0;
            let c = rgba([236, 232, 220], 0.55);
            blip.draw_circle(cx, cy - 6.0, 3.5, 1.0, c);
            blip.draw_circle(cx, cy + 6.0, 3.5, 1.0, c);
            blip.draw_line(cx, cy - 6.0, cx, cy + 6.0, c);
        }
    }
}

/// What stands on the pavement where two lots meet: a street tree or a rack of parked bikes.
fn draw_furniture(blip: &Blip, g: &Game, r: usize, y: f32) {
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
fn lit(g: &Game, s: usize, r: usize) -> bool {
    daylight(g) < 0.25 + 0.65 * scatter(r * 2 + s, 7) || g.weather() == Weather::Fog || g.rain()
}

fn draw_lot(blip: &Blip, g: &Game, s: usize, r: usize, t: f32) {
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

fn draw_park(blip: &Blip, s: usize, r: usize, y: f32) {
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
fn cemetery(blip: &Blip, s: usize, r: usize, y: f32) {
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
fn amaliehaven(blip: &Blip, g: &Game, s: usize, r: usize, y: f32) {
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

fn draw_bakery(blip: &Blip, g: &Game, s: usize, r: usize, y: f32, t: f32) {
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
fn draw_house(blip: &Blip, g: &Game, s: usize, r: usize, y: f32, dy: f32, drive: bool, t: f32) {
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

// ---- height ---------------------------------------------------------------
// The street is seen from above the bike: anything with height leans away from the middle of the street and from
// the bike, as it would through a camera. Terraces show their fronts; towers stand up as they pass.

/// The point straight under the camera, where nothing leans: under the bike.
fn vanish() -> (f32, f32) { (VIEW.with(|v| v.get().1), BIKE_Y + 30.0) }
/// How far a point leans per pixel of height, across the street and (a long lens: less) along it.
const LEAN: f32 = 0.0068;
const LEAN_ALONG: f32 = 0.0022;

/// Where a point `h` up from (`x`, `y`) is drawn.
pub fn lift(x: f32, y: f32, h: f32) -> (f32, f32) {
    // The lean is the camera's: more across the screen than up it, whichever way the street has been turned.
    let v = vanish();
    let (dx, dy) = (x - v.0, y - v.1);
    let (s, c) = TURNED.with(|a| a.get()).sin_cos();
    let (sx, sy_) = ((dx * c - dy * s) * h * LEAN, (dx * s + dy * c) * h * LEAN_ALONG);
    (x + sx * c + sy_ * s, y - sx * s + sy_ * c)
}

fn tri(a: (f32, f32), b: (f32, f32), c: (f32, f32), col: BlipColor) {
    use blip::macroquad::math::vec2;
    blip::macroquad::shapes::draw_triangle(vec2(a.0, a.1), vec2(b.0, b.1), vec2(c.0, c.1), col);
}

fn quad(a: (f32, f32), b: (f32, f32), c: (f32, f32), d: (f32, f32), col: BlipColor) {
    tri(a, b, c, col);
    tri(a, c, d, col);
}

fn mix2(a: (f32, f32), b: (f32, f32), k: f32) -> (f32, f32) { (a.0 + (b.0 - a.0) * k, a.1 + (b.1 - a.1) * k) }

/// A box standing on the ground, `hw` x `hh` either side of (`x`, `y`), `lo` to `hi` up: its walls that face the
/// camera, then its top.
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
fn terrace_height(g: &Game, s: usize, r: usize) -> f32 {
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
fn house_colours(s: usize, r: usize, k: usize) -> ([u8; 3], [u8; 3]) {
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
fn draw_building(blip: &Blip, g: &Game, s: usize, r: usize, t: f32) {
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
    if lit(g, s, r) && h.sub || h.sub && g.weather() == Weather::Fog {
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

fn side_of(s: usize) -> Side { if s == 0 { Side::Left } else { Side::Right } }

fn shade_u(c: [u8; 3], k: f32) -> [u8; 3] { [(c[0] as f32 * k) as u8, (c[1] as f32 * k) as u8, (c[2] as f32 * k) as u8] }

/// A Danish pennant on a garden flagpole, streaming with the wind: the wind's tell.
fn draw_flag(blip: &Blip, g: &Game, s: usize, y: f32, t: f32) {
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
fn draw_bunting(blip: &Blip, g: &Game, r: usize, t: f32) {
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

// ---- what is on the street -------------------------------------------------

fn person(blip: &Blip, x: f32, y: f32, coat: [u8; 3], hair: [u8; 3], swing: f32) {
    let (ox, oy) = sun(2.5);
    blip.fill_circle(x + ox, y + oy, 7.0, rgba([20, 20, 30], 0.25));
    blip.fill_rect(x - 8.0, y - 2.0 + swing, 4.0, 4.0, rgb(coat));
    blip.fill_rect(x + 4.0, y - 2.0 - swing, 4.0, 4.0, rgb(coat));
    blip.fill_circle(x, y, 6.0, rgb(coat));
    blip.fill_circle(x, y - 1.0, 4.0, rgb(hair));
}

/// Are bike lamps on: in the blue hour before the sun is up, and in fog (the law asks for lights in the dark).
pub fn lamps_on(g: &Game) -> bool {
    daylight(g) < 0.45 || g.weather() == Weather::Fog
}

/// A bicycle as Copenhagen rides them, seen from above: tyres showing past the mudguards, a thin frame, the crank
/// and pedals, a saddle, black swept-back bars with grips and a bell, a rack (or a wicker basket on the front), and
/// lamps, lit white ahead and red behind when `lit`. `dir` is +1 riding down the screen, -1 up; `wheelbase` is
/// axle to axle; `bars` how far ahead of `y` the bars are; `steer` turns the front wheel and bars.
#[allow(clippy::too_many_arguments)]
fn bicycle(blip: &Blip, x: f32, y: f32, dir: f32, wheelbase: f32, bars: f32, steer: f32, frame: [u8; 3], basket: bool, lit: bool) {
    let (front, rear) = (y + dir * wheelbase / 2.0, y - dir * wheelbase / 2.0);
    let bars_y = y + dir * bars;
    // the shadow: the wheels' line and the bars, a little to the side
    let shade = rgba([16, 16, 24], 0.28);
    let (ox, oy) = sun(2.0);
    blip.fill_rect(x - 1.5 + ox, y - wheelbase / 2.0 - 6.0 + oy, 3.0, wheelbase + 12.0, shade);
    blip.fill_rect(x + steer - 6.5 + ox * 1.5, bars_y + oy * 1.5, 13.0, 2.0, shade);
    let tyre = rgb([20, 20, 24]);
    let guard = rgb(shade_u(frame, 0.75));
    for (wx, wy) in [(x, rear), (x + steer, front)] {
        blip.fill_rect(wx - 1.5, wy - 6.5, 3.0, 13.0, tyre);
        blip.fill_rect(wx - 1.0, wy - 4.0, 2.0, 8.0, guard);
    }
    blip.draw_line_ex(x, rear + dir * 3.0, x + steer * 0.6, bars_y, 2.5, rgb(frame));
    // the crank and pedals, turning
    let crank = y - dir * 1.0;
    blip.fill_circle(x, crank, 1.8, rgb([150, 150, 156]));
    // rack and rear lamp, or a basket in front
    blip.fill_rect(x - 2.0, rear - 3.5, 4.0, 7.0, rgb([46, 46, 52]));
    blip.fill_rect(x - 1.0, rear - 3.0, 2.0, 6.0, rgb([90, 90, 98]));
    if basket {
        blip.fill_rect(x + steer - 4.0, front - 4.0, 8.0, 7.0, rgb([150, 108, 56]));
        blip.fill_rect(x + steer - 3.0, front - 3.0, 6.0, 5.0, rgb([196, 150, 88]));
        blip.fill_rect(x + steer - 3.0, front - 1.0, 6.0, 1.0, rgb([150, 108, 56]));
    }
    // the saddle, brown leather, nose forward
    blip.fill_rect(x - 1.5, y - dir * 5.0 - 2.0, 3.0, 4.0, rgb([70, 46, 30]));
    blip.fill_rect(x - 0.5, y - dir * 3.0 - 1.0, 1.0, 2.0, rgb([70, 46, 30]));
    // black bars swept back to dark grips, the bell's silver dome on the left
    let ink = rgb([18, 18, 22]);
    let (l, r) = (x + steer - 6.5, x + steer + 6.5);
    blip.draw_line_ex(l + 2.5, bars_y, r - 2.5, bars_y, 1.5, ink);
    blip.draw_line_ex(l + 2.5, bars_y, l, bars_y - dir * 2.5, 1.5, ink);
    blip.draw_line_ex(r - 2.5, bars_y, r, bars_y - dir * 2.5, 1.5, ink);
    blip.fill_circle(x + steer - 3.0, bars_y + dir * 0.5, 1.3, rgb([222, 226, 232]));
    // lamps: white on the front mudguard, red on the rack
    let (fx, fy, ry) = (x + steer, front + dir * 6.0, rear - dir * 4.5);
    if lit {
        blip.fill_glow_circle(fx, fy, 2.0, rgb([255, 252, 230]));
        blip.fill_glow_circle(x, ry, 1.7, rgb([255, 40, 40]));
    } else {
        blip.fill_rect(fx - 1.0, fy - 1.0, 2.0, 2.0, rgb([196, 196, 190]));
        blip.fill_rect(x - 1.0, ry - 1.0, 2.0, 2.0, rgb([130, 30, 30]));
    }
}

/// A rider sitting up on a bicycle, seen from above as Copenhagen rides: knees rising either side of the frame in
/// turn, the coat's back over the saddle, shoulders square, arms forward to the grips, the head (a helmet or hair)
/// on top. `bars` is how far ahead of `y` the bars are.
#[allow(clippy::too_many_arguments)]
fn rider_on(blip: &Blip, x: f32, y: f32, dir: f32, bars: f32, steer: f32, coat: [u8; 3], head: [u8; 3], t: f32, s: f32) {
    let pedal = (t * 9.0).sin();
    let legs = rgb([44, 48, 64]);
    // knees: the one coming up shows further forward
    blip.fill_rect(x - 4.0 * s, y + dir * (1.5 + pedal * 2.0) * s - 1.5 * s, 2.5 * s, 3.0 * s, legs);
    blip.fill_rect(x + 1.5 * s, y + dir * (1.5 - pedal * 2.0) * s - 1.5 * s, 2.5 * s, 3.0 * s, legs);
    let back = rgb(shade_u(coat, 0.8));
    blip.fill_rect(x - 3.0 * s, y - dir * 4.0 * s - 2.0 * s, 6.0 * s, 5.0 * s, back);
    let shoulders = y - dir * 0.5 * s;
    let grips = y + dir * (bars - 2.0);
    let sleeve = rgb(shade_u(coat, 0.9));
    blip.draw_line_ex(x - 3.5 * s, shoulders, x + steer - 5.5, grips, 2.0 * s, sleeve);
    blip.draw_line_ex(x + 3.5 * s, shoulders, x + steer + 5.5, grips, 2.0 * s, sleeve);
    blip.fill_rect(x + steer - 6.5, grips - 1.0, 2.0, 2.0, rgb([226, 180, 140]));
    blip.fill_rect(x + steer + 4.5, grips - 1.0, 2.0, 2.0, rgb([226, 180, 140]));
    // square shoulders, the head on top, a touch forward
    blip.fill_rect(x - 4.5 * s, shoulders - 2.0 * s, 9.0 * s, 4.0 * s, rgb(coat));
    blip.fill_circle(x - 4.0 * s, shoulders, 2.0 * s, rgb(coat));
    blip.fill_circle(x + 4.0 * s, shoulders, 2.0 * s, rgb(coat));
    let hy = shoulders + dir * 1.0 * s;
    blip.fill_circle(x + steer * 0.2, hy, 2.9 * s, rgb(head));
    blip.fill_rect(x + steer * 0.2 - 1.0 * s, hy - 2.5 * s, 2.0 * s, 1.0 * s, rgba([255, 255, 255], 0.35));
}

fn car(blip: &Blip, x: f32, y: f32, across: bool, body: [u8; 3]) {
    // a small box in height: the body, then the glasshouse and the roof
    let (hw, hh) = if across { (20.0, 11.0) } else { (11.0, 20.0) };
    let (ox, oy) = sun(4.0);
    blip.fill_rect(x - hw + ox, y - hh + oy, hw * 2.0, hh * 2.0, rgba([20, 20, 30], 0.3));
    block(x, y, hw, hh, 0.0, 10.0, rgb(shade_u(body, 0.85)), rgb(body));
    let (cw, ch) = if across { (hw * 0.5, hh - 1.5) } else { (hw - 1.5, hh * 0.5) };
    block(x, y, cw, ch, 10.0, 17.0, rgb([110, 140, 170]), rgb(shade_u(body, 0.92)));
}

fn draw_hazard(blip: &Blip, g: &Game, h: &Hazard, t: f32) {
    if h.gone { return; }
    let y = sy(h.d);
    if y < -80.0 || y > WIN_H as f32 + 80.0 { return; }
    let x = h.x;
    let blink = (t * 6.0).fract() < 0.5;
    match h.kind {
        Hz::Car if h.bus => {
            // a city bus coming: its headlights on the setts ahead of it
            for k in 0..6 {
                let k = k as f32;
                blip.fill_rect(x - 10.0 - k * 2.0, y + 42.0 + k * 10.0, 20.0 + k * 4.0, 10.0, rgba([255, 240, 170], 0.16 - k * 0.02));
            }
            let line = if district(Side::Left, row_at(h.d).unwrap_or(0)) == District::Norrebro { "5C" } else { "1A" };
            bus(blip, x, y, false, (h.d as usize).is_multiple_of(3), line, false, false, t);
        }
        Hz::Bus => {
            let up = h.side == Side::Right;
            let line = if district(Side::Left, row_at(h.d).unwrap_or(0)) == District::Norrebro { "5C" } else { "1A" };
            // passengers stepping over the cycle track between the door and the kerb, on the ground beside the bus
            if h.phase == TELL {
                let lane = BIKE_LANE[h.side.index()];
                let out = (lane - x).signum();
                for p in 0..2 {
                    let k = ((h.t * 0.8 + p as f32 * 0.5).fract() * 2.0 - 1.0).abs();
                    let px = blip::lerp(x + out * 22.0, lane + out * 14.0, k);
                    person(blip, px, y - 20.0 + p as f32 * 12.0, [[60, 80, 140], [200, 90, 60]][p], [80, 60, 40], (t * 6.0 + p as f32).sin() * 2.0);
                }
            }
            bus(blip, x, y, up, false, line, h.phase == TELL, h.phase == TELL && h.t > BUS_DWELL - 1.4, t);
        }
        Hz::Tourist => {
            // a white rental e-bike, the big screen glowing on its bars, a backpack on the rider
            let (_, staring, _) = tourist_phase(h.t + h.beat);
            let staring = staring && !h.guide;
            let up = h.side == Side::Right;
            let dir = if up { -1.0 } else { 1.0 };
            let lean = if h.guide { 0.0 } else { (tourist_x(h.home, h.side, h.t + h.beat + 0.1) - x) * 0.6 };
            bicycle(blip, x, y, dir, 26.0, 9.0, lean, [236, 236, 232], false, lamps_on(g));
            // the e-bike's battery on the down tube
            blip.fill_rect(x - 1.5, y + dir * 2.0 - 3.0, 3.0, 6.0, rgb([70, 70, 78]));
            rider_on(blip, x, y, dir, 9.0, lean, [250, 120, 60], [240, 210, 150], t * 0.8 + h.beat, 1.0);
            // the backpack on the rider's back, and the big tablet on the bars, glowing when stared at
            blip.fill_rect(x - 3.5, y - dir * 4.0 - 2.5, 7.0, 5.0, rgb([60, 130, 90]));
            let glow = if staring && (t * 8.0).fract() < 0.6 { [160, 230, 255] } else { [70, 110, 140] };
            let sy0 = y + dir * 11.5;
            if staring { blip.fill_glow_circle(x + lean, sy0, 6.0, rgba(glow, 0.25)); }
            blip.fill_rect(x - 4.0 + lean, sy0 - 2.0, 8.0, 4.0, rgb([30, 30, 34]));
            blip.fill_rect(x - 3.0 + lean, sy0 - 1.0, 6.0, 2.0, rgb(glow));
            if staring && !h.guide { label(blip, "?", x + 10.0, y - 26.0, 2.0, rgb([255, 255, 255])); }
            if h.guide {
                // the guide's flag, held high so the flock can follow it
                let (fx, fy) = lift(x + 4.0, y, 34.0);
                blip.draw_line(x + 4.0, y, fx, fy, rgb([60, 60, 66]));
                blip.fill_rect(fx, fy, 10.0, 7.0, rgb([200, 22, 40]));
                blip.fill_rect(fx + 3.0, fy, 2.0, 7.0, rgb([250, 250, 250]));
                blip.fill_rect(fx, fy + 3.0, 10.0, 1.5, rgb([250, 250, 250]));
            }
        }
        Hz::Car if h.oncoming => {
            // headlights reach down the street ahead of it: you see the light before the car
            if h.phase >= GO || y > -200.0 {
                for k in 0..6 {
                    let k = k as f32;
                    blip.fill_rect(x - 9.0 - k * 2.0, y + 22.0 + k * 10.0, 18.0 + k * 4.0, 10.0, rgba([255, 240, 170], 0.16 - k * 0.02));
                }
            }
            let taxi = (h.d as usize / 110) % 3 == 1;
            car(blip, x, y, false, if taxi { [36, 36, 40] } else { CARS[(h.home as usize + h.d as usize / 7) % CARS.len()] });
            blip.fill_rect(x - 8.0, y + 18.0, 4.0, 2.0, rgb([255, 250, 210]));
            blip.fill_rect(x + 4.0, y + 18.0, 4.0, 2.0, rgb([255, 250, 210]));
            if taxi {
                // a Copenhagen taxi: the TAXA sign lit on the roof
                let (tx, ty) = lift(x, y, 19.0);
                blip.fill_rect(tx - 7.0, ty - 3.0, 14.0, 6.0, rgb([250, 214, 70]));
                blip.draw_text("TAXA", tx - 6.0, ty - 2.0, 0.5, rgb([40, 30, 10]));
            }
        }
        Hz::Car => {
            let across = h.phase < PAUSE || h.phase == PAUSE && h.t < 0.25;
            let body = CARS[(h.d as usize / 13) % CARS.len()];
            car(blip, x, y, across, body);
            if across && (h.phase == TELL || h.phase == GO) {
                // the reverse lights: white and glowing at the end nearest the street
                let rx = x - h.side.sign() * 20.0;
                for dy in [-7.0, 7.0] { blip.fill_glow_circle(rx, y + dy, 3.5, rgba([255, 255, 240], if blink { 1.0 } else { 0.6 })); }
            }
        }
        Hz::Dog => {
            if h.phase == WAIT || h.phase == TELL {
                // behind the gate: an ear and a wagging tail over the hedge
                if h.phase == TELL { label(blip, "!", x, y - 22.0, 2.0, rgb([255, 220, 90])); }
                blip.fill_circle(x, y, 5.0, rgb([150, 100, 60]));
                return;
            }
            let wag = (t * 18.0).sin() * 4.0;
            let (ox, oy) = sun(2.5);
            blip.fill_circle(x + ox, y + oy, 8.0, rgba([20, 20, 30], 0.25));
            blip.fill_rect(x - 5.0, y - 7.0, 10.0, 15.0, rgb([170, 112, 64]));
            blip.fill_circle(x, y - 9.0, 5.5, rgb([186, 126, 74]));
            blip.fill_rect(x - 6.0, y - 13.0, 3.0, 5.0, rgb([110, 70, 40]));
            blip.fill_rect(x + 3.0, y - 13.0, 3.0, 5.0, rgb([110, 70, 40]));
            blip.draw_line_ex(x, y + 8.0, x + wag, y + 14.0, 2.0, rgb([170, 112, 64]));
            if h.phase == AWAY && h.fetch.is_none() || h.phase == HOME && h.t < 3.0 && h.fetch.is_none() && h.touched {
                blip.fill_rect(x - 5.0, y - 15.0, 10.0, 3.0, rgb([250, 248, 236]));
            }
        }
        Hz::Sprinkler => {
            blip.fill_circle(x, y, 3.0, rgb([90, 90, 100]));
            if h.phase == TELL {
                for k in 0..4 {
                    let a = (t * 3.0 + k as f32 / 4.0).fract();
                    blip.fill_circle(x + (k as f32 - 1.5) * 3.0, y - a * 10.0, 1.5, rgba([200, 230, 255], 1.0 - a));
                }
            }
            if h.phase == GO {
                // a fan of droplets over the pavement and the bike lane
                let reach = (CURB[0] + SPRAY_REACH) - FENCE[0] + 14.0;
                for k in 0..40 {
                    let a = scatter(k, 31);
                    let dist = 10.0 + ((t * 1.6 + scatter(k, 32)).fract()) * reach;
                    let ang = (a - 0.5) * 1.6;
                    let px = x - h.side.sign() * dist * ang.cos();
                    let py = y + dist * ang.sin() * 0.9;
                    blip.fill_rect(px, py, 2.0, 2.0, rgba([180, 220, 255], 0.75));
                }
            }
        }
        Hz::Blader => {
            let swing = (t * 8.0).sin() * 3.0;
            blip.fill_rect(x - 4.0, y + 6.0, 2.0, 6.0, rgb([60, 60, 70]));
            blip.fill_rect(x + 2.0, y + 6.0, 2.0, 6.0, rgb([60, 60, 70]));
            person(blip, x + swing * 0.5, y, [80, 200, 210], [250, 120, 80], swing);
        }
        Hz::Van => {
            // a white delivery van in height: the cargo box, the cab in front, hazard lights when it is parked
            let up = h.side == Side::Right;
            let cab = if up { y - 22.0 } else { y + 22.0 };
            let (ox, oy) = sun(5.0);
            blip.fill_rect(x - 13.0 + ox, y - 30.0 + oy, 26.0, 60.0, rgba([20, 20, 30], 0.3));
            block(x, y + if up { 6.0 } else { -6.0 }, 13.0, 24.0, 0.0, 30.0, rgb([214, 214, 210]), rgb([240, 240, 236]));
            block(x, cab, 12.0, 8.0, 0.0, 20.0, rgb([110, 140, 170]), rgb([236, 236, 232]));
            if h.side == Side::Left && blink {
                for (dx, dy) in [(-12.0, -29.0), (10.0, -29.0), (-12.0, 27.0), (10.0, 27.0)] {
                    blip.fill_glow_circle(x + dx + 1.0, y + dy + 1.0, 3.0, rgb([255, 170, 40]));
                }
            }
        }
        Hz::Puddle(_) => {
            let big = if g.rain() { 1.25 } else { 1.0 };
            for (k, a) in [(1.0, 0.75), (0.7, 0.5)] {
                let (rw, rh) = (22.0 * big * k, 14.0 * big * k);
                for i in 0..(rh as i32) {
                    let f = i as f32 / rh;
                    let w = rw * (1.0 - (2.0 * f - 1.0).powi(2)).sqrt();
                    blip.fill_rect(x - w, y - rh / 2.0 + i as f32, w * 2.0, 1.0, rgba([96, 130, 160], a));
                }
            }
            let ripple = (t * 1.5).fract();
            blip.draw_circle(x + 4.0, y, 3.0 + ripple * 9.0, 1.0, rgba([220, 240, 255], 0.5 * (1.0 - ripple)));
        }
        Hz::Pigeons(_) => {
            for k in 0..5 {
                let (ox, oy) = ((scatter(k, 41) - 0.5) * 34.0, (scatter(k, 42) - 0.5) * 26.0);
                if h.phase == GO {
                    let up = h.t * 120.0;
                    let flap = (t * 20.0 + k as f32).sin() * 5.0;
                    let px = x + ox * (1.0 + h.t * 2.0) + (k as f32 - 2.0) * up * 0.4;
                    let py = y + oy - up;
                    blip.fill_rect(px - 6.0, py + flap * 0.2, 12.0, 2.0, rgb([150, 150, 165]));
                    blip.fill_circle(px, py, 3.0, rgb([120, 120, 135]));
                } else {
                    let peck = ((t * 3.0 + k as f32).sin() > 0.6) as i32 as f32 * 2.0;
                    blip.fill_circle(x + ox, y + oy, 4.0, rgb([136, 136, 150]));
                    blip.fill_circle(x + ox, y + oy - 4.0 + peck, 2.5, rgb([90, 110, 120]));
                }
            }
        }
        Hz::Cart => {
            let (ox, oy) = sun(3.0);
            blip.fill_rect(x - 12.0 + ox, y - 14.0 + oy, 24.0, 28.0, rgba([20, 20, 30], 0.3));
            blip.fill_rect(x - 12.0, y - 14.0, 24.0, 28.0, rgb([200, 60, 50]));
            for k in 0..4 { blip.fill_rect(x - 9.0, y - 11.0 + k as f32 * 6.0, 18.0, 4.0, rgb([248, 244, 232])); }
            blip.fill_circle(x - 12.0, y + 10.0, 3.0, rgb([40, 40, 40]));
            blip.fill_circle(x + 12.0, y + 10.0, 3.0, rgb([40, 40, 40]));
            let bob = (t * 4.0).sin() * 2.0;
            label(blip, "+10", x, y - 30.0 + bob, 2.0, rgb(GOLD));
        }
        Hz::Stall => {
            let c = [[220, 70, 70], [70, 140, 210], [240, 180, 60], [90, 170, 110]][(h.d as usize / 11) % 4];
            let (ox, oy) = sun(4.0);
            blip.fill_rect(x - 25.0 + ox, y - 32.0 + oy, 50.0, 64.0, rgba([20, 20, 30], 0.3));
            for k in 0..8 {
                let col = if k % 2 == 0 { c } else { [250, 246, 236] };
                blip.fill_rect(x - 25.0, y - 32.0 + k as f32 * 8.0, 50.0, 8.0, rgb(col));
            }
            for k in 0..6 {
                let fx = x - 18.0 + (k % 3) as f32 * 18.0;
                let fy = y - 16.0 + (k / 3) as f32 * 26.0;
                blip.fill_circle(fx, fy, 5.0, rgb([[240, 120, 40], [220, 40, 60], [120, 190, 70]][k % 3]));
            }
        }
        Hz::CrossCar => {
            let body = if h.bus { [244, 196, 40] } else { CARS[(h.d as usize + h.home as usize / 3) % CARS.len()] };
            car(blip, x, y, true, body);
            let front = x + if h.side == Side::Left { 20.0 } else { -20.0 };
            blip.fill_rect(front - 1.0, y - 8.0, 2.0, 4.0, rgb([255, 250, 210]));
            blip.fill_rect(front - 1.0, y + 4.0, 2.0, 4.0, rgb([255, 250, 210]));
        }
        Hz::Cyclist => {
            // a commuter on a Copenhagen bike, upright, now and then a cargo bike with a child in the box
            let up = h.side == Side::Right;
            let dir = if up { -1.0 } else { 1.0 };
            let pick = |n: usize, m: usize| (h.d as usize / n) % m;
            let cargo = pick(17, 3) == 0;
            let coat = if g.rain() { [[250, 210, 40], [40, 60, 110], [230, 60, 60], [60, 140, 90]][pick(5, 4)] }
                else { [[60, 70, 90], [180, 60, 60], [230, 220, 200], [70, 120, 90]][pick(5, 4)] };
            // frames: black, racing green, cream, sky blue, the classic red of a Danish city bike
            let frame = [[34, 34, 38], [40, 90, 60], [226, 214, 186], [110, 160, 200], [176, 40, 40]][pick(7, 5)];
            // half wear a helmet (white, black or orange), half ride in their hair
            let head = [[240, 240, 236], [90, 60, 40], [30, 30, 34], [230, 200, 120], [236, 120, 40], [40, 30, 30]][pick(3, 6)];
            let lit = lamps_on(g);
            if cargo {
                // a long-john cargo bike: the wooden box between the rider and the small front wheel, a child in it
                bicycle(blip, x, y - dir * 4.0, dir, 36.0, 5.0, 0.0, frame, false, lit);
                let bx = y + dir * 9.0;
                blip.fill_rect(x - 7.0, bx - 6.0, 14.0, 12.0, rgb([150, 104, 60]));
                blip.fill_rect(x - 6.0, bx - 5.0, 12.0, 10.0, rgb([188, 140, 84]));
                for k in [-2.5, 0.5, 3.5] { blip.fill_rect(x - 6.0, bx + k - 0.5, 12.0, 1.0, rgb([150, 104, 60])); }
                blip.fill_circle(x, bx, 3.0, rgb([230, 120, 90]));
                blip.fill_circle(x, bx - dir * 0.5, 2.0, rgb([240, 200, 140]));
                rider_on(blip, x, y - dir * 4.0, dir, 5.0, 0.0, coat, head, t + h.d * 0.01, 1.0);
            } else {
                bicycle(blip, x, y, dir, 26.0, 9.0, 0.0, frame, pick(11, 3) == 0, lit);
                rider_on(blip, x, y, dir, 9.0, 0.0, coat, head, t + h.d * 0.01, 1.0);
            }
        }
        Hz::Walker => {
            let swing = (t * 6.0).sin() * 2.0;
            let coat = [[150, 80, 160], [60, 120, 90], [200, 120, 60]][(h.d as usize / 9) % 3];
            person(blip, x, y, coat, [90, 60, 40], swing);
            blip.fill_rect(x + 6.0, y + 2.0, 5.0, 6.0, rgb([230, 210, 160]));
            if g.rain() { blip.fill_circle(x, y - 3.0, 10.0, rgb([[40, 40, 50], [200, 40, 50], [40, 90, 160]][(h.d as usize / 7) % 3])); }
        }
    }
}

// ---- papers and the bike -----------------------------------------------------

pub fn draw_paper(blip: &Blip, p: &Paper) {
    let (x, d, hgt) = p.at();
    let y = sy(d);
    let fade = p.landed.map_or(1.0, |(t, _)| (2.5 - t).clamp(0.0, 1.0));
    if fade <= 0.0 || p.landed.is_some_and(|(t, _)| t >= 9.0) { return; }
    blip.fill_rect(x - 5.0, y - 2.0, 10.0, 4.0, rgba([20, 20, 30], 0.3 * fade));
    let lift = hgt * 16.0;
    let spin = p.t * 22.0;
    let (cx, cy) = (x, y - lift);
    let (dx, dy) = (spin.cos() * 6.0, spin.sin() * 3.0);
    let c = rgba([250, 248, 238], fade);
    blip.draw_line_ex(cx - dx, cy - dy, cx + dx, cy + dy, 5.0, c);
    blip.draw_line_ex(cx - dx * 0.3, cy - dy * 0.3, cx + dx * 0.3, cy + dy * 0.3, 5.5, rgba([200, 60, 60], fade));
}

/// The player's riders are drawn this much larger than the town's, so the bike is found at a glance.
const PLAYER_SIZE: f32 = 1.3;

pub fn draw_bike(blip: &Blip, g: &Game, t: f32, x: f32, y: f32) {
    if g.state == State::Crash {
        // over it goes: the bike on its side, papers out, the rider sitting up with stars
        let k = (g.clock / 0.4).min(1.0);
        let dir = g.crash_dir;
        blip.fill_rect(x - 22.0, y - 6.0, 44.0, 12.0, rgba([20, 20, 30], 0.3));
        blip.fill_rect(x - 20.0, y - 4.0, 40.0, 8.0, rgb([60, 70, 90]));
        blip.fill_rect(x - 24.0, y - 9.0, 16.0, 18.0, rgb([176, 120, 70]));
        let (rx, ry) = (x + dir * 26.0 * k, y + 6.0);
        person(blip, rx, ry, [250, 200, 60], [220, 60, 60], 0.0);
        for i in 0..3 {
            let a = t * 5.0 + i as f32 * TAU / 3.0;
            blip.fill_circle(rx + a.cos() * 10.0, ry - 12.0 + a.sin() * 4.0, 2.0, rgb(GOLD));
        }
        return;
    }
    if g.safe > 0.0 && (t * 12.0).fract() < 0.5 { return; }
    let lean = (g.steer_v / STEER_V) * 3.0;
    let tandem = g.two_player;
    let lit = lamps_on(g);
    // a long-john: the small front wheel, the box ahead of the bars, the rider (two on the tandem) sitting up behind
    let rear = if tandem { 28.0 } else { 10.0 };
    let (front, centre) = (-34.0, (rear - 34.0) / 2.0);
    bicycle(blip, x, y + centre, -1.0, rear - front, 7.0 + centre, lean, [40, 110, 160], false, lit);
    let rain = g.rain();
    let coat1 = if rain { [250, 210, 40] } else { [236, 110, 60] };
    let arm = |cd: f32| if cd > THROW_GAP - 0.14 { 8.0 } else { 0.0 };
    let (al, ar) = (arm(g.throw_cd[0]), arm(g.throw_cd[1]));
    // the box: a plywood tub with a dark rim, the day's papers stacked in it
    let (bx, by) = (x + lean, y - 18.0);
    let (ox, oy) = sun(2.0);
    blip.fill_rect(bx - 10.0 + ox, by - 10.0 + oy, 20.0, 20.0, rgba([16, 16, 24], 0.28));
    blip.fill_rect(bx - 10.0, by - 10.0, 20.0, 20.0, rgb([120, 80, 44]));
    blip.fill_rect(bx - 9.0, by - 9.0, 18.0, 18.0, rgb([186, 136, 80]));
    for k in [-4.5, 0.5, 5.5] { blip.fill_rect(bx - 9.0, by + k - 0.5, 18.0, 1.0, rgb([160, 112, 64])); }
    let shown = g.stock.clamp(0, 6);
    for i in 0..shown {
        let (px, py) = (bx - 5.0 + (i % 2) as f32 * 10.0, by - 6.0 + (i / 2) as f32 * 5.0);
        blip.fill_rect(px - 4.0, py, 8.0, 4.0, rgb([248, 246, 236]));
        blip.fill_rect(px - 1.0, py, 2.0, 4.0, rgb([200, 60, 60]));
    }
    // the riders; a throwing arm reaches out to its side
    let throw = |rx: f32, ry: f32, coat: [u8; 3], l: f32, r: f32| {
        if l > 0.0 { blip.draw_line_ex(rx - 3.5, ry, rx - 4.0 - l, ry - 3.0, 2.0, rgb(coat)); }
        if r > 0.0 { blip.draw_line_ex(rx + 3.5, ry, rx + 4.0 + r, ry - 3.0, 2.0, rgb(coat)); }
    };
    if tandem {
        let coat2 = if rain { [120, 220, 120] } else { [80, 170, 110] };
        rider_on(blip, x, y + 20.0, -1.0, 9.0, 0.0, coat2, [250, 250, 250], t + 0.3, PLAYER_SIZE);
        throw(x, y + 20.0, coat2, 0.0, ar);
        rider_on(blip, x + lean * 0.5, y + 2.0, -1.0, 7.0, lean * 0.5, coat1, [220, 60, 60], t, PLAYER_SIZE);
        throw(x + lean * 0.5, y + 2.0, coat1, al, 0.0);
    } else {
        rider_on(blip, x + lean * 0.5, y + 2.0, -1.0, 7.0, lean * 0.5, coat1, [220, 60, 60], t, PLAYER_SIZE);
        throw(x + lean * 0.5, y + 2.0, coat1, al, ar);
    }
}

pub fn draw_bits(blip: &Blip, g: &Game) {
    for b in &g.bits {
        let a = 1.0 - b.t / b.life;
        blip.fill_rect(b.x - b.size / 2.0, sy(b.d) - b.size / 2.0, b.size, b.size, rgba(b.color, a));
    }
}

/// The points and words over the street, after the light so they read in any weather.
pub fn draw_pops(blip: &Blip, g: &Game) {
    for p in &g.pops {
        let a = (1.0 - p.t).clamp(0.0, 1.0);
        label(blip, &p.text, p.x.clamp(60.0, WIN_W as f32 - 60.0), sy(p.d), 2.0, rgba(p.color, a));
    }
}

// ---- weather and light ------------------------------------------------------

pub fn draw_weather(blip: &Blip, g: &Game, t: f32) {
    let w = WIN_W as f32;
    let top = HUD_H;
    let bottom = WIN_H as f32 - BAR_H;
    match g.weather() {
        Weather::Rain if g.state != State::Canal => {
            blip.fill_rect(0.0, top, w, bottom - top, rgba([40, 60, 90], 0.18));
            for k in 0..70 {
                let x = scatter(k, 51) * w;
                let y = top + ((scatter(k, 52) + t * 1.8) % 1.0) * (bottom - top);
                blip.draw_line(x, y, x - 3.0, y + 12.0, rgba([200, 220, 245], 0.45));
            }
        }
        Weather::Fog if g.state != State::Canal => {
            // the street ahead fades into white; only the lit doorways show through it early
            let mut y = top;
            while y < BIKE_Y - 60.0 {
                let a = ((BIKE_Y - 110.0 - y) / 200.0).clamp(0.0, 0.88);
                blip.fill_rect(0.0, y, w, 6.0, rgba([226, 230, 232], a));
                y += 6.0;
            }
            for s in 0..2 {
                for r in 0..ROWS {
                    let h = &g.houses[s][r];
                    let dy = sy(door_d(r));
                    if !h.sub || dy < top - 20.0 || dy > BIKE_Y { continue; }
                    blip.fill_glow_circle(mx(s, WALL[0] + 3.0), dy - 16.0, 6.0, rgba(LAMP, 0.9));
                }
            }
        }
        Weather::Wind(_) if g.state != State::Canal => {
            let dir = -g.wind().signum();
            for k in 0..14 {
                let x = scatter(k, 61) * w + (t * 3.0 + k as f32).sin() * 12.0;
                let y = top + ((scatter(k, 62) + dir * t * 0.5).rem_euclid(1.0)) * (bottom - top);
                let c = [[220, 140, 50], [200, 90, 50], [150, 170, 60]][k % 3];
                let spin = (t * 6.0 + k as f32).sin();
                blip.fill_rect(x - 3.0 * spin.abs() - 1.0, y - 2.0, 6.0 * spin.abs() + 2.0, 4.0, rgb(c));
            }
        }
        _ => {}
    }
    // the low sun ahead and to the right leaves the right-hand houses' fronts and their pavement in shade, a band
    // that draws back to the houses as the sun climbs (not while the view turns at the corner)
    let k = 1.0 - daylight(g);
    if k > 0.05 && TURNED.with(|a| a.get()).abs() < 0.01 && g.state != State::Canal {
        let edge = CURB[1] + 40.0 - 70.0 * k;
        blip.fill_rect(edge, top, w - edge, bottom - top, rgba([24, 26, 52], 0.20 * k));
        blip.fill_rect(edge - 4.0, top, 4.0, bottom - top, rgba([24, 26, 52], 0.10 * k));
    }
    // dawn: a cool blue hour warming to a golden morning, the low sun ahead up the street
    if k > 0.01 {
        let mut y = top;
        while y < bottom {
            let f = (y - top) / (bottom - top);
            blip.fill_rect(0.0, y, w, 10.0, rgba([40, 36, 96], k * (0.10 + 0.14 * f)));
            blip.fill_rect(0.0, y, w, 10.0, rgba([255, 168, 96], k * 0.16 * (1.0 - f).powi(2)));
            y += 10.0;
        }
    }
}

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

const NYHAVN: [[u8; 3]; 8] = [
    [206, 62, 48], [240, 184, 60], [64, 116, 186], [232, 122, 48], [238, 228, 204], [118, 168, 120], [204, 92, 112],
    [246, 210, 120],
];

/// One quay's houses: narrow and tall, each its own colour, white windows, a gable or a cornice on top.
fn nyhavn_row(g: &Game, s: usize) {
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
fn draw_mark(blip: &Blip, tg: &Target, t: f32) {
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
                for j in 0..2 { blip.fill_circle(cx - 5.0 + j as f32 * 10.0, cy - 30.0 + i as f32 * 12.0, 3.0, rgb([[230, 190, 150], [120, 80, 50], [240, 220, 160]][(i + j) % 3])); }
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
fn anchor(blip: &Blip, cx: f32, base: f32) {
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
fn vor_frelsers(blip: &Blip, cx: f32, base: f32, t: f32) {
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

// ---- screens ------------------------------------------------------------------

/// A weather picture for the day card and the bar: the day's tell, drawn.
pub fn weather_icon(blip: &Blip, weather: Weather, x: f32, y: f32, k: f32, t: f32) {
    match weather {
        Weather::Clear => {
            blip.fill_sunburst(x, y, 12, t * 0.3, 16.0 * k, [rgba(GOLD, 0.8), rgba(GOLD, 0.0)]);
            blip.fill_circle(x, y, 9.0 * k, rgb([255, 200, 70]));
        }
        Weather::Wind(_) => {
            for i in 0..3 {
                let yy = y - 8.0 * k + i as f32 * 8.0 * k;
                let off = ((t * 2.0 + i as f32) % 1.0) * 4.0;
                blip.fill_rect(x - 14.0 * k + off, yy, (22.0 - i as f32 * 4.0) * k, 2.0 * k.max(1.0), rgb([220, 236, 250]));
            }
        }
        Weather::Rain => {
            for (dx, r) in [(-7.0, 7.0), (2.0, 9.0), (10.0, 6.0)] { blip.fill_circle(x + dx * k, y - 4.0 * k, r * k, rgb([170, 180, 200])); }
            for i in 0..4 {
                let dy = ((t * 2.0 + i as f32 * 0.25) % 1.0) * 10.0 * k;
                blip.fill_rect(x + (i as f32 * 6.0 - 9.0) * k, y + 6.0 * k + dy, 2.0, 4.0 * k, rgb([120, 170, 240]));
            }
        }
        Weather::Fog => {
            for i in 0..3 { blip.fill_rect(x - 14.0 * k, y - 8.0 * k + i as f32 * 7.0 * k, 28.0 * k, 4.0 * k, rgba([230, 232, 236], 0.9)); }
        }
        Weather::Market => {
            for i in 0..4 {
                let c = if i % 2 == 0 { [220, 70, 70] } else { [250, 246, 236] };
                blip.fill_rect(x - 14.0 * k + i as f32 * 7.0 * k, y - 10.0 * k, 7.0 * k, 9.0 * k, rgb(c));
            }
            blip.fill_rect(x - 12.0 * k, y - 1.0 * k, 24.0 * k, 10.0 * k, rgb([170, 116, 70]));
        }
        Weather::Bundles => {
            for i in 0..3 {
                let yy = y + (4.0 - i as f32 * 6.0) * k;
                blip.fill_rect(x - 12.0 * k, yy, 24.0 * k, 5.0 * k, rgb([248, 246, 236]));
                blip.fill_rect(x - 1.0 * k, yy, 2.0 * k, 5.0 * k, rgb([200, 60, 60]));
            }
        }
    }
}

fn panel(blip: &Blip, x: f32, y: f32, w: f32, h: f32) {
    blip.fill_rect(x + 4.0, y + 4.0, w, h, rgba([0, 0, 0], 0.4));
    blip.fill_rect(x, y, w, h, rgb([250, 242, 222]));
    blip.draw_rect(x + 3.0, y + 3.0, w - 6.0, h - 6.0, rgb([200, 80, 90]));
}

const WEEKDAYS: [&str; 7] = ["MON", "TUE", "WED", "THU", "FRI", "SAT", "SUN"];

pub fn draw_bar(blip: &Blip, g: &Game, t: f32) {
    let y = WIN_H as f32 - BAR_H;
    let w = WIN_W as f32;
    blip.fill_rect(0.0, y, w, BAR_H, rgb([28, 22, 34]));
    blip.fill_rect(0.0, y, w, 2.0, rgb([200, 80, 90]));
    let ty = y + 9.0;
    blip.draw_text(WEEKDAYS[g.day], 10.0, ty, 2.0, rgb([255, 236, 200]));
    weather_icon(blip, g.weather(), 64.0, y + 15.0, 0.55, t);
    // papers in the box
    blip.fill_rect(90.0, ty + 2.0, 14.0, 8.0, rgb([248, 246, 236]));
    blip.fill_rect(96.0, ty + 2.0, 2.0, 8.0, rgb([200, 60, 60]));
    let low = g.stock <= 3 && (t * 4.0).fract() < 0.5;
    blip.draw_number(g.stock, 110.0, ty, 2.0, if low { rgb([255, 120, 110]) } else { BLIP_WHITE });
    // the round: homes that take the paper
    heart(blip, 166.0, ty + 4.0, 5.0, rgb(HEART));
    blip.draw_number(g.subscribers() as i32, 178.0, ty, 2.0, BLIP_WHITE);
    if g.two_player {
        blip.draw_text("2P", 230.0, ty, 2.0, rgb([140, 230, 160]));
        blip.draw_number(g.score2, 258.0, ty, 2.0, BLIP_WHITE);
    }
    // how far along the round
    let (x0, x1) = (if g.two_player { 350.0 } else { 236.0 }, w - 44.0);
    let k = if g.state == State::Canal { 1.0 - g.timer.remaining() / CANAL_SECS } else { g.bike_d / g.end_d() };
    blip.fill_rect(x0, ty + 4.0, x1 - x0, 4.0, rgb([70, 62, 80]));
    blip.fill_rect(x0, ty + 4.0, (x1 - x0) * k.clamp(0.0, 1.0), 4.0, rgb([240, 190, 90]));
    blip.fill_circle(x0 + (x1 - x0) * k.clamp(0.0, 1.0), ty + 6.0, 5.0, rgb([236, 110, 60]));
    blip.fill_circle(x1 + 6.0, ty + 6.0, 5.0, rgb([120, 200, 140]));
}

pub fn draw_card(blip: &Blip, g: &Game, t: f32) {
    let (x, y, w, h) = (60.0, 120.0, 360.0, 250.0);
    let slide = ((g.clock / 0.35).min(1.0) - 1.0) * 40.0;
    let y = y + slide;
    panel(blip, x, y, w, h);
    let ink = rgb([60, 40, 50]);
    blip.draw_centered(&format!("WEEK {}  NOERREBRO TO NYHAVN", g.week), y + 22.0, 2.0, rgb([200, 80, 90]));
    blip.draw_centered(g.today().name, y + 46.0, 4.0, ink);
    weather_icon(blip, g.weather(), 240.0, y + 112.0, 1.6, t);
    blip.draw_centered(g.today().tell, y + 150.0, 2.0, ink);
    blip.draw_centered(&format!("ROUND {} HOMES", g.subscribers()), y + 182.0, 2.0, rgb([200, 80, 90]));
    blip.draw_centered(&format!("PAPERS {}", g.stock), y + 206.0, 2.0, ink);
    if g.weather() == Weather::Wind(-60.0) || matches!(g.weather(), Weather::Wind(_)) {
        // the wind's way, an arrow along the street
        let dir = -g.wind().signum();
        let (ax, ay) = (290.0, y + 112.0);
        blip.draw_line_ex(ax, ay - 12.0 * dir, ax, ay + 12.0 * dir, 3.0, ink);
        blip.draw_line_ex(ax, ay + 12.0 * dir, ax - 6.0, ay + 5.0 * dir, 3.0, ink);
        blip.draw_line_ex(ax, ay + 12.0 * dir, ax + 6.0, ay + 5.0 * dir, 3.0, ink);
    }
}

pub fn draw_tally(blip: &Blip, g: &Game) {
    let (x, y, w, h) = (50.0, 96.0, 380.0, 320.0);
    panel(blip, x, y, w, h);
    let ink = rgb([60, 40, 50]);
    let accent = rgb([200, 80, 90]);
    blip.draw_centered(&format!("{} DONE", g.today().name), y + 22.0, 3.0, ink);
    let x2 = bundles_mult(g);
    let rows: [(&str, String, bool); 6] = [
        ("LETTERBOX", format!("{} X {}", g.tally.letterbox, LETTERBOX_PTS * x2), true),
        ("DOORSTEP", format!("{} X {}", g.tally.doorstep, DOORSTEP_PTS * x2), true),
        ("SAMPLES", format!("{} X {}", g.tally.samples, SAMPLE_PTS * x2), true),
        ("MISSED", g.tally.misses.to_string(), g.tally.misses > 0),
        ("CANCELLED", g.tally.lost.to_string(), g.tally.lost > 0),
        ("NEW TOMORROW", g.tally.won.to_string(), g.tally.won > 0),
    ];
    for (i, (name, val, on)) in rows.iter().enumerate() {
        if g.clock < 0.25 + i as f32 * 0.22 { break; }
        let ry = y + 64.0 + i as f32 * 30.0;
        let c = if *on { ink } else { rgba([60, 40, 50], 0.4) };
        blip.draw_text(name, x + 24.0, ry, 2.0, c);
        blip.draw_text(val, x + w - 24.0 - text_width(val, 2.0), ry, 2.0, c);
    }
    if g.clock > 1.8 {
        if g.tally.perfect > 0 {
            let pulse = 0.75 + 0.25 * (g.clock * 6.0).sin();
            blip.draw_centered(&format!("PERFECT DAY +{}", g.tally.perfect), y + 252.0, 2.0, BlipColor { a: pulse, ..accent });
        }
        blip.draw_centered("NEXT: THE CANAL RUN", y + 282.0, 2.0, rgb([60, 110, 150]));
    }
}

fn bundles_mult(g: &Game) -> i32 { if g.weather() == Weather::Bundles { 2 } else { 1 } }

pub fn draw_week(blip: &Blip, g: &Game) {
    let (x, y, w, h) = (60.0, 140.0, 360.0, 220.0);
    panel(blip, x, y, w, h);
    let ink = rgb([60, 40, 50]);
    blip.draw_centered(&format!("WEEK {} DONE", g.week), y + 26.0, 3.0, ink);
    blip.draw_centered(&format!("{} HOMES KEPT", g.tally.won), y + 80.0, 2.0, ink);
    blip.draw_centered(&format!("+{}", g.tally.won * KEPT_PTS), y + 108.0, 3.0, rgb([200, 80, 90]));
    blip.draw_centered("A SPARE LIFE", y + 150.0, 2.0, ink);
    blip.draw_centered("THE STREETS GET BUSIER", y + 180.0, 2.0, rgb([60, 110, 150]));
}

pub fn draw_canal_hud(blip: &Blip, g: &Game) {
    if g.clock < 2.2 {
        let a = (2.2 - g.clock).min(1.0);
        label(blip, "NYHAVN", 240.0, 120.0, 5.0, rgba([255, 236, 200], a));
        label(blip, "PAPERS FOR THE CAFES", 240.0, 176.0, 2.0, rgba([255, 255, 255], a));
        label(blip, "AND THE BOATS", 240.0, 200.0, 2.0, rgba([255, 255, 255], a));
    }
    let secs = g.timer.remaining().ceil() as i32;
    label(blip, &secs.to_string(), 240.0, HUD_H + 10.0, 3.0, rgb(if secs <= 3 { [255, 140, 120] } else { [255, 255, 255] }));
}

pub fn draw_banner(blip: &Blip, g: &Game) {
    // at the T: what the turn is waiting for
    if g.in_junction() && g.heading < 0.5 {
        label(blip, "TURN LEFT HERE", 240.0, BIKE_Y - 90.0, 2.0, rgb([255, 236, 160]));
    }
    if g.state == State::Play && g.place_t < 3.0 {
        let a = (3.0 - g.place_t).min(1.0) * (g.place_t * 3.0).min(1.0);
        let name = PLACES[g.place].1;
        let wdt = text_width(name, 2.0) + 20.0;
        blip.fill_rect(240.0 - wdt / 2.0, HUD_H + 8.0, wdt, 24.0, rgba([24, 40, 80], 0.85 * a));
        blip.draw_rect(240.0 - wdt / 2.0 + 2.0, HUD_H + 10.0, wdt - 4.0, 20.0, rgba([240, 236, 220], a));
        blip.draw_centered(name, HUD_H + 13.0, 2.0, rgba([250, 248, 240], a));
    }
    if let Some((text, left)) = &g.banner {
        label(blip, text, 240.0, 150.0, 2.0, rgba(GOLD, left.min(1.0)));
    }
    if g.state == State::Play && g.clock < 1.2 {
        label(blip, "GO!", 240.0, 200.0, 5.0, rgba([255, 236, 200], (1.2 - g.clock).min(1.0)));
    }
}

pub fn draw_title(blip: &Blip, g: &Game, hi: i32, t: f32) {
    draw_world(blip, g, t);
    draw_weather(blip, g, t);
    skyline(blip, t);
    let (cx, cy) = (240.0, 70.0);
    label(blip, "ROUNDS", cx, cy, 7.0, rgb([255, 244, 220]));
    label(blip, "A MORNING ROUND IN COPENHAGEN", cx, cy + 52.0, 2.0, rgb([255, 236, 210]));
    blip.draw_hi(hi, cy + 74.0, rgb([255, 236, 200]));
    blip.draw_start_prompt(WIN_H as f32 * 0.52);
    let by = blip::web::controls();
    label(blip, by.pick("A D STEER  W S SPEED", "STICK STEERS AND PEDALS", "DRAG TO STEER AND PEDAL"), cx, 330.0, 2.0, BLIP_WHITE);
    label(blip, by.pick("F THROWS LEFT  G RIGHT", "A THROWS LEFT  B RIGHT", "TAP LEFT OR RIGHT TO THROW"), cx, 354.0, 2.0, BLIP_WHITE);
    label(blip, by.pick("J: TWO ON A TANDEM", "2P JOINS ON THE BACK", "2P JOINS ON THE BACK"), cx, 378.0, 2.0, rgb([160, 230, 170]));
    let _ = BLIP_BLACK;
}

/// Copenhagen at dawn, as a postcard: the sky warming, the towers dark against it (Vor Frelsers Kirke's twisting
/// spire, Christiansborg's tower, Marmorkirken's dome, Rundetaarn), and Nyhavn's houses on the water in front.
fn skyline(blip: &Blip, t: f32) {
    let (top, horizon, bottom) = (0.0, 238.0, 290.0);
    let mut y = top;
    while y < horizon {
        let f = (y - top) / (horizon - top);
        let c = [(60.0 + 195.0 * f) as u8, (50.0 + 120.0 * f * f) as u8, (110.0 - 10.0 * f) as u8];
        blip.fill_rect(0.0, y, WIN_W as f32, 4.0, rgb(c));
        y += 4.0;
    }
    blip.fill_circle(330.0, horizon - 6.0, 30.0, rgba([255, 214, 130], 0.9));
    blip.fill_sunburst(330.0, horizon - 6.0, 14, t * 0.08, 140.0, [rgba([255, 214, 140], 0.18), rgba([255, 214, 140], 0.0)]);
    let ink = rgb([46, 36, 70]);
    // Vor Frelsers Kirke
    blip.fill_rect(40.0, horizon - 50.0, 26.0, 50.0, ink);
    for i in 0..90 {
        let f = i as f32 / 90.0;
        let half = 10.0 * (1.0 - f) + 1.0;
        blip.fill_rect(53.0 - half, horizon - 50.0 - i as f32, half * 2.0, 1.2, ink);
        let a = f * TAU * 4.0;
        if a.sin() > 0.0 { blip.fill_rect(53.0 + a.cos() * (half + 1.5) - 1.0, horizon - 50.0 - i as f32, 2.0, 1.5, rgb(GOLD)); }
    }
    blip.fill_circle(53.0, horizon - 143.0, 3.0, rgb(GOLD));
    // Christiansborg's tower and its crowns
    blip.fill_rect(118.0, horizon - 70.0, 30.0, 70.0, ink);
    blip.fill_rect(124.0, horizon - 96.0, 18.0, 26.0, ink);
    for i in 0..30 { let half = 7.0 * (1.0 - i as f32 / 30.0) + 1.0; blip.fill_rect(133.0 - half, horizon - 96.0 - i as f32, half * 2.0, 1.2, ink); }
    for k in 0..3 { blip.fill_circle(133.0, horizon - 132.0 - k as f32 * 7.0, 3.5, ink); }
    blip.fill_rect(80.0, horizon - 36.0, 120.0, 36.0, ink);
    // Marmorkirken
    blip.fill_rect(200.0, horizon - 40.0, 70.0, 40.0, ink);
    for i in 0..34 { let f = i as f32 / 34.0; let half = 30.0 * (1.0 - f * f).sqrt(); blip.fill_rect(235.0 - half, horizon - 40.0 - i as f32, half * 2.0, 1.2, ink); }
    blip.fill_rect(231.0, horizon - 90.0, 8.0, 18.0, ink);
    blip.fill_rect(234.0, horizon - 98.0, 2.0, 8.0, ink);
    // Rundetaarn
    blip.fill_rect(380.0, horizon - 62.0, 42.0, 62.0, ink);
    blip.fill_circle(401.0, horizon - 66.0, 8.0, ink);
    blip.fill_rect(300.0, horizon - 24.0, 180.0, 24.0, ink);
    // the water and Nyhavn's houses on it, lit by the low sun
    blip.fill_rect(0.0, horizon, WIN_W as f32, bottom - horizon, rgb([70, 90, 130]));
    for i in 0..16 {
        let x = i as f32 * 30.0;
        let c = NYHAVN[i % NYHAVN.len()];
        let h = 18.0 + (scatter(i, 99) * 3.0).floor() * 5.0;
        blip.fill_rect(x, horizon - h + 26.0, 29.0, h, rgb(c));
        tri((x, horizon - h + 26.0), (x + 29.0, horizon - h + 26.0), (x + 14.5, horizon - h + 16.0), rgb(shade_u(c, 0.92)));
        for wx in [x + 5.0, x + 18.0] { blip.fill_rect(wx, horizon - h + 31.0, 6.0, 5.0, rgb([255, 220, 140])); }
        let ripple = (t * 2.0 + i as f32).sin() * 2.0;
        blip.fill_rect(x + ripple, horizon + 30.0, 29.0, 4.0, rgba(c, 0.35));
    }
}

pub fn draw_over(blip: &Blip, g: &Game, hi: i32) {
    blip.fill_rect(0.0, 0.0, WIN_W as f32, WIN_H as f32, rgba([20, 14, 30], 0.72));
    let title = if g.subscribers() == 0 { "NO ONE LEFT" } else { "GAME OVER" };
    let offering = g.offering();
    blip.draw_end_screen(title, g.sess.score + g.score2, if g.two_player { 0 } else { hi }, rgb([255, 150, 130]),
        rgb([255, 214, 90]), BLIP_WHITE, !offering && !g.wait.active());
    if offering { blip.draw_continue(&g.cont, rgb([255, 214, 90])); }
    let note = if g.two_player { format!("1P {}   2P {}", g.sess.score, g.score2) } else {
        format!("WEEK {} {}   {} HOMES", g.week, WEEKDAYS[g.day], g.subscribers())
    };
    blip.draw_game_over_note(&note, rgb([200, 190, 210]));
}
