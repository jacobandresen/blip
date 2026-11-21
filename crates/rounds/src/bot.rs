//! How to ride Rounds, on blip's shared bot parts:
//! - which door next is a utility choice (points, a last heart at stake, whether it can still be reached in time,
//!   papers to spare), held against close rivals so the bike does not weave between sides;
//! - where to ride is a look-ahead: every lane and pedal is rolled a second and a half forward through the street's
//!   cars, dogs, skaters, stalls, sprinklers and bridge railings (each moving as the game moves it), and the clearest
//!   that heads for the door wins;
//! - throws go the moment a paper thrown now lands where it does good (the slot when it is still to come).
//!
//! On the Canal Run it lines up the best target that can still be reached. `BLIP_BOT_PLAYERS=2` rides the tandem.

use super::*;
use blip::bot::rollout::{self, Mover, Rollout};
use blip::bot::best;
use blip::bot::utility::{self, norm, Commit, Curve};
use blip::bot::{remember, Goal::*, Pad, Playthrough};
use blip::macroquad::input::KeyCode;

/// What the bot must show it can do in Rounds.
pub const PLAYTHROUGHS: &[Playthrough] = &[
    Playthrough {
        name: "the kid rides Monday's round and delivers the papers",
        goals: &[AtLeast("deliveries", 10.0), AtLeast("days", 1.0)],
        ..Playthrough::DEFAULT
    },
    Playthrough {
        name: "papers go through the letterbox",
        goals: &[AtLeast("letterbox", 8.0)],
        ..Playthrough::DEFAULT
    },
    Playthrough {
        name: "a sample in a box wins a new subscriber",
        seconds: 240.0,
        goals: &[AtLeast("new_subscribers", 1.0)],
        ..Playthrough::DEFAULT
    },
    Playthrough {
        name: "on windy Wednesday the throws drift and still land",
        level: 3,
        goals: &[AtLeast("deliveries", 10.0), AtLeast("days", 1.0)],
        ..Playthrough::DEFAULT
    },
    Playthrough {
        name: "in Thursday's rain the papers skid and it aims short",
        level: 4,
        goals: &[AtLeast("deliveries", 10.0), AtLeast("days", 1.0)],
        ..Playthrough::DEFAULT
    },
    Playthrough {
        name: "through Friday's fog by the lit doors",
        level: 5,
        goals: &[AtLeast("deliveries", 10.0), AtLeast("days", 1.0)],
        ..Playthrough::DEFAULT
    },
    Playthrough {
        name: "market day: round the stalls and the crowd",
        level: 6,
        goals: &[AtLeast("days", 1.0), AtMost("crashes", 0.0)],
        ..Playthrough::DEFAULT
    },
    Playthrough {
        name: "Sunday's heavy bundles pay double",
        level: 7,
        goals: &[AtLeast("deliveries", 8.0), AtLeast("days", 1.0)],
        ..Playthrough::DEFAULT
    },
    Playthrough {
        name: "a perfect day: every subscriber gets the paper",
        seconds: 480.0,
        goals: &[AtLeast("perfect_days", 1.0)],
        ..Playthrough::DEFAULT
    },
    Playthrough {
        name: "the Canal Run lands papers in baskets and floats",
        goals: &[AtLeast("canal_hits", 5.0)],
        ..Playthrough::DEFAULT
    },
    Playthrough {
        name: "a dog fetches a paper instead of the bike",
        level: 2,
        seconds: 600.0,
        goals: &[AtLeast("fetches", 1.0)],
        settings: &[("FETCH", "1")],
        ..Playthrough::DEFAULT
    },
    Playthrough {
        name: "two riders share the tandem",
        settings: &[("PLAYERS", "2")],
        goals: &[AtLeast("join", 1.0), AtLeast("deliveries", 10.0), AtLeast("days", 1.0)],
        ..Playthrough::DEFAULT
    },
    Playthrough {
        name: "it rides the whole week",
        seconds: 900.0,
        goals: &[AtLeast("weeks", 1.0)],
        ..Playthrough::DEFAULT
    },
];

