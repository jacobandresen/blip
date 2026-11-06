//! How to play Defender: keep firing under the lowest invader not hidden
//! behind a shield, leading the march (or at a high diver, the mothership, or
//! the saucer when its column is clear). Step out from under
//! the saucer's beam and a diver about to land. Against bombs it looks ahead:
//! it tries short moves (hold left or right a moment, then let go), plays
//! each a second ahead with the game's own cannon and bomb hit test, and
//! takes the safe one that ends nearest where it wants to be. Under a low
//! column a bomb lands too soon to dodge, so it passes under one only to shoot it.

use super::*;
use blip::bot::{Goal::*, Pad, Playthrough};

/// What the bot must show it can do in Defender.
pub const PLAYTHROUGHS: &[Playthrough] = &[
    Playthrough {
        name: "the cannon shoots down the invading rows to clear wave 1",
        seconds: 180.0,
        goals: &[AtLeast("invaders", 30.0), AtLeast("level", 2.0)],
        ..Playthrough::DEFAULT
    },
    Playthrough {
        name: "it shoots the saucer as it crosses",
        seconds: 240.0,
        goals: &[AtLeast("saucers", 1.0)],
        ..Playthrough::DEFAULT
    },
    Playthrough {
        name: "it dodges and shoots the divers from wave 3",
        level: DIVE_LEVEL as u32,
        seconds: 240.0,
        goals: &[AtLeast("divers", 1.0)],
        ..Playthrough::DEFAULT
    },
    Playthrough {
        name: "it brings down the mothership on wave 5",
        level: BOSS_EVERY as u32,
        seconds: 240.0,
        goals: &[AtLeast("motherships", 1.0)],
        ..Playthrough::DEFAULT
    },
];

pub fn drive(g: &Game) {
    let mut pad = Pad::new();
    match g.state {
        State::Title => { pad.press_start(); }
        State::Play => {
            let me = g.player_x + ALIEN_W as f32 / 2.0;
            let want = if in_danger(g, me) { nearest_safe_spot(g, me) } else { aim_point(g, me) };
            pad.fire();
            match dodge_bombs(g, want) {
                Some(dir) => { pad.steer(0.0, dir as f32, 0.5); }
                None => { pad.steer(me, want, 3.0); }
            }
        }
        _ => {}
    }
    pad.send();
}

/// A short move: hold `dir` (-1 left, 1 right) for `frames`, then let go.
type Move = (i32, u32);

/// A bomb dropped from lower than this lands too soon to dodge.
const POINT_BLANK_SECS: f32 = 0.4;

/// Where bombs would fall from the columns low enough to bomb point blank.
fn point_blank_lines(g: &Game) -> Vec<f32> {
    let ship_y = (GROUND_Y - 28) as f32;
    (0..g.active_cols).filter_map(|c| {
        (0..g.active_rows).rev().map(|r| &g.aliens[(r * g.active_cols + c) as usize]).find(|a| a.alive)
    })
    .filter(|a| (ship_y - (a.y + ALIEN_H as f32)) / g.bomb_speed < POINT_BLANK_SECS)
    .map(|a| a.x + (ALIEN_W / 2 - 4) as f32)
    .collect()
}

