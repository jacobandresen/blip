//! The town at dawn, drawn at runtime: terraced houses on both sides, the light warming as the round goes on,
//! the weather, the neighbours waking and the cargo bike.

use crate::game::*;
use crate::levels::*;
use blip::color::*;
use blip::font::text_width;
use blip::{scatter, Blip, BlipColor, BLIP_BLACK, BLIP_WHITE};
use std::f32::consts::{PI, TAU};
mod landmarks;
mod lots;
mod height;
mod traffic;
mod canal;
mod screens;
pub use landmarks::*;
use lots::*;
pub use height::*;
pub use traffic::*;
pub use canal::*;
pub use screens::*;

pub fn rgb(c: [u8; 3]) -> BlipColor { BlipColor::from_rgba(c[0], c[1], c[2], 255) }
pub fn rgba(c: [u8; 3], a: f32) -> BlipColor { BlipColor { a, ..rgb(c) } }

const INK: [u8; 3] = blip::look::INK;
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
    // Headings (GO!, a canal's name) in the blip logo lettering, when fully shown.
    if sz >= 4.0 && c.a >= 1.0 {
        blip.draw_logo(text, cx, y, sz, c);
        return;
    }
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
        if !near_turn && (seg == 0 && g.turned || seg == 1 && !g.turned) { continue; }
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
        let open = matches!(g.houses[0][r].lot, Lot::Bridge | Lot::Cross | Lot::Palace) || r > 0 && g.houses[0][r - 1].lot == Lot::Palace;
        if !open { draw_furniture(blip, g, r, sy((r + 1) as f32 * LOT)); }
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
        let coat = if g.rain() { [[250, 210, 40], [40, 60, 110], [60, 140, 90]][k % 3] }
            else { [[200, 70, 70], [60, 90, 150], [230, 220, 200], [70, 120, 90], [40, 40, 50]][k % 5] };
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
        let lamps = [(light == Light::Red, [240, 40, 40]), (light == Light::Amber, [250, 180, 30]), (light == Light::Green, [60, 220, 90])];
        for (i, (on, c)) in lamps.into_iter().enumerate() {
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
