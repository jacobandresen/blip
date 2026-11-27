//! Planes shot down: the wreck each leaves, falling, burning and spinning into the sea.

use super::*;

/// An enemy is down: points, the kill count, its drop, and either a fireball
/// or (`flames`) a wreck that burns all the way to the sea.
pub(super) fn shoot_down(g: &mut Game, ei: usize, sfx: &Sounds, flames: bool) {
    let e = g.enemies[ei];
    g.enemies[ei].active = false;
    blip::bot::add("planes", 1.0);
    let (ecx, ecy) = (e.x + ENEMY_W as f32 / 2.0, e.y + ENEMY_H as f32 / 2.0);
    if flames && pool_iter(&g.wrecks).count() < MAX_WRECKS {
        g.spawn_explosion(ecx, ecy, 0.45, EXPLOSION_ORANGE);
        pool_spawn(&mut g.wrecks, new_wreck(&e));
        play_take(&sfx.enemy_explode, 0.55);
    } else {
        g.spawn_explosion(ecx, ecy, 1.0, EXPLOSION_ORANGE);
        play_take(&sfx.enemy_explode, 1.0);
    }
    let pts = match e.kind { EnemyKind::Grunt => 20, EnemyKind::Weaver => 30, EnemyKind::Ace => 50 };
    g.score_at(ecx, ecy, pts * g.sess.level);
    g.wave_kills += 1;
    // The whole flight shot down, none got away.
    let f = &mut g.flights[e.flight];
    f.downed += 1;
    if f.members >= 2 && f.downed == f.members {
        let bonus = FLIGHT_BONUS * f.members as i32 * g.sess.level;
        g.sess.add_score(bonus);
        g.fx.popup(ecx, ecy - 30.0, &format!("FLIGHT +{bonus}"), BLIP_YELLOW);
        blip::bot::add("flights_down", 1.0);
    }
    if e.kind == EnemyKind::Ace {
        pool_spawn(&mut g.powerups, Powerup { x: e.x, y: e.y, active: true });
    } else if rand_range_f32(0.0, 1.0) < HEALTH_DROP_CHANCE {
        pool_spawn(&mut g.health_pickups, HealthPickup { x: e.x, y: e.y, active: true });
    }
}

/// A burning plane's fall, one of the Fall styles at random. Whatever the
/// style it starts with the plane's own velocity: a wreck keeps flying the
/// way the plane was, and only drag and the turn it falls into change that.
pub(super) fn new_wreck(e: &Enemy) -> Wreck {
    // A banked plane falls into a turn that way; a level one either way.
    let side = if e.bank.abs() > 0.05 { e.bank.signum() } else if rand_range_f32(0.0, 1.0) < 0.5 { -1.0 } else { 1.0 };
    let fall = match rand_range_f32(0.0, 1.0) {
        r if r < 0.30 => Fall::Roll,
        r if r < 0.55 => Fall::Spiral,
        r if r < 0.70 => Fall::FlatSpin,
        r if r < 0.85 => Fall::Dive,
        _ => Fall::Tumble,
    };
    let mut w = Wreck { x: e.x, y: e.y, heading: e.heading, kind: e.kind, active: true,
        course: e.heading, speed: e.speed, fall,
        spin: side * (1.0 + rand_range_f32(0.0, 1.0) * 3.0), bend: side * rand_range_f32(0.0, 1.0) * 0.25,
        wobble: rand_range_f32(0.0, 1.0) * 2.0, dur: WRECK_SECS * (0.75 + rand_range_f32(0.0, 1.0) * 0.55), ..DEAD_WRECK };
    match fall {
        Fall::Roll => {}
        // a descending turn: `spin` is the turn rate, tightening as it falls
        Fall::Spiral => { w.spin = side * (1.4 + rand_range_f32(0.0, 1.0) * 0.8); w.dur *= 1.15; }
        Fall::FlatSpin => { w.spin = side * (7.0 + rand_range_f32(0.0, 1.0) * 4.0); w.dur *= 0.8; }
        Fall::Dive => { w.spin = 0.0; w.bend = 0.0; w.wobble = 0.4 + rand_range_f32(0.0, 1.0) * 0.5; }
        Fall::Tumble => { w.spin = side * (3.0 + rand_range_f32(0.0, 1.0) * 3.0); w.flip_t = 0.2 + rand_range_f32(0.0, 1.0) * 0.3; }
    }
    w
}

