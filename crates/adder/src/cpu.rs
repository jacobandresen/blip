//! Brains: what a CPU viper wants this frame. The native autopilot asks the
//! same question and presses the keys a person would, so a playtest steers
//! through the ordinary input path.

use blip::angle_diff;

use crate::arena::{Pt, Steer, World};

/// How far a snake looks ahead when it is deciding where to go.
const LOOK: f32 = 70.0;
/// The clearance a heading needs is a time, not a distance: half a second of
/// travel, so a strike (which covers the ground 1.75 times as fast) asks for
/// more room rather than the same 44 px.
const SAFE_SECS: f32 = 0.45;
/// Straight on, then every 14 degrees out to a hard turn either way.
const FAN: [f32; 13] = [
    0.0, 0.25, -0.25, 0.5, -0.5, 0.75, -0.75, 1.0, -1.0, 1.25, -1.25, 1.5, -1.5,
];

/// Where a snake is heading for: the nearest egg, or — when its own way there is
/// walled off — the nearest hole in another body, because a hole is the one way
/// through and it pays.
fn quarry(w: &World, who: usize) -> Option<Pt> {
    let head = w.snakes[who].head;
    let mut egg = None;
    let mut egg_d = f32::MAX;
    for e in &w.eggs {
        let d = head.dist(*e);
        if d < egg_d { egg_d = d; egg = Some(*e); }
    }
    if let Some(e) = egg {
        let dir = (e.y - head.y).atan2(e.x - head.x);
        if w.clearance(who, dir, egg_d) >= egg_d - 1.0 { return Some(e); }
    }
    let mut hole = None;
    let mut hole_d = f32::MAX;
    for (j, s) in w.snakes.iter().enumerate() {
        if j == who || !s.alive { continue; }
        if let Some(c) = s.hole_centre() {
            let d = head.dist(c);
            if d < hole_d { hole_d = d; hole = Some(c); }
        }
    }
    hole.or(egg)
}

/// A turn of up to full lock, and the throttle down only where the way ahead is
/// clear for the whole of a strike step and then some.
pub fn steer(w: &World, who: usize) -> Steer {
    let s = &w.snakes[who];
    let want = quarry(w, who).map(|q| (q.y - s.head.y).atan2(q.x - s.head.x));
    let need = w.speed_base() * SAFE_SECS * 1.3;
    let mut best_safe = (0.0f32, f32::MIN);
    let mut best_any = (0.0f32, f32::MIN);
    for off in FAN {
        let dir = s.heading + off;
        let clear = w.clearance(who, dir, LOOK).min(LOOK);
        let mut score = clear;
        score -= off.abs() * 4.0; // a lazy turn is a cheap turn
        if let Some(want) = want { score -= angle_diff(dir, want).abs() * 40.0; }
        if score > best_any.1 { best_any = (off, score); }
        if clear >= need && score > best_safe.1 { best_safe = (off, score); }
    }
    // A way out of trouble beats a way toward the egg, every time.
    let off = if best_safe.1 > f32::MIN { best_safe.0 } else { best_any.0 };
    let reach = need * 3.0;
    let ahead = w.clearance(who, s.heading, reach);
    Steer { turn: (off / 0.5).clamp(-1.0, 1.0), boost: ahead >= reach }
}
