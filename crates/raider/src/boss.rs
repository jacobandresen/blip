//! The boss at the end of each stage: its arrival, turrets, bursts and lasers.

use super::*;

pub(super) fn start_boss(g: &mut Game, sfx: &Sounds) {
    let tier = ((g.sess.level - 1) as usize).min(BOSS_SPECS.len() - 1);
    let spec = &BOSS_SPECS[tier];
    let (bw, bh) = boss_size(tier);
    let mut escort_timer = Timer::default();
    if spec.escorts { escort_timer.start(2.5); }
    let mut turrets = [DEAD_TURRET; MAX_TURRETS];
    let drum = boss_burst_len(tier) * DRUM_BURSTS;
    let mut n = 0;
    let (rle, rte, tle, tte) = BOSS_WINGS[tier];
    let mounts = BOSS_TURRETS[tier].iter().copied().chain(BOSS_WING_GUNS[tier].iter().flat_map(|&t| {
        let mid = (rle + (tle - rle) * t + rte + (tte - rte) * t) / 2.0;
        [(-t, mid), (t, mid)]
    }));
    for (fx, fy) in mounts.take(MAX_TURRETS) {
        // Drums start part-used and the first bursts staggered, so the guns
        // never all fall silent to reload at once.
        let ammo = drum - (n as u8 * 3) % drum;
        turrets[n] = Turret { fx, fy, hp: 6 + tier as i32 * 2, ammo, next_t: 0.6 + n as f32 * 0.13, ..DEAD_TURRET };
        n += 1;
    }
    let mut lasers = [DEAD_LASER; MAX_LASERS];
    for (i, &(fx, fy)) in BOSS_LASERS[tier].iter().enumerate().take(MAX_LASERS) {
        // the two of a pair out of step, so they never fire together
        lasers[i] = Laser { fx, fy, hp: LASER_HP, t: 1.5 + i as f32 * 1.6, ..DEAD_LASER };
    }
    g.boss = Boss {
        turrets,
        n_turrets: n,
        lasers,
        n_lasers: BOSS_LASERS[tier].len().min(MAX_LASERS),
        x: (WIN_W as f32 - bw) / 2.0, // centred: its patrol swings about the middle
        y: -bh,
        active: true,
        entered: false,
        hp: spec.hp, max_hp: spec.hp,
        dir: 1.0,
        tier,
        t: 0.0,
        escort_timer,
    };
    g.boss_intro.start(BOSS_INTRO_TIME);
    // The last fight is over the city: it begins at the top edge now.
    if let Some(cy) = land::coast_y(g) { g.city_from = Some(cy.max(0.0)); }
    play_sfx(&sfx.boss_warning);
}

/// The boss's bank in the draw (radians): it leans into its patrol.
pub(super) fn boss_bank(g: &Game) -> f32 { -g.boss.dir * 0.05 }

/// A turret's centre on screen: the sprite flies nose-down and is drawn
/// rotated by boss_bank() about its centre.
pub(super) fn turret_pos(g: &Game, k: usize) -> (f32, f32) {
    let t = &g.boss.turrets[k];
    mount_pos(g, t.fx, t.fy)
}

/// Where a mount at (`fx`, `fy`) on the boss is on screen (see BOSS_TURRETS).
pub(super) fn mount_pos(g: &Game, fx: f32, fy: f32) -> (f32, f32) {
    let (bw, bh) = boss_size(g.boss.tier);
    let (cx, cy) = (g.boss.x + bw / 2.0, g.boss.y + bh / 2.0);
    let (dx, dy) = (fx * bw / 2.0, (0.5 - fy) * bh);
    let (s, c) = boss_bank(g).sin_cos();
    (cx + dx * c - dy * s, cy + dx * s + dy * c)
}

pub(super) fn laser_radius(tier: usize) -> f32 { turret_radius(tier) * 1.35 }

