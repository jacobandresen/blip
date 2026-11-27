//! The players: running and jumping round the room, the dust they kick up, and being caught by a monster, a shot or
//! the skull.

use super::*;

#[allow(clippy::too_many_arguments)]
pub(super) fn player_dust(g: &mut Game, x: f32, y: f32, speed: (f32, f32), lift: (f32, f32), life: f32, alpha: f32, size: f32) {
    for side in [-1.0, 1.0] {
        g.parts.push(blip::EffectParticle {
            x,
            y,
            vx: side * blip::rand_range_f32(speed.0, speed.1),
            vy: -blip::rand_range_f32(lift.0, lift.1),
            life,
            max_life: life,
            data: ParticleStyle { c: rgba(1.0, 1.0, 1.0, alpha), size, star: false, ring: false },
        });
    }
}

pub(super) fn move_players(g: &mut Game, inp: [Input; 2], dt: f32, sfx: &Sounds) {
    for (i, k) in inp.iter().enumerate() {
        if !g.p[i].joined { continue; }
        if g.p[i].dead_t > 0.0 {
            fall_out(&mut g.p[i], dt);
            continue;
        }
        if !g.p[i].alive { continue; }
        run_and_jump(g, i, k, dt, sfx);
        tick_face(&mut g.p[i], dt);
        if k.blow && g.p[i].blow_cd <= 0.0 && g.bubbles.len() < MAX_BUBBLES { blow_bubble(g, i, sfx); }
    }
}

/// Caught, a player is knocked up and falls out of the room; then the next heart, or out.
fn fall_out(p: &mut Player, dt: f32) {
    p.dead_t -= dt;
    p.vy += GRAV * 0.5 * dt;
    p.y += p.vy * dt;
    if p.dead_t > 0.0 { return; }
    p.lives -= 1;
    if p.lives > 0 { p.place(); } else { p.alive = false; }
}

/// Runs toward the stick, jumps from the ground (a puff of dust, a squash), and lands (another, if it fell hard).
fn run_and_jump(g: &mut Game, i: usize, k: &Input, dt: f32, sfx: &Sounds) {
    let (mut x, mut y, mut vx, mut vy) = (g.p[i].x, g.p[i].y, g.p[i].vx, g.p[i].vy);
    let dir = k.right as i32 as f32 - k.left as i32 as f32;
    vx += (dir * RUN - vx) * (18.0 * dt).min(1.0);
    if dir != 0.0 { g.p[i].face = dir; }
    if k.jump && g.p[i].on_ground {
        vy = -JUMP_V;
        g.p[i].squash = 1.35;
        let (fx, fy) = (x + P_W / 2.0, y + P_H);
        player_dust(g, fx, fy, (40.0, 70.0), (5.0, 20.0), 0.25, 0.45, 1.8);
        play_sfx_volume(&sfx.jump, 0.5);
    }
    let was_air = !g.p[i].on_ground;
    let fall_speed = vy;
    let (ground, _) = g.move_body(&mut x, &mut y, &mut vx, &mut vy, P_W, P_H, dt);
    if ground && was_air && fall_speed > 150.0 {
        g.p[i].squash = 0.7;
        player_dust(g, x + P_W / 2.0, y + P_H, (30.0, 60.0), (10.0, 30.0), 0.3, 0.5, 2.0);
    }
    let p = &mut g.p[i];
    p.x = x;
    p.y = y;
    p.vx = vx;
    p.vy = vy;
    p.on_ground = ground;
    p.squash += (1.0 - p.squash) * (12.0 * dt).min(1.0);
    p.walk += vx.abs() * dt * 0.09;
}

/// The face's clocks: the bubble's cooldown, the mouth, chewing, the safe spell after a respawn, and blinking.
fn tick_face(p: &mut Player, dt: f32) {
    p.blow_cd -= dt;
    p.mouth = (p.mouth - dt).max(0.0);
    if p.eat >= 0.0 {
        p.eat += dt;
        if p.eat > EAT_SECS { p.eat = -1.0; }
    }
    p.safe = (p.safe - dt).max(0.0);
    p.blink -= dt;
    if p.blink < -0.12 { p.blink = blip::rand_range_f32(2.0, 4.5); }
}

