//! How to play Meteors: turn to aim ahead of the most urgent rock or the
//! saucer and fire when lined up; thrust away from rocks closing in, and
//! jump through hyperspace from a saucer's shot about to hit.

use super::*;
use blip::bot::{predict, Goal::*, Pad, Playthrough};

/// What the bot must show it can do in Meteors.
pub const PLAYTHROUGHS: &[Playthrough] = &[
    Playthrough {
        name: "the ship breaks big rocks into dust and clears wave 1",
        goals: &[AtLeast("rocks_split", 6.0), AtLeast("rocks_dust", 12.0), AtLeast("level", 2.0)],
        ..Playthrough::DEFAULT
    },
    Playthrough {
        name: "it shoots down a saucer",
        seconds: 240.0,
        goals: &[AtLeast("saucers", 1.0)],
        ..Playthrough::DEFAULT
    },
    Playthrough {
        name: "it jumps through hyperspace out of a saucer's line of fire",
        seconds: 240.0,
        goals: &[AtLeast("hyperspace", 1.0)],
        ..Playthrough::DEFAULT
    },
    Playthrough {
        name: "it clears a storm wave",
        level: STORM_WAVE as u32,
        seconds: 240.0,
        goals: &[AtLeast("level", STORM_WAVE as f64 + 1.0)],
        ..Playthrough::DEFAULT
    },
];

/// A rock or saucer as the ship sees it.
struct Target {
    /// Seconds-ish until it matters: the lower, the sooner to deal with.
    urgency: f32,
    /// The heading that hits it, leading its motion.
    aim: f32,
    distance: f32,
    /// How close its edge is, and how fast it is coming.
    gap: f32,
    closing: f32,
    /// Where it is, relative to the ship.
    dx: f32,
    dy: f32,
}

pub fn drive(g: &Game) {
    let mut pad = Pad::new();
    match g.state {
        State::Title => { pad.press_start(); }
        State::Play if g.ship_alive => fly(g, &mut pad),
        _ => {}
    }
    pad.send();
}

fn fly(g: &Game, pad: &mut Pad) {
    let targets = rocks_and_saucer(g);
    if let Some(t) = targets.iter().min_by(|a, b| a.urgency.total_cmp(&b.urgency)) {
        pad.turn_to(g.ship.angle, t.aim, 0.06);
        let lined_up = blip::angle_diff(g.ship.angle, t.aim).abs() < 0.12;
        if lined_up && t.distance < BULLET_SPEED * BULLET_TTL + 30.0 { pad.fire(); }
    }
    // A shot that hits within a fifth of a second cannot be flown from.
    if saucer_shots(g).iter().any(|t| t.gap < t.closing * 0.2) {
        pad.tap_button2();
        return;
    }
    let closing_in = targets.iter().find(|t| t.gap < 70.0 && t.closing > 20.0);
    if let Some(t) = closing_in {
        // The nose points away from it: thrust.
        if blip::angle_diff(g.ship.angle, (-t.dx).atan2(t.dy)).abs() < 1.2 { pad.up(); }
    }
}

/// Every rock and the saucer.
fn rocks_and_saucer(g: &Game) -> Vec<Target> {
    let mut seen: Vec<_> = pool_iter(&g.asteroids).map(|a| see(g, a.x, a.y, a.vx, a.vy, a.size.radius())).collect();
    if g.saucer.active { seen.push(see(g, g.saucer.x, g.saucer.y, g.saucer.vx, 0.0, 16.0)); }
    seen
}

/// The saucer's shots in flight.
fn saucer_shots(g: &Game) -> Vec<Target> {
    g.bullets.iter().filter(|b| b.active && !b.from_player).map(|b| see(g, b.x, b.y, b.vx, b.vy, 8.0)).collect()
}

/// Something at (x, y) moving at (vx, vy), as the ship sees it across the screen's wrap.
fn see(g: &Game, x: f32, y: f32, vx: f32, vy: f32, radius: f32) -> Target {
    let s = &g.ship;
    let (dx, dy) = (predict::wrapped(s.x, x, PLAY_W as f32), predict::wrapped(s.y, y, PLAY_H as f32));
    let (rvx, rvy) = (vx - s.vx, vy - s.vy);
    let distance = dx.hypot(dy);
    let flight = distance / BULLET_SPEED;
    let aim = (dx + rvx * flight).atan2(-(dy + rvy * flight));
    let closing = -(dx * rvx + dy * rvy) / distance.max(1.0);
    Target { urgency: distance - radius - closing * 1.5, aim, distance, gap: distance - radius, closing, dx, dy }
}