/// Turn, lock, charge, fire and cool every laser turret.
pub(super) fn update_lasers(g: &mut Game, dt: f32, sfx: &Sounds) {
    let (px, py) = (g.player_x + PLAYER_W as f32 / 2.0, g.player_y + PLAYER_H as f32 / 2.0);
    for i in 0..g.boss.n_lasers {
        let (x, y) = mount_pos(g, g.boss.lasers[i].fx, g.boss.lasers[i].fy);
        let l = &mut g.boss.lasers[i];
        if l.hp <= 0 { continue; }
        l.t -= dt;
        match l.phase {
            LaserPhase::Cooling => {
                let want = (px - x).atan2(py - y);
                let err = blip::angle_diff(l.angle, want);
                l.angle += err.clamp(-LASER_TURN * dt, LASER_TURN * dt);
                if l.t <= 0.0 {
                    l.phase = LaserPhase::Charging;
                    l.t = LASER_CHARGE;
                    play_sfx_volume(&sfx.laser_charge, 0.8);
                }
            }
            LaserPhase::Charging => if l.t <= 0.0 {
                l.phase = LaserPhase::Firing;
                l.t = LASER_FIRE;
                play_sfx_volume(&sfx.laser_fire, 0.9);
            },
            LaserPhase::Firing => if l.t <= 0.0 {
                l.phase = LaserPhase::Cooling;
                l.t = LASER_COOL;
            },
        }
    }
}

/// Is the player's hit box in a firing laser's beam?
pub(super) fn laser_hits(g: &Game, hx: f32, hy: f32, hw: f32, hh: f32) -> bool {
    let (cx, cy) = (hx + hw / 2.0, hy + hh / 2.0);
    (0..g.boss.n_lasers).any(|i| {
        let l = &g.boss.lasers[i];
        if l.hp <= 0 || l.phase != LaserPhase::Firing { return false; }
        let (x, y) = mount_pos(g, l.fx, l.fy);
        let (s, c) = l.angle.sin_cos();
        let (dx, dy) = (cx - x, cy - y);
        let along = dx * s + dy * c;
        let across = (dx * c - dy * s).abs();
        along > 0.0 && across < LASER_HALF_W + hw.min(hh) * 0.35
    })
}

pub(super) fn turret_radius(tier: usize) -> f32 { 5.6 + tier as f32 * 0.35 }

/// One round from (x, y) at `angle` off straight down.
pub(super) fn boss_round(g: &mut Game, x: f32, y: f32, angle: f32) {
    let (s, c) = angle.sin_cos();
    pool_spawn(&mut g.enemy_bullets, Bullet {
        x, y, vx: s * ENEMY_BULLET_SPEED * 0.95, vy: c * ENEMY_BULLET_SPEED * 0.95, active: true,
    });
}

/// Rounds per turret burst: more every other level.
pub(super) fn boss_burst_len(tier: usize) -> u8 { 2 + tier as u8 / 2 }

/// Half the width of the lane a boss's spray leaves open (see lane_x).
pub(super) const LANE_HALF: f32 = 46.0;

/// Where the open lane through a boss's spray is, at the player's height:
/// drifting slowly from side to side so the way through moves, but never
/// faster than the plane can follow.
pub(super) fn lane_x(t: f32) -> f32 {
    WIN_W as f32 / 2.0 + (t * 0.35).sin() * (WIN_W as f32 * 0.32)
}

/// Seconds between one turret's bursts: steady for a boss, so its rhythm
/// can be learnt. The guns share the old volley rate between them.
pub(super) fn boss_burst_gap(g: &Game) -> f32 {
    let spec = &BOSS_SPECS[g.boss.tier];
    let hp_frac = g.boss.hp as f32 / g.boss.max_hp as f32;
    // The largest boss enrages below half health: its gunners fire twice as often.
    let enraged = g.boss.tier == BOSS_SPECS.len() - 1 && hp_frac <= 0.5;
    let gap = (spec.fire_min + spec.fire_max) / 2.0 * (1.0 + g.boss.n_turrets as f32 * 0.09);
    if enraged { gap * 0.5 } else { gap }
}

/// Traverse, fire and reload every turret. Traverse is rate-limited (about
/// 100 degrees a second), so a fast-moving player can outrun the gunners.
/// Where gun `k` at `at` wants to point `bt` seconds into the fight, the player at `player`: at the player, or in the
/// pattern's fan, sweep or curtain (`fx`: where the gun sits across the boss). The spray keeps a lane open at the
/// player's height: a gun whose rounds would cross inside it swings to the lane's nearer edge.
fn turret_aim(pattern: BossPattern, (x, y): (f32, f32), (px, py): (f32, f32), fx: f32, bt: f32, k: usize) -> f32 {
    let aim = (px - x).atan2(py - y);
    let want = match pattern {
        BossPattern::Aimed => return aim,
        BossPattern::Fan => aim * 0.5 + fx * 0.6,
        BossPattern::Sweep => (bt * 0.8 + k as f32 * 0.7).sin() * 0.9,
        BossPattern::Curtain => fx * 1.1 + (bt * 1.3).sin() * 0.3,
    };
    let lane = lane_x(bt);
    let cross = x + want.tan() * (py - y).max(1.0);
    if (cross - lane).abs() >= LANE_HALF { return want; }
    let edge = lane + LANE_HALF * if cross < lane { -1.0 } else { 1.0 };
    (edge - x).atan2((py - y).max(1.0))
}