/// With a bomb on its way, or a low column overhead: the first step of the
/// safest short move that ends nearest `want`. `None` with nothing to fear.
fn dodge_bombs(g: &Game, want: f32) -> Option<i32> {
    let falling = g.bullets[MAX_PLAYER_BULLETS..].iter().any(|b| b.active && b.y > (GROUND_Y - 28) as f32 - g.bomb_speed);
    // With the shot away, keep out from under low columns until it is back.
    // The column it is shooting is worth the risk: each drops a bomb only now and then.
    let lines: Vec<f32> = if g.bullets[0].active { point_blank_lines(g) } else { Vec::new() };
    let lines: Vec<f32> = lines.into_iter().filter(|l| (l + 4.0 - want).abs() > ALIEN_W as f32 / 2.0).collect();
    if !falling && lines.is_empty() { return None; }
    let exposed = |x: f32| lines.iter().any(|&l| bomb_hits_ship(l, (GROUND_Y - 20) as f32, x - ALIEN_W as f32 / 2.0));
    let moves: Vec<Move> = [(0, 0)].into_iter()
        .chain([-1, 1].into_iter().flat_map(|d| [3, 6, 10, 15, 22, 30, 45].map(|f| (d, f))))
        .collect();
    let outcome = |m: Move| {
        let (hit_at, end_x) = play_ahead(g, m);
        // Safe moves first, out from under low columns, ending nearest `want`; else the latest hit.
        (hit_at.is_some(), -hit_at.unwrap_or(0.0), exposed(end_x), (end_x - want).abs())
    };
    let best = moves.into_iter().min_by(|a, b| outcome(*a).partial_cmp(&outcome(*b)).unwrap())?;
    Some(if best.1 > 0 { best.0 } else { 0 })
}

/// Play `m` one second ahead with the game's cannon (it spools up and
/// brakes) and its bomb hit test: when a bomb would hit, and where the
/// cannon's centre ends up.
fn play_ahead(g: &Game, (dir, frames): Move) -> (Option<f32>, f32) {
    let dt = 1.0 / 60.0;
    let (mut x, mut vx) = (g.player_x, g.player_vx);
    let max_x = (WIN_W - ALIEN_W) as f32;
    let bombs: Vec<_> = g.bullets[MAX_PLAYER_BULLETS..].iter().filter(|b| b.active).map(|b| (b.x, b.y)).collect();
    for f in 0..60 {
        let target = if f < frames { dir as f32 * PLAYER_SPEED } else { 0.0 };
        let rate = if target == 0.0 || target * vx < 0.0 { PLAYER_BRAKE } else { PLAYER_ACCEL } * dt;
        vx += (target - vx).clamp(-rate, rate);
        x = (x + vx * dt).clamp(0.0, max_x);
        let t = (f + 1) as f32 * dt;
        if bombs.iter().any(|&(bx, by)| bomb_hits_ship(bx, by + g.bomb_speed * t, x)) {
            return (Some(t), x + ALIEN_W as f32 / 2.0);
        }
    }
    (None, x + ALIEN_W as f32 / 2.0)
}

/// Where a bomb at `x` would hit the cannon: the base of its wedge (half the
/// sprite) and a little margin.
fn span(x: f32) -> (f32, f32) {
    let half = ALIEN_W as f32 / 2.0 + 2.0;
    (x - half, x + half)
}

/// A diver closer than this is dodged; a higher one is shot at.
const DIVER_DODGE_SECS: f32 = 0.8;

/// Will anything be over the cannon at `x` within 0.7 s: a bomb, the
/// saucer's beam, or a diver about to land, with the room it can still steer across?
fn in_danger(g: &Game, x: f32) -> bool {
    let (x0, x1) = span(x);
    let ship_y = (GROUND_Y - 28) as f32;
    let bomb = g.bullets[MAX_PLAYER_BULLETS..].iter().any(|b| {
        b.active && b.y < ship_y + 20.0 && (ship_y - b.y) / g.bomb_speed < 0.7 && b.x + 4.0 > x0 && b.x - 4.0 < x1
    });
    // The beam fires from where the saucer is when its charge runs out.
    let beam_x = if g.ufo_mode == UfoMode::Firing { g.ufo_laser_x } else { g.ufo_x } + UFO_W / 2.0;
    let beam = matches!(g.ufo_mode, UfoMode::Charging | UfoMode::Firing) && g.ufo_active
        && (beam_x - x).abs() < LASER_HIT_W / 2.0 + ALIEN_W as f32 / 2.0 + 6.0;
    let diver = g.diver.is_some_and(|d| {
        let secs = ((ship_y - d.y) / DIVE_SPEED).max(0.0);
        let reach = 8.0 + secs * DIVE_STEER;
        secs < DIVER_DODGE_SECS && d.x + ALIEN_W as f32 + reach > x0 && d.x - reach < x1
    });
    bomb || beam || diver
}