/// A door the bot can aim for: which side and row, and where across the street a throw reaches it.
#[derive(Copy, Clone, PartialEq, Debug)]
struct Door { side: Side, row: usize, x: f32, letterbox: bool }

#[derive(Default)]
struct Mind { door: Commit<(Side, usize)> }

pub fn drive(g: &Game) {
    let mut pad = Pad::new();
    let two = blip::bot::is("PLAYERS", "2");
    match g.state {
        State::Title if two => { pad.tap(BLIP_KEY_J); }
        State::Over if g.offering() => { if blip::bot::may_continue() { pad.press_start(); } }
        State::Title | State::Over | State::Card | State::Tally | State::Week => { pad.press_start(); }
        State::Play => ride(g, &mut pad),
        State::Canal => canal(g, &mut pad),
        State::Crash => {}
    }
    pad.send();
}

/// The keys that steer (less, more) and pedal (slower, faster), alone or on the tandem.
fn keys(g: &Game) -> ((KeyCode, KeyCode), (KeyCode, KeyCode)) {
    let steer = if g.two_player { (BLIP_KEY_A, BLIP_KEY_D) } else { (BLIP_KEY_LEFT, BLIP_KEY_RIGHT) };
    (steer, (BLIP_KEY_DOWN, BLIP_KEY_UP))
}

fn throw_key(g: &Game, side: Side) -> KeyCode {
    match side { Side::Left => BLIP_KEY_F, Side::Right if g.two_player => BLIP_KEY_J, Side::Right => BLIP_KEY_G }
}

/// Is a paper already on its way to this door?
fn on_its_way(g: &Game, side: Side, row: usize) -> bool {
    g.papers.iter().any(|p| p.landed.is_none() && p.side == side && row_at(p.f.d) == Some(row))
}

/// Papers left over once every subscriber still ahead has one.
fn spare(g: &Game) -> i32 {
    let due = Side::BOTH.iter().flat_map(|&s| (0..ROWS).map(move |r| (s, r)))
        .filter(|&(s, r)| { let h = g.houses[s.index()][r]; h.sub && !h.done && door_d(r) > g.bike_d }).count() as i32;
    g.stock - due
}

/// The band of x across the street from which a throw to `side` at today's speed ends as `want` at `row`.
fn band(g: &Game, side: Side, want: Outcome, row: usize) -> Option<(f32, f32)> {
    let lot = Some(g.houses[side.index()][row].lot);
    let (mut lo, mut hi) = (f32::MAX, f32::MIN);
    let mut x = RIDE[0];
    while x <= RIDE[1] {
        let f = flight(x, 0.0, g.speed, side, g.wind(), g.rain());
        let at = Flight { d: door_d(row) + if want == Outcome::Sample { BOX_D } else { 0.0 }, ..f };
        if outcome(side, at, lot).0 == want { lo = lo.min(x); hi = hi.max(x); }
        x += 3.0;
    }
    (lo <= hi).then_some((lo, hi))
}

/// A subscriber still waiting on the same side within a second's ride of `d`: it comes first. (One across the
/// street is no conflict: a sample box is reached from the far half of the road, where that side's slot is.)
fn subscriber_near(g: &Game, side: Side, d: f32) -> bool {
    [side].iter().any(|&s| (0..ROWS).any(|r| {
        let h = g.houses[s.index()][r];
        h.sub && !h.done && door_d(r) > g.bike_d && (door_d(r) - d).abs() < g.speed
    }))
}