/// Advance a wreck's inherited course and speed; drag, turning and rotation continue as it falls behind the scrolling ground.
pub(super) fn fly_wreck(w: &mut Wreck, scroll: f32, dt: f32) {
    let drag = match w.fall {
        Fall::Dive => -0.15,     // nose down, gravity outruns drag
        Fall::Spiral => 0.1,
        Fall::Roll => 0.35,
        Fall::Tumble => 0.8,
        Fall::FlatSpin => 1.3,   // a flat spin bleeds speed fast
    };
    w.speed *= 1.0 - drag * dt;
    match w.fall {
        Fall::Spiral => {
            // the turn tightens as the spiral winds down; the nose follows the path
            w.spin *= 1.0 + 0.45 * dt;
            w.course += w.spin * dt;
            w.heading = w.course + w.spin.signum() * 0.25;
        }
        Fall::Dive => {
            w.heading = w.course + (w.t * 6.0).sin() * 0.08 * w.wobble;
        }
        Fall::Tumble => {
            w.flip_t -= dt;
            if w.flip_t <= 0.0 {
                w.spin = -w.spin * (0.7 + rand_range_f32(0.0, 1.0) * 0.6);
                w.flip_t = 0.15 + rand_range_f32(0.0, 1.0) * 0.35;
            }
            w.heading += w.spin * dt;
            w.course += w.bend * dt;
        }
        Fall::Roll | Fall::FlatSpin => {
            w.spin *= 1.0 + 0.8 * dt; // the rotation tightens as it falls
            w.heading += (w.spin + (w.t * 5.0).sin() * w.wobble) * dt;
            w.course += w.bend * dt;
        }
    }
    // height lost, as a share of the fall: the lower, the more it lags with the ground
    let low = (w.t / w.dur).min(1.0);
    w.x += w.course.sin() * w.speed * dt;
    w.y += w.course.cos() * w.speed * dt + SEA_SCROLL_SPEED * scroll * low * dt;
}

pub(super) fn update_wrecks(g: &mut Game, dt: f32, sfx: &Sounds) {
    burn_wrecks(g, dt);
    wrecks_collide(g, sfx);
    drift_puffs(g, dt);
}

/// Each wreck falls, trailing smoke and flame, and ends in a splash, or a fireball over land.
fn burn_wrecks(g: &mut Game, dt: f32) {
    for i in 0..MAX_WRECKS {
        if !g.wrecks[i].active { continue; }
        let scroll = g.scroll_k;
        let w = &mut g.wrecks[i];
        w.t += dt;
        fly_wreck(w, scroll, dt);
        w.puff_t -= dt;
        let (cx, cy, k) = (w.x + ENEMY_W as f32 / 2.0, w.y + ENEMY_H as f32 / 2.0, w.t / w.dur);
        let (emit, done) = (w.puff_t <= 0.0, w.t >= w.dur);
        // a diving plane burns hardest, trailing the thickest smoke
        if emit { w.puff_t = if w.fall == Fall::Dive { 0.022 } else { 0.035 }; }
        if done { w.active = false; }
        let s = 1.0 - 0.55 * k;
        if emit && !done {
            let j = || (rand_range_f32(0.0, 1.0) - 0.5) * 4.0;
            pool_spawn(&mut g.puffs, Puff { x: cx + j(), y: cy + j(), r: 2.5 * s, grow: 16.0 * s,
                ttl: 0.9, max_ttl: 0.9, fire: false, top: false, active: true });
            pool_spawn(&mut g.puffs, Puff { x: cx + j(), y: cy + j(), r: 3.4 * s, grow: -5.0,
                ttl: 0.22, max_ttl: 0.22, fire: true, top: false, active: true });
        }
        if done {
            // the splash where it goes in, or the fireball over land
            let land = land::over_land(g, cx, cy);
            let c = if land { EXPLOSION_ORANGE } else { BlipColor::new(0.85, 0.93, 1.0, 1.0) };
            g.spawn_explosion(cx, cy, if land { 1.2 } else { 0.8 }, c);
        }
    }
}

/// A burning plane still at altitude brings down any plane it crosses, and that one burns too. Low in its fall it is
/// under them.
fn wrecks_collide(g: &mut Game, sfx: &Sounds) {
    for wi in 0..MAX_WRECKS {
        let w = g.wrecks[wi];
        if !w.active || w.t / w.dur > 0.6 { continue; }
        let s = 1.0 - 0.55 * w.t / w.dur;
        let (ww, wh) = (ENEMY_W as f32 * s * 0.7, ENEMY_H as f32 * s * 0.7);
        let (wx, wy) = (w.x + (ENEMY_W as f32 - ww) / 2.0, w.y + (ENEMY_H as f32 - wh) / 2.0);
        for ei in 0..MAX_ENEMIES {
            let e = g.enemies[ei];
            if e.active && rects_overlap(wx, wy, ww, wh, e.x + 4.0, e.y + 4.0,
                ENEMY_W as f32 - 8.0, ENEMY_H as f32 - 8.0) {
                shoot_down(g, ei, sfx, true);
            }
        }
    }
}

/// Smoke drifts with the sea, swelling as it fades.
fn drift_puffs(g: &mut Game, dt: f32) {
    let sc = SEA_SCROLL_SPEED * g.scroll_k;
    for p in pool_iter_mut(&mut g.puffs) {
        p.y += sc * dt;
        p.r = (p.r + p.grow * dt).max(0.5);
        p.ttl -= dt;
        if p.ttl <= 0.0 { p.active = false; }
    }
}