/// The nearest clear spot either side that the cannon can reach without
/// passing under a bomb on the way.
fn nearest_safe_spot(g: &Game, me: f32) -> f32 {
    let half = ALIEN_W as f32 / 2.0;
    (1..30).flat_map(|k| [me - k as f32 * 12.0, me + k as f32 * 12.0])
        .find(|&x| x > half && x < WIN_W as f32 - half && !in_danger(g, x) && way_clear(g, me, x))
        .unwrap_or(me)
}

/// Driving from `me` to `to` at full speed: is the cannon ever under a bomb
/// as it reaches the cannon's row?
fn way_clear(g: &Game, me: f32, to: f32) -> bool {
    let ship_y = (GROUND_Y - 28) as f32;
    let (x0, x1) = span(0.0);
    g.bullets[MAX_PLAYER_BULLETS..].iter().filter(|b| b.active && b.y < ship_y + 20.0).all(|b| {
        let lands = (ship_y - b.y) / g.bomb_speed;
        let travelled = (PLAYER_SPEED * lands).min((to - me).abs());
        let there = me + travelled * (to - me).signum();
        lands > 1.2 || b.x + 4.0 < there + x0 || b.x - 4.0 > there + x1
    })
}

/// Under a high diver, the mothership, or the lowest nearby invader with a clear shot.
fn aim_point(g: &Game, me: f32) -> f32 {
    let half = ALIEN_W as f32 / 2.0;
    if let Some(d) = g.diver { return d.x + half; }
    if g.boss_active { return g.boss_x + BOSS_W / 2.0; }
    if let Some(x) = saucer_shot(g) { return x; }
    let worth = |a: &Alien| a.y - (a.x - me).abs() * 0.3 - if behind_shield(g, a.x + half) { 400.0 } else { 0.0 };
    g.aliens.iter().filter(|a| a.alive).max_by(|p, q| worth(p).total_cmp(&worth(q))).map_or(me, |a| a.x + half + lead(g, a))
}

/// Where to stand to hit the crossing saucer, leading it, if that column is
/// clear of invaders and shields (a shot stops at the first thing it meets).
fn saucer_shot(g: &Game) -> Option<f32> {
    if !g.ufo_active || g.ufo_mode != UfoMode::Flying { return None; }
    let climb = ((GROUND_Y - 28) as f32 - UFO_Y) / BULLET_SPEED;
    let x = g.ufo_x + UFO_W / 2.0 + 80.0 * g.ufo_dir as f32 * climb;
    let half = ALIEN_W as f32 / 2.0;
    let blocked = g.aliens.iter().any(|a| a.alive && (a.x + half - x).abs() < half) || behind_shield(g, x);
    (x > half && x < WIN_W as f32 - half && !blocked).then_some(x)
}

/// How far the formation will march while a shot climbs to `a`: it steps a
/// third of an invader every `march_interval`, faster as it thins out.
fn lead(g: &Game, a: &Alien) -> f32 {
    if g.march_drop_next { return 0.0; }
    let speed = (ALIEN_W / 3) as f32 / (g.march_interval() / 1000.0) * g.march_dir as f32;
    let climb = ((GROUND_Y - 28) as f32 - (a.y + ALIEN_H as f32)) / BULLET_SPEED;
    speed * climb
}

/// Is a shield block standing over column `x`?
fn behind_shield(g: &Game, x: f32) -> bool {
    let block = SHIELD_BLOCK as f32;
    g.shields.iter().any(|s| (0..SHIELD_COLS).any(|c| (0..SHIELD_ROWS).any(|r| {
        s.alive[r][c] && (s.x + c as f32 * block + block / 2.0 - x).abs() < block
    })))
}