/// Every door ahead that wants something, with where to ride for it.
fn doors(g: &Game) -> Vec<(Door, f32)> {
    let spare = spare(g);
    let mut out = Vec::new();
    for side in Side::BOTH {
        for row in 0..ROWS {
            let ahead = door_d(row) - g.bike_d;
            if !(30.0..460.0).contains(&ahead) { continue; }
            let h = g.houses[side.index()][row];
            if !h.lot.is_house() || on_its_way(g, side, row) { continue; }
            let (want, worth) = if h.sub && !h.done {
                // a subscriber on a last heart would be lost: it matters most
                (Outcome::Letterbox, if h.hearts <= 1 { 1.0 } else { 0.8 })
            } else if !h.sub && !h.sampled && spare > 2 && !subscriber_near(g, side, door_d(row)) {
                (Outcome::Sample, 0.85 * utility::Curve::Power(0.5).of(norm(spare as f32, 2.0, 8.0)))
            } else { continue };
            let pick = |want| band(g, side, want, row).map(|(lo, hi)| {
                // well inside the band, nearer the middle of the street
                let inset = ((hi - lo) * 0.3).min(10.0);
                (lo + inset).max((hi - inset).min(240.0)).min(hi - inset)
            });
            let (x, letterbox) = match pick(want) {
                Some(x) => (x, want == Outcome::Letterbox),
                None if want == Outcome::Letterbox => match pick(Outcome::Doorstep) { Some(x) => (x, false), None => continue },
                None => continue,
            };
            // can the bike get across in the time it has?
            let time = ahead / g.speed.max(1.0);
            let across = (x - g.bike_x).abs() / (STEER_V * 0.7) + 0.25;
            let reach = Curve::Logistic { mid: 0.35, steep: 10.0 }.of(norm(time - across, -1.0, 2.0));
            let soon = Curve::Inverse.of(norm(ahead, 30.0, 460.0));
            let score = utility::score(&[worth, reach, 0.3 + 0.7 * soon]);
            out.push((Door { side, row, x, letterbox }, score));
        }
    }
    out
}

