//! How to play Bubbler: pop trapped monsters once they bunch up together (a
//! pop bursts every bubble touching it) or are about to break out; pick up fruit;
//! then face the nearest monster and blow a bubble at it, hopping away
//! from any monster about to walk into the dragon and over thrown rocks.
//! `BLIP_BOT_PLAYERS=2` plays both dragons, `BLIP_BOT_PASSIVE=1` never blows
//! (to reach HURRY and the skull), `BLIP_BOT_GOD=1` cannot be caught.

use super::*;
use blip::bot::{Goal::*, Pad, Playthrough};

/// What the bot must show it can do in Bubbler.
pub const PLAYTHROUGHS: &[Playthrough] = &[
    Playthrough {
        name: "the dragon bubbles monsters and pops them",
        goals: &[AtLeast("traps", 5.0), AtLeast("pops", 5.0)],
        ..Playthrough::DEFAULT
    },
    Playthrough {
        name: "it clears round 1 and picks up the fruit",
        goals: &[AtLeast("level", 2.0), AtLeast("fruit", 3.0)],
        ..Playthrough::DEFAULT
    },
    Playthrough {
        name: "two dragons clear a round together",
        settings: &[("PLAYERS", "2")],
        goals: &[AtLeast("level", 2.0)],
        ..Playthrough::DEFAULT
    },
    Playthrough {
        name: "one pop catches two monsters at once",
        seconds: 240.0,
        goals: &[AtLeast("chain2", 1.0)],
        ..Playthrough::DEFAULT
    },
    Playthrough {
        name: "a round left too long brings HURRY UP and the skull",
        settings: &[("PASSIVE", "1"), ("GOD", "1")],
        goals: &[AtLeast("hurries", 1.0), AtLeast("skulls", 1.0)],
        ..Playthrough::DEFAULT
    },
    Playthrough {
        name: "it clears the last round and wins",
        level: ROUNDS as u32,
        seconds: 240.0,
        goals: &[Ends("won")],
        ..Playthrough::DEFAULT
    },
];

/// What one dragon's autopilot remembers between frames.
#[derive(Default)]
pub struct Mind {
    jump_wait: f32,
    wander_t: f32,
    wander_dir: f32,
}

/// One dragon's wish this frame, as `Input` (presses, not keys).
fn think(g: &Game, pi: usize, mem: &mut Mind) -> Input {
    let mut k = Input::default();
    let passive = blip::bot::flag("PASSIVE");
    let p = &g.p[pi];
    if !p.joined || !p.alive || p.dead_t > 0.0 {
        return k;
    }
    let (px, py) = (p.cx(), p.cy());
    mem.jump_wait -= 1.0 / 60.0;
    // danger: a monster about to walk into us
    // danger: a monster near, or about to be (where it will be a quarter-second on)
    let danger = g
        .enemies
        .iter()
        .filter(|e| e.active)
        .flat_map(|e| [0.0, 0.25].map(|t| (e.x + e.vx * t + E_W / 2.0 - px, e.y + e.vy * t + E_H / 2.0 - py)))
        .find(|(dx, dy)| dx.abs() < 34.0 && dy.abs() < 22.0);
    // trapped bubbles are the priority, when popping them is worth it
    let trapped = g.bubbles.iter().filter(|b| ready_to_pop(g, b)).min_by(|a, b| (a.x - px).abs().total_cmp(&(b.x - px).abs()));
    let fruit = g
        .fruits
        .iter()
        .filter(|f| f.on_ground && (f.y + 8.0 - py).abs() < 30.0)
        .min_by(|a, b| (a.x - px).abs().total_cmp(&(b.x - px).abs()));
    let target = if let Some(b) = trapped {
        Some((b.x, b.y, true))
    } else if let Some(f) = fruit.filter(|f| (f.x - px).abs() < 160.0) {
        Some((f.x + 8.0, py, true))
    } else {
        g.enemies
            .iter()
            .filter(|e| e.active && e.pop_in >= 1.0)
            .min_by(|a, b| ((a.x - px).hypot((a.y - py) * 1.6)).total_cmp(&(b.x - px).hypot((b.y - py) * 1.6)))
            .map(|e| (e.x + E_W / 2.0, e.y + E_H / 2.0, false))
    };
    // a rock or spark about to hit: hop over one along the ground, else step aside
    if let Some(shot) = incoming_shot(g, p) {
        let away = if shot.x > px { -1.0 } else { 1.0 };
        k.left = away < 0.0;
        k.right = away > 0.0;
        k.jump = p.on_ground && shot.vy.abs() < 60.0;
        k.jump_held = k.jump;
        return k;
    }
    // in the air over a monster: steer clear of it
    if !p.on_ground {
        if let Some(e) = g
            .enemies
            .iter()
            .filter(|e| e.active)
            .find(|e| (e.x + E_W / 2.0 - px).abs() < 30.0 && e.y > py && e.y - py < 90.0)
        {
            let away = if e.x + E_W / 2.0 > px { -1.0 } else { 1.0 };
            k.left = away < 0.0;
            k.right = away > 0.0;
            return k;
        }
    }
    if let Some((dx, _)) = danger {
        // hop away / over
        k.left = dx > 0.0;
        k.right = dx < 0.0;
        if p.on_ground && blip::rand_range_f32(0.0, 1.0) < 0.25 {
            k.jump = true;
        }
        // and bubble it if we face it
        if (dx > 0.0) == (p.face > 0.0) && !passive {
            k.blow = true;
        }
        k.jump_held = true;
        return k;
    }
    if let Some((tx, ty, is_bubble)) = target {
        let dx = tx - px;
        let dy = ty - py;
        if is_bubble {
            if dx.abs() > 10.0 {
                k.left = dx < 0.0;
                k.right = dx > 0.0;
            }
            if p.on_ground && (dy < -20.0 || dx.abs() < 40.0) && mem.jump_wait <= 0.0 {
                k.jump = true;
                mem.jump_wait = 0.3;
            }
            k.jump_held = false; // bump into it, do not ride it
        } else {
            let level = dy.abs() < 18.0;
            if level && dx.abs() < 150.0 {
                // face it and blow
                if (dx > 0.0) != (p.face > 0.0) {
                    k.left = dx < 0.0;
                    k.right = dx > 0.0;
                } else if blip::rand_range_f32(0.0, 1.0) < 0.5 && !passive {
                    k.blow = true;
                }
                if dx.abs() > 70.0 {
                    k.left = dx < 0.0;
                    k.right = dx > 0.0;
                }
            } else {
                if dx.abs() > 30.0 || dy > 0.0 {
                    // head for it; to go down walk off an edge, to go up jump
                    let dir = if mem.wander_t > 0.0 { mem.wander_dir } else { dx.signum() };
                    k.left = dir < 0.0;
                    k.right = dir > 0.0;
                }
                if dy < -30.0 && p.on_ground && mem.jump_wait <= 0.0 && blip::rand_range_f32(0.0, 1.0) < 0.12 {
                    k.jump = true;
                    mem.jump_wait = 0.4;
                }
            }
        }
        // stuck against a wall or under a ledge: wander a bit
        mem.wander_t -= 1.0 / 60.0;
        if (p.vx.abs() < 5.0 && (k.left || k.right)) && mem.wander_t <= 0.0 {
            mem.wander_t = 0.8;
            mem.wander_dir = if blip::rand_range_f32(0.0, 1.0) < 0.5 { -1.0 } else { 1.0 };
        }
    }
    k.jump_held = k.jump_held || k.jump;
    k
}