/// Player `i` blows a bubble from the mouth, the way they face.
fn blow_bubble(g: &mut Game, i: usize, sfx: &Sounds) {
    let p = &mut g.p[i];
    p.blow_cd = BLOW_CD;
    p.mouth = 0.18;
    let (bx, by, f) = (p.cx() + p.face * 14.0, p.cy() - 1.0, p.face);
    g.bubbles.push(Bubble {
        x: bx,
        y: by,
        vx: f * SHOOT_V,
        age: 0.0,
        phase: Phase::Shoot,
        owner: i,
        trapped: None,
        active: true,
        wob: blip::rand_range_f32(0.0, 6.0),
        squish: 0.0,
    });
    play_sfx_volume(&sfx.blow, 0.6);
}

pub(super) fn hurt_players(
    g: &mut Game,
    sfx: &Sounds,
    skull_motion: Option<((f32, f32), (f32, f32))>,
    shot_hits: [bool; 2],
    player_starts: [(f32, f32); 2],
    enemy_starts: &[(f32, f32)],
) {
    if blip::bot::flag("GOD") {
        if g.skull.active && g.p.iter().any(|p| p.alive && (g.skull.x - p.cx()).hypot(g.skull.y - p.cy()) < 18.0) {
            g.stats.deaths_skull += 1; // would have been caught
        }
        return;
    }
    for pi in 0..2 {
        let p = &g.p[pi];
        if !p.joined || !p.alive || p.dead_t > 0.0 || p.safe > 0.0 { continue; }
        let (px, py) = (p.cx(), p.cy());
        let from = player_starts[pi];
        let killer = g.enemies.iter().enumerate()
            .find(|(ei, e)| {
                if !e.active || e.pop_in < 1.0 { return false; }
                let end = (e.x + E_W / 2.0, e.y + E_H / 2.0);
                let start = enemy_starts.get(*ei).copied().unwrap_or(end);
                let (rx, ry) = ((E_W + P_W) / 2.0 - 2.0 * CONTACT_INSET.0, (E_H + P_H) / 2.0 - 2.0 * CONTACT_INSET.1);
                blip::sweep_hits(from, (px, py), start, end, (-rx, -ry, 2.0 * rx, 2.0 * ry))
            })
            .map(|(_, e)| e.kind);
        let hit_skull = g.skull.active && match skull_motion {
            Some((s0, s1)) => segment_circle_overlap(s0.0 - from.0, s0.1 - from.1, s1.0 - px, s1.1 - py, 0.0, 0.0, 18.0),
            None => (g.skull.x - px).hypot(g.skull.y - py) < 18.0,
        };
        let hit_shot = shot_hits[pi] || g.shots.iter().any(|s| shot_hits_player(s, px, py));
        if hit_shot { g.shots.retain(|s| !shot_hits_player(s, px, py)); }
        if killer.is_some() || hit_shot || hit_skull { knock_out(g, pi, sfx, killer, hit_skull); }
    }
}

/// Player `pi` is caught (by a monster `killer`, a shot, or the skull): counted, and knocked up out of the room.
pub(super) fn knock_out(g: &mut Game, pi: usize, sfx: &Sounds, killer: Option<Kind>, by_skull: bool) {
    if pi == 0 { blip::bot::add("hit_caught", 1.0); }
    if by_skull { g.stats.deaths_skull += 1; } else { g.stats.deaths_enemy += 1; }
    if let Some(k) = killer { g.stats.deaths_by[k as usize] += 1; }
    if blip::bot::active() { report_death(g, pi); }
    g.stats.deaths_round[g.round] += 1;
    let p = &mut g.p[pi];
    p.dead_t = 1.4;
    p.vy = -260.0;
    play_sfx(&sfx.lose);
    let (x, y) = (p.cx(), p.cy());
    g.burst(x, y, 14, rgba(1.0, 0.5, 0.6, 1.0), 120.0, true);
}

/// A bot run's line on each death: where the player was and the nearest monster.
pub(super) fn report_death(g: &Game, pi: usize) {
    let p = &g.p[pi];
    let e = g.enemies.iter().filter(|e| e.active)
        .min_by(|a, b| (a.x - p.x).hypot(a.y - p.y).total_cmp(&(b.x - p.x).hypot(b.y - p.y)));
    println!(
        "DEATH round={} t={:.1} p{} at ({:.0},{:.0}) v=({:.0},{:.0}) ground={} face={} | {:?}",
        g.round + 1, g.round_t, pi + 1, p.x, p.y, p.vx, p.vy, p.on_ground, p.face,
        e.map(|e| (e.kind, e.x as i32, e.y as i32, e.vx as i32, e.vy as i32, e.angry))
    );
}