/// The street as things the bike must not meet, each moving as the game moves it.
fn movers(g: &Game) -> Vec<Mover> {
    let mut out = Vec::new();
    // only what the look-ahead can reach: a second and a half at top speed, and what comes down to meet it
    let (lo, hi) = (g.bike_d - 80.0, g.bike_d + SPEED_MAX * 1.6 + 260.0);
    for h in g.hazards.iter().filter(|h| !h.gone && h.d > lo && h.d < hi + if h.oncoming || h.kind == Hz::Blader { 200.0 } else { 0.0 }) {
        let (hw, hl) = h.size();
        let (vx, vd) = h.velocity(g.bike_x, g.bike_d);
        let s = h.side.index();
        let here = Mover::moving(h.x, h.d, vx, vd).sized(hw, hl);
        match h.kind {
            Hz::Car if !h.oncoming && h.phase <= GO => {
                // backing out once the reverse lights have glowed, stopping in its lane
                let lane = CAR_LANE[s];
                let wait = if h.phase == WAIT { ((h.d - g.bike_d - CAR_WAKE).max(0.0) / g.speed.max(1.0)) + CAR_TELL }
                    else if h.phase == TELL { (CAR_TELL - h.t).max(0.0) } else { 0.0 };
                let v = -h.side.sign() * 65.0;
                let travel = ((lane - h.x) / v).max(0.0);
                if h.phase < GO { out.push(Mover::still(h.x, h.d).sized(19.0, 10.0).weighted(0.0)); }
                out.push(Mover::moving(h.x, h.d, v, 0.0).sized(19.0, 12.0).from(wait).stops(wait + travel));
            }
            // a dog out, or about to be, can be anywhere between its gate and as far as it runs; it keeps up with the bike
            Hz::Dog if h.phase <= GO => {
                let wait = match h.phase {
                    WAIT => (h.d - g.bike_d - DOG_WAKE).max(0.0) / g.speed.max(1.0) + DOG_TELL,
                    TELL => (DOG_TELL - h.t).max(0.0),
                    _ => 0.0,
                };
                let vd = if h.phase == GO { ((g.bike_d - h.d) * 3.0).clamp(-DOG_SPEED, DOG_SPEED) } else { 0.0 };
                let reach = DOG_REACH[s];
                let (lo, hi) = if h.side == Side::Left { (h.x, reach) } else { (reach, h.x) };
                out.push(Mover::moving((lo + hi) / 2.0, h.d, 0.0, vd).sized((hi - lo) / 2.0 + 7.0, 14.0).from(wait));
            }
            Hz::Sprinkler => {
                let reach = if h.side == Side::Left { CURB[0] + SPRAY_REACH } else { CURB[1] - SPRAY_REACH };
                let (lo, hi) = if h.side == Side::Left { (FENCE[0], reach) } else { (reach, FENCE[1]) };
                out.push(Mover::still((lo + hi) / 2.0, h.d).sized((hi - lo) / 2.0, SPRAY_REACH).weighted(0.12));
            }
            Hz::Puddle(_) => out.push(Mover::still(h.x, h.d).sized(20.0, 14.0).weighted(0.08)),
            Hz::Cart => out.push(Mover::moving(h.x, h.d, vx, vd).sized(12.0, 12.0).weighted(if g.stock < 15 { -0.4 } else { 0.0 })),
            Hz::Pigeons(_) => {}
            // a tourist can be anywhere from the track out into the road: give it the lot
            Hz::Tourist => {
                let base = h.home;
                let (lo, hi) = if h.side == Side::Left { (base - 10.0, base + 46.0) } else { (base - 46.0, base + 10.0) };
                let vd = if h.side == Side::Right { 80.0 } else { -95.0 } * if h.phase == GO { 1.0 } else { 0.0 };
                out.push(Mover::moving((lo + hi) / 2.0, h.d, 0.0, vd).sized((hi - lo) / 2.0, 14.0));
            }
            Hz::Bus => {
                out.push(here);
                // the passengers' crossing over the cycle track while it waits, and a moment before
                if h.phase <= TELL {
                    let wait = if h.phase == WAIT { (h.d - g.bike_d - BUS_WAKE).max(0.0) / g.speed.max(1.0) } else { 0.0 };
                    let left = if h.phase == TELL { BUS_DWELL - h.t } else { BUS_DWELL + wait };
                    out.push(Mover::still(BIKE_LANE[s], h.d).sized(13.0, 12.0).from(wait).stops(f32::INFINITY).weighted(if left > 0.0 { 1.0 } else { 0.0 }));
                }
            }
            _ if hw > 0.0 => {
                // walkers, skaters and vans not yet moving soon will be
                let (vx, vd) = match (h.kind, h.phase) {
                    (Hz::Walker, WAIT) => (-h.side.sign() * 34.0, 0.0),
                    (Hz::Blader, WAIT) => (0.0, -70.0),
                    (Hz::Cyclist, WAIT) => (0.0, if h.side == Side::Right { 58.0 } else { -85.0 }),
                    (Hz::Car, WAIT) if h.oncoming => (0.0, -110.0),
                    _ => (vx, vd),
                };
                out.push(Mover { vx, vy: vd, ..here });
            }
            _ => {}
        }
    }
    // Frederik V on Amalienborg's square
    if statue_d() > lo - LOT && statue_d() < hi { out.push(Mover::still(240.0, statue_d()).sized(STATUE_HALF, STATUE_HALF)); }
    // the railings of every bridge ahead
    for r in 0..ROWS {
        if g.houses[0][r].lot != Lot::Bridge { continue; }
        let mid = door_d(r);
        if mid < lo - LOT || mid > hi { continue; }
        out.push(Mover::still(RAIL[0] - 40.0, mid).sized(40.0, LOT / 2.0 + 4.0));
        out.push(Mover::still(RAIL[1] + 40.0, mid).sized(40.0, LOT / 2.0 + 4.0));
    }
    out
}