/// A rock or spark that will hit the dragon within 0.4 s, both moving as they are.
fn incoming_shot<'a>(g: &'a Game, p: &Player) -> Option<&'a Shot> {
    g.shots.iter().filter(|s| s.active).find(|s| {
        (0..=8).any(|k| {
            let t = k as f32 * 0.05;
            let (dx, dy) = (s.x + s.vx * t - (p.cx() + p.vx * t), s.y + s.vy * t - (p.cy() + p.vy * t));
            dx.abs() < P_W / 2.0 + 10.0 && dy.abs() < P_H / 2.0 + 8.0
        })
    })
}

/// Pop a trapped monster now? When it touches another trapped one (one pop
/// takes both), when it is about to break out, or when no monster is left free.
fn ready_to_pop(g: &Game, b: &Bubble) -> bool {
    if b.trapped.is_none() { return false; }
    let touching = g.bubbles.iter().any(|o| o.trapped.is_some() && !std::ptr::eq(o, b)
        && (o.x - b.x).hypot(o.y - b.y) < TRAP_R * 2.4);
    let breaking_out = b.age > trap_life(g.round) - 3.0;
    let none_free = !g.enemies.iter().any(|e| e.active);
    touching || breaking_out || none_free
}

/// This frame's controls: both dragons' wishes, through their own key sets.
pub fn drive(g: &Game, minds: &mut [Mind; 2]) {
    let mut pad = Pad::new();
    if g.state == State::Title {
        // P2's blow key starts a two-player game, P1's a solo one.
        let two = blip::bot::num("PLAYERS") == Some(2.0);
        pad.tap(if two { P2_KEYS.blow[0] } else { P1_KEYS.blow[0] });
    } else {
        for (pi, keys) in [&P1_KEYS, &P2_KEYS].into_iter().enumerate() {
            let k = think(g, pi, &mut minds[pi]);
            if k.left { pad.hold(keys.left[0]); }
            if k.right { pad.hold(keys.right[0]); }
            if k.jump { pad.tap(keys.jump[0]); } else if k.jump_held { pad.hold(keys.jump[0]); }
            if k.blow { pad.tap(keys.blow[0]); }
        }
    }
    pad.send();
}

/// The run's tallies, for the RESULT line.
pub fn report(g: &Game) {
    use blip::bot::set;
    let st = &g.stats;
    set("score", (g.p[0].score + g.p[1].score) as f64);
    set("deaths", (st.deaths_enemy + st.deaths_skull) as f64);
    for (name, n) in [
        ("traps", st.traps), ("escapes", st.escapes), ("pops", st.pops), ("kills", st.kills),
        ("fruit", st.fruit), ("deaths_enemy", st.deaths_enemy), ("deaths_skull", st.deaths_skull),
        ("hurries", st.hurries), ("skulls", st.skulls), ("bounces", st.bounces),
    ] {
        set(name, n as f64);
    }
    // chains[n]: pops that caught n monsters at once.
    for (n, count) in st.chains.iter().enumerate().skip(2).filter(|(_, c)| **c > 0) {
        set(&format!("chain{n}"), *count as f64);
    }
    for (kind, n) in ["walker", "hopper", "ghost"].iter().zip(st.deaths_by) {
        if n > 0 { set(&format!("death_{kind}"), n as f64); }
    }
    for (r, n) in st.deaths_round.iter().enumerate().filter(|(_, n)| **n > 0) {
        set(&format!("death_round{}", r + 1), *n as f64);
    }
}
