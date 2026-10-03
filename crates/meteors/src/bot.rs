//! Native-only autopilot (BLIP_BOT=1): aim with lead at the most urgent
//! rock or saucer, fire when lined up, thrust away from anything close.

use super::*;

/// Shortest offset across the wrap.
fn wd(d: f32, span: f32) -> f32 { let d = d.rem_euclid(span); if d > span / 2.0 { d - span } else { d } }

pub fn drive(g: &Game, t: f32) {
    let tap = (t * 10.0) as i32 % 2 == 0;
    let keys: Vec<_> = match g.state {
        State::Title => if tap { vec![BLIP_KEY_SPACE] } else { vec![] },
        State::Play if g.ship_alive => {
            let s = &g.ship;
            let (w, h) = (PLAY_W as f32, PLAY_H as f32);
            // Targets: every rock and the saucer, scored by time to reach us.
            let mut best: Option<(f32, f32, f32)> = None; // (urgency, aim angle, distance)
            let mut danger = None;
            let mut consider = |x: f32, y: f32, vx: f32, vy: f32, r: f32| {
                let (dx, dy) = (wd(x - s.x, w), wd(y - s.y, h));
                let dist = (dx * dx + dy * dy).sqrt();
                let tt = dist / BULLET_SPEED;
                let (lx, ly) = (dx + (vx - s.vx) * tt, dy + (vy - s.vy) * tt);
                let aim = lx.atan2(-ly);
                let closing = -(dx * (vx - s.vx) + dy * (vy - s.vy)) / dist.max(1.0);
                let urgency = dist - r - closing * 1.5;
                if best.map_or(true, |b| urgency < b.0) { best = Some((urgency, aim, dist)); }
                if dist - r < 70.0 && closing > 20.0 { danger = Some((dx, dy)); }
            };
            for a in pool_iter(&g.asteroids) { consider(a.x, a.y, a.vx, a.vy, a.size.radius()); }
            if g.saucer.active { consider(g.saucer.x, g.saucer.y, g.saucer.vx, 0.0, 16.0); }
            let mut k = vec![];
            if let Some((_, aim, dist)) = best {
                let d = blip::angle_diff(s.angle, aim);
                if d > 0.06 { k.push(BLIP_KEY_RIGHT); } else if d < -0.06 { k.push(BLIP_KEY_LEFT); }
                if d.abs() < 0.12 && dist < BULLET_SPEED * BULLET_TTL + 30.0 { k.push(BLIP_KEY_SPACE); }
            }
            if let Some((dx, dy)) = danger {
                // Thrust when the nose points away from the threat.
                let away = (-dx).atan2(dy);
                if blip::angle_diff(s.angle, away).abs() < 1.2 { k.push(BLIP_KEY_UP); }
            }
            k
        }
        _ => vec![],
    };
    blip::bot::hold(&keys);
}