const LOOK: Rollout = Rollout { me: (BIKE_HW, BIKE_HL), margin: 5.0, dt: 0.1, hit: 10.0, halo: 0.12, halo_reach: 8.0, fade: 0.5 };

/// The left turn into Frederiksgade: keep to the left half coming up to it, wait for a gap in the traffic coming
/// down the old street, then steer round until the bike points down the new street.
fn turn_left(g: &Game, pad: &mut Pad) {
    let ((less, more), (slower, _)) = keys(g);
    let target = if g.bike_d > JD - 30.0 { std::f32::consts::FRAC_PI_2 } else { 0.0 };
    if g.oncoming_near() && g.heading < 0.5 {
        pad.hold(slower);
        return;
    }
    if g.heading < target - 0.05 { pad.hold(less); } else if g.heading > target + 0.05 { pad.hold(more); }
}

/// The hit boxes the game tests (`solid` hazards: each box shrunk by half of `CRASH_INSET`), for the fun meter, on the canvas (`draw::sy`); not while the view turns at the junction.
fn hit_boxes(g: &Game) {
    let seg = g.seg();
    let (ix, iy) = (CRASH_INSET.0 / 2.0, CRASH_INSET.1 / 2.0);
    let me = Mover::still(g.bike_x, crate::draw::sy(g.bike_d)).sized(BIKE_HW - ix, BIKE_HL - iy);
    let hazards: Vec<Mover> = g.hazards.iter().filter(|h| !h.gone && h.seg == seg && h.solid())
        .map(|h| { let (hw, hl) = h.size(); Mover::still(h.x, crate::draw::sy(h.d)).sized(hw - ix, hl - iy) }).collect();
    blip::bot::near(&[me], &hazards);
}

fn ride(g: &Game, pad: &mut Pad) {
    if g.in_junction() { return turn_left(g, pad); }
    hit_boxes(g);
    throws(g, pad);
    let doors = doors(g);
    let chosen = remember(|m: &mut Mind| m.door.pick(doors.iter().map(|(d, s)| ((d.side, d.row), *s)), 0.25));
    let door = chosen.and_then(|k| doors.iter().map(|(d, _)| *d).find(|d| (d.side, d.row) == k));
    let goal = door.map_or(240.0, |d| d.x);
    let busy = door.is_some();
    let movers = movers(g);
    // every lane and pedal, a second and a half ahead
    let top = g.top_speed();
    let pedals = [(-1, SPEED_MIN), (0, CRUISE * g.pace().min(1.2)), (1, top)];
    let mut options = Vec::new();
    let mut x = RIDE[0];
    while x <= RIDE[1] { for p in pedals { options.push((x, p)); } x += 8.0; }
    let cost = |(x, (pedal, v)): (f32, (i32, f32))| {
        let speed = (g.speed + v) / 2.0;
        let path = rollout::glide(g.bike_x, x, STEER_V * 0.75, g.bike_d, speed, LOOK.dt, 15);
        let danger = LOOK.risk(&path, &movers);
        // near the door's line, unhurried while there are doors to make, and steady
        let off = (x - goal).abs() / 120.0;
        // cruise unless the street says otherwise; sprint the empty stretches
        let pace = match pedal { 0 => 0.0, 1 if !busy => -0.02, _ => 0.12 };
        danger + off + pace + (x - g.bike_x).abs() / 2000.0
    };
    let Some((x, (pedal, _))) = best(options, cost) else { return };
    let ((less, more), (slower, faster)) = keys(g);
    // coming up to the left turn: over to the left half of the road
    let x = if !g.turned && g.bike_d > JD - MOUTH_HALF - 260.0 { 200.0 } else { x };
    pad.arrive(g.bike_x, g.steer_v, x, 5.0, STEER_V, 25.0, less, more);
    if wait_for_light(g) { pad.hold(slower); return; }
    match pedal { -1 => { pad.hold(slower); } 1 => { pad.hold(faster); } _ => {} }
}

