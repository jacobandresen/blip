//! Brains: what a CPU viper wants this frame. The native autopilot asks the
//! same question and presses the keys a person would, so a playtest steers
//! through the ordinary input path.

use blip::angle_diff;

use crate::arena::{Pt, Steer, World};

/// How far a snake looks ahead when it is deciding where to go.
const LOOK: f32 = 70.0;
/// The look-ahead margin is measured in seconds at the shared movement speed.
const SAFE_SECS: f32 = 0.45;
/// Straight on, then every 14 degrees out to a hard turn either way.
const FAN: [f32; 13] = [
    0.0, 0.25, -0.25, 0.5, -0.5, 0.75, -0.75, 1.0, -1.0, 1.25, -1.25, 1.5, -1.5,
];

/// An unspent gap offers a route through the growing trails and pays once.
fn quarry(w: &World, who: usize) -> Option<Pt> {
    let head = w.snakes[who].head;

    let mut hole = None;
    let mut hole_d = f32::MAX;
    for (j, s) in w.snakes.iter().enumerate() {
        if j == who { continue; }
        if let Some(c) = s.hole_centre() {
            let d = head.dist(c);
            if d < hole_d { hole_d = d; hole = Some(c); }
        }
    }
    hole
}

/// Safe headings take priority over scoring through a gap.
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
    // A clear route takes priority over a scoring gap.
    let off = if best_safe.1 > f32::MIN { best_safe.0 } else { best_any.0 };
    Steer { turn: (off / 0.5).clamp(-1.0, 1.0) }
}