pub(super) fn update_turrets(g: &mut Game, dt: f32) {
    let spec = &BOSS_SPECS[g.boss.tier];
    let pattern = spec.patterns[(g.boss.t / PATTERN_SECS) as usize % spec.patterns.len()];
    let (px, py) = (g.player_x + PLAYER_W as f32 / 2.0, g.player_y + PLAYER_H as f32 / 2.0);
    let r = turret_radius(g.boss.tier);
    let gap = boss_burst_gap(g);
    let len = boss_burst_len(g.boss.tier);
    for k in 0..g.boss.n_turrets {
        let (x, y) = turret_pos(g, k);
        let bt = g.boss.t;
        let t = &mut g.boss.turrets[k];
        if t.hp <= 0 {
            t.smoke_t -= dt;
            if t.smoke_t <= 0.0 {
                t.smoke_t = 0.12;
                pool_spawn(&mut g.puffs, Puff { x, y, r: 2.5, grow: 12.0, ttl: 0.8, max_ttl: 0.8, fire: false, top: false, active: true });
            }
            continue;
        }
        let want = turret_aim(pattern, (x, y), (px, py), t.fx, bt, k);
        let err = blip::angle_diff(t.angle, want);
        t.angle += err.clamp(-1.75 * dt, 1.75 * dt);
        t.flash = (t.flash - dt).max(0.0);
        if t.reload_t > 0.0 {
            t.reload_t -= dt;
            if t.reload_t <= 0.0 { t.ammo = len * DRUM_BURSTS; }
            continue;
        }
        t.next_t -= dt;
        // A burst starts on the beat, and only with the barrels on target.
        if t.burst == 0 && t.next_t <= 0.0 && err.abs() < ON_TARGET {
            t.burst = len.min(t.ammo);
            t.burst_t = 0.0;
            t.next_t = gap;
        }
        if t.burst > 0 {
            t.burst_t -= dt;
            if t.burst_t <= 0.0 {
                t.burst -= 1;
                t.ammo -= 1;
                t.burst_t = ROUND_GAP;
                t.flash = 0.05;
                let (s, c) = t.angle.sin_cos();
                // twin barrels, firing alternately, straight along them
                let side = if t.burst.is_multiple_of(2) { 1.0 } else { -1.0 };
                let (ox, oy) = (c * 1.8 * side, -s * 1.8 * side);
                let a = t.angle;
                if t.ammo == 0 { t.reload_t = RELOAD_SECS; t.burst = 0; }
                boss_round(g, x + s * r * 2.6 + ox, y + c * r * 2.6 + oy, a);
            }
        }
    }
}

pub(super) fn update_boss(g: &mut Game, dt: f32, sfx: &Sounds) {
    if !g.boss.active { return; }
    // Stay off-screen until the WARNING banner has cleared.
    if g.boss_intro.active() { return; }

    let spec = &BOSS_SPECS[g.boss.tier];
    let (bw, _bh) = boss_size(g.boss.tier);
    g.boss.t += dt;

    let target_y = (HUD_H + 40) as f32;
    if !g.boss.entered {
        g.boss.y = (g.boss.y + 60.0 * dt).min(target_y);
        if g.boss.y >= target_y { g.boss.entered = true; g.boss.t = 0.0; }
        return;
    }

    // A big aeroplane does not zig-zag: it drifts across the sky in long,
    // gentle curves (the bank shows in the draw), a little faster per tier.
    let amp = ((WIN_W as f32 - bw) / 2.0 - 8.0).max(0.0);
    let w = 0.25 + spec.speed / 900.0;
    g.boss.x = (WIN_W as f32 - bw) / 2.0 + amp * (g.boss.t * w).sin();
    g.boss.dir = (g.boss.t * w).cos();
    let dip = if spec.dips { (g.boss.t * 0.5).sin().max(0.0) * 16.0 } else { 0.0 };
    g.boss.y = target_y + dip;

    update_turrets(g, dt);
    update_lasers(g, dt, sfx);
    if spec.escorts && g.boss.escort_timer.tick(dt) {
        spawn_dive_pass(g, EnemyKind::Grunt, Formation::Pair);
        g.boss.escort_timer.start(5.0 + rand_range_f32(0.0, 1.0) * 2.5);
    }
}