/// Should the bike stop at the next junction's line? When the light there will not still be green by the time the
/// bike is over the side street.
fn wait_for_light(g: &Game) -> bool {
    JUNCTIONS.iter().any(|&row| {
        let gap = Game::stop_d(row) - g.bike_d;
        if !(-0.5..170.0).contains(&gap) { return false; }
        let (light, t) = g.light(row);
        let over = (gap + 80.0 + 2.0 * BIKE_HL) / g.speed.max(CRUISE * 0.8);
        // the turn is over the side street only: the box is just past it
        light != Light::Green || t + over > GREEN_END - 0.3
    })
}

/// Throw whenever a paper thrown now lands where it does good: the slot, or the doorstep once the slot cannot be had.
fn throws(g: &Game, pad: &mut Pad) {
    for side in Side::BOTH {
        if g.throw_cd[side.index()] > 0.0 { continue; }
        let (what, row) = g.would_hit(side);
        let Some(row) = row else { continue };
        let h = g.houses[side.index()][row];
        if on_its_way(g, side, row) { continue; }
        let good = match what {
            Outcome::Letterbox => h.sub && !h.done,
            Outcome::Doorstep => h.sub && !h.done && !slot_ahead(g, side, row),
            Outcome::Sample => !h.sub && !h.sampled && spare(g) > 2,
            _ => false,
        };
        if good { pad.tap(throw_key(g, side)); }
    }
    if blip::bot::flag("FETCH") { if let Some(side) = dog_to_distract(g) { pad.tap(throw_key(g, side)); } }
}

/// Would waiting a moment put a throw through the slot of `row`?
fn slot_ahead(g: &Game, side: Side, row: usize) -> bool {
    let f = flight(g.bike_x, g.bike_d + 10.0, g.speed, side, g.wind(), g.rain());
    f.wall && !f.slid && f.d < door_d(row) - SLOT_HALF
}

/// A dog out of its gate and ahead: a paper its way.
fn dog_to_distract(g: &Game) -> Option<Side> {
    g.hazards.iter().find(|h| h.kind == Hz::Dog && h.phase == GO && h.d > g.bike_d && h.d - g.bike_d < 160.0)
        .map(|h| h.side).filter(|_| g.stock > 3)
}

fn canal(g: &Game, pad: &mut Pad) {
    // the target worth most per second of riding, that a throw can still reach in time
    let lim = g.ride_limits();
    let reachable = |t: &&Target| {
        let x = (t.x - t.side().sign() * THROW_V * FLIGHT).clamp(lim[0], lim[1]);
        let ahead = t.d + t.vd * FLIGHT - (g.bike_d + 10.0 + g.speed * CARRY * FLIGHT);
        let need = (x - g.bike_x).abs() / (STEER_V * 0.7) + 0.2;
        (ahead > 0.0 && ahead / g.speed > need).then_some((x, ahead))
    };
    // the soonest, unless one worth more is close behind it
    let mut target: Option<(i32, f32, f32)> = None;
    for t in g.targets.iter().filter(|t| t.hit < 0.0) {
        let Some((x, ahead)) = reachable(&t) else { continue };
        let pts = Game::target_points(t.x);
        if target.is_none_or(|(p, _, a)| ahead < a - 150.0 || (ahead < a + 150.0 && pts > p)) { target = Some((pts, x, ahead)); }
    }
    let ((less, more), _) = keys(g);
    if let Some((_, x, _)) = target { pad.arrive(g.bike_x, g.steer_v, x, 5.0, STEER_V, 25.0, less, more); }
    for side in Side::BOTH {
        let (lx, ld) = g.canal_landing(side);
        let hit = g.targets.iter().any(|t| t.hit < 0.0 && (t.x - lx).abs() < 10.0 && (t.d + t.vd * FLIGHT - ld).abs() < 9.0);
        if hit && g.throw_cd[side.index()] <= 0.0 { pad.tap(throw_key(g, side)); }
    }
}
