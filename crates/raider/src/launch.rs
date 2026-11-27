//! The take-off from the carrier before each level: the deck run, the engine coming up to full throttle, the climb.

use super::*;

/// Animate engine start, signal, deck run and climb; the opening wave holds fire until the launch ends.
pub(super) fn update_launch(g: &mut Game, dt: f32, sfx: &Sounds) {
    g.launch_t += dt;
    let t = g.launch_t;
    let (d, climb) = launch_path(t);
    g.launch_climb = climb;
    g.carrier_scale = 1.0 - 0.3 * climb;
    g.player_y = LAUNCH_DECK_Y + (LAUNCH_END_Y - LAUNCH_DECK_Y) * climb;
    let plane_cy = g.player_y + PLAYER_H as f32 / 2.0;
    g.ship_y = plane_cy - (CARRIER_BOW + d * CARRIER_DECK_LEN) * g.carrier_scale;

    // the sea: the ship's own way at first, the plane's once it flies
    let roll_len = LAUNCH_ROLL_END - LAUNCH_SIGNAL_END;
    g.scroll_k = if t < LAUNCH_SIGNAL_END { 0.35 }
        else if t < LAUNCH_ROLL_END { 0.35 + 0.4 * (t - LAUNCH_SIGNAL_END) / roll_len }
        else { 0.75 + 0.25 * climb };
    update_background(g, dt);
    update_wrecks(g, dt, sfx);
    g.airspeed = launch_engine(t, climb);
    if t > 0.3 && t < LAUNCH_START_END + 0.3 { start_up_smoke(g, dt); }

    // on the deck the plane runs the centreline; airborne it is yours
    if t < LAUNCH_ROLL_END + 0.4 {
        g.player_x = ((WIN_W - PLAYER_W) / 2) as f32;
    } else {
        let left  = Stick::Left.held();
        let right = Stick::Right.held();
        if left  { g.player_x -= PLAYER_SPEED * dt; }
        if right { g.player_x += PLAYER_SPEED * dt; }
        g.player_x = clamp(g.player_x, 0.0, (WIN_W - PLAYER_W) as f32);
        let target_bank = if left && !right { -0.30 } else if right && !left { 0.30 } else { 0.0 };
        g.player_bank += (target_bank - g.player_bank) * (dt * 9.0).min(1.0);
    }

    if !g.launch_wave_up && t >= LAUNCH_WAVE_AT {
        spawn_opening_wave(g);
        g.launch_wave_up = true;
    }
    if g.launch_wave_up { move_enemies(g, dt, false, sfx); }

    if t >= LAUNCH_TIME {
        g.airspeed = 1.0;
        g.scroll_k = 1.0;
        g.launch_climb = 1.0;
        g.respawn_grace.start(0.4);
        g.spawn_timer.start(1.5);
        g.state = State::Play;
        #[cfg(not(target_arch = "wasm32"))]
        playtest_setup(g);
    }
}

/// Where on the deck the plane is `t` seconds into the take-off (0 at the bow, negative once airborne), and how far
/// through its climb (0 to 1): waiting for the signal, rolling faster and faster, then up, the sea below appearing to
/// slide by more slowly with height.
pub(super) fn launch_path(t: f32) -> (f32, f32) {
    let roll_len = LAUNCH_ROLL_END - LAUNCH_SIGNAL_END;
    let lift_v = 2.0 * LAUNCH_DECK_POS * CARRIER_DECK_LEN / roll_len; // deck px/s at the bow
    if t < LAUNCH_SIGNAL_END { return (LAUNCH_DECK_POS, 0.0); }
    if t < LAUNCH_ROLL_END {
        let u = (t - LAUNCH_SIGNAL_END) / roll_len;
        return (LAUNCH_DECK_POS * (1.0 - u * u), 0.0);
    }
    let u = t - LAUNCH_ROLL_END;
    let c = blip::smoothstep01(u / (LAUNCH_TIME - LAUNCH_ROLL_END));
    (-(lift_v * u * (1.0 - 0.45 * c)) / CARRIER_DECK_LEN, c)
}

/// The engine note through the take-off: coughing into life, then full throttle, then cruise.
pub(super) fn launch_engine(t: f32, climb: f32) -> f32 {
    if t < 0.9 { 0.1 }
    else if t < LAUNCH_START_END { 0.1 + 0.4 * (t - 0.9) / (LAUNCH_START_END - 0.9) }
    else if t < LAUNCH_SIGNAL_END { 0.5 + 0.9 * (t - LAUNCH_START_END) / (LAUNCH_SIGNAL_END - LAUNCH_START_END) }
    else if t < LAUNCH_ROLL_END { AIRSPEED_MAX }
    else { AIRSPEED_MAX - (AIRSPEED_MAX - 1.0) * climb }
}

/// Start-up smoke from the exhaust stacks either side of the cowling, at deck scale (0.62).
pub(super) fn start_up_smoke(g: &mut Game, dt: f32) {
    g.smoke_t -= dt;
    if g.smoke_t > 0.0 { return; }
    g.smoke_t = 0.07;
    let (cx, ny) = (g.player_x + PLAYER_W as f32 / 2.0, g.player_y + PLAYER_H as f32 * (0.5 - 0.3 * 0.62));
    for side in [-1.0f32, 1.0] {
        pool_spawn(&mut g.puffs, Puff { x: cx + side * 4.5, y: ny, r: 1.3, grow: 8.0, ttl: 0.8, max_ttl: 0.8, fire: false, top: false, active: true });
    }
}

/// Native playtest controls, read as a level starts: `RAIDER_BOSS`, `RAIDER_HP`, `RAIDER_BARRIER` and
/// `RAIDER_ISLAND`; `BLIP_BOT_LEVEL` picks the level.
#[cfg(not(target_arch = "wasm32"))]
pub(super) fn playtest_setup(g: &mut Game) {
    let env = |k: &str| std::env::var(k).ok().and_then(|v| v.parse::<i32>().ok());
    if let Some(n) = env("RAIDER_BOSS") {
        g.sess.level = n.clamp(1, BOSS_SPECS.len() as i32);
        g.wave_kills = g.wave_target;
        // the last boss is met inland, as in a full run
        if g.sess.level == MAX_LEVEL {
            g.batteries = land::lay_batteries();
            g.land_dist = land::COAST_DIST + 1400.0;
        }
    }
    if let Some(hp) = env("RAIDER_HP") { g.health = hp.clamp(1, PLAYER_HEALTH_MAX); }
    if let Some(n) = env("RAIDER_ISLAND") { spawn_island_sized(g, n.clamp(0, 2) as u8); }
    if env("RAIDER_BARRIER").is_some() {
        g.sess.level = g.sess.level.max(BARRIER_MIN_LEVEL);
        g.barrier_timer.start(1.0);
    }
}
