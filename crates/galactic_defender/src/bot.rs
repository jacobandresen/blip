//! Native-only autopilot (BLIP_BOT=1): step out from under bombs and the
//! UFO's beam, otherwise sit under the lowest invader and hold fire.

use super::*;
use blip::input::{BLIP_KEY_LEFT, BLIP_KEY_RIGHT};

pub fn drive(g: &Game, t: f32) {
    let tap = (t * 10.0) as i32 % 2 == 0;
    let keys: Vec<_> = match g.state {
        State::Title => if tap { vec![BLIP_KEY_SPACE] } else { vec![] },
        State::Play => {
            let w = ALIEN_W as f32;
            let me = g.player_x + w / 2.0;
            let ship_y = (GROUND_Y - 28) as f32;
            // Anything that will be over the cannon's row within 0.7 s.
            let threat = |x0: f32, x1: f32| {
                g.bullets[MAX_PLAYER_BULLETS..].iter().any(|b| {
                    b.active && b.y < ship_y + 20.0 && (ship_y - b.y) / g.bomb_speed < 0.7
                        && b.x + 4.0 > x0 && b.x - 4.0 < x1
                }) || (matches!(g.ufo_mode, UfoMode::Charging | UfoMode::Firing) && g.ufo_active
                    && (g.ufo_laser_x - (x0 + x1) / 2.0).abs() < LASER_HIT_W)
                // A diver, with the room it can still steer across.
                || g.diver.is_some_and(|d| {
                    let secs = ((ship_y - d.y) / DIVE_SPEED).max(0.0);
                    let reach = 8.0 + secs.min(1.0) * DIVE_STEER;
                    d.x + w + reach > x0 && d.x - reach < x1
                })
            };
            let span = |c: f32| (c - w / 2.0 - 6.0, c + w / 2.0 + 6.0);
            let (a, b) = span(me);
            let target = if threat(a, b) {
                // The nearest spot either side that is clear.
                (1..20).flat_map(|k| [me - k as f32 * 12.0, me + k as f32 * 12.0])
                    .find(|&c| c > w / 2.0 && c < WIN_W as f32 - w / 2.0 && { let (a, b) = span(c); !threat(a, b) })
                    .unwrap_or(me)
            } else if g.boss_active {
                g.boss_x + BOSS_W / 2.0
            } else {
                // The lowest invader near by whose column is not behind a shield.
                let shielded = |x: f32| g.shields.iter().any(|s| {
                    (0..SHIELD_COLS).any(|c| (0..SHIELD_ROWS).any(|r| s.alive[r][c]
                        && (s.x + (c as i32 * SHIELD_BLOCK) as f32 + SHIELD_BLOCK as f32 / 2.0 - x).abs() < SHIELD_BLOCK as f32))
                });
                let score = |a: &Alien| a.y - (a.x - me).abs() * 0.3 - if shielded(a.x + w / 2.0) { 400.0 } else { 0.0 };
                g.aliens.iter().filter(|a| a.alive)
                    .max_by(|p, q| score(p).total_cmp(&score(q)))
                    .map_or(me, |a| a.x + w / 2.0)
            };
            let mut k = vec![BLIP_KEY_SPACE];
            if target < me - 3.0 { k.push(BLIP_KEY_LEFT); } else if target > me + 3.0 { k.push(BLIP_KEY_RIGHT); }
            k
        }
        _ => vec![],
    };
    blip::bot::hold(&keys);
}
