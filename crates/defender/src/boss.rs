//! The mothership on every BOSS_EVERY-th level: its sway, its volleys, and the rounds that wear it down.

use super::*;

impl Game {
    pub(super) fn is_boss_level(&self) -> bool { self.sess.level % BOSS_EVERY == 0 }

    /// How many motherships deep this run is — the first is 1.
    pub(super) fn boss_number(&self) -> i32 { (self.sess.level / BOSS_EVERY).max(1) }

    pub(super) fn start_boss(&mut self) {
        let n = self.boss_number();
        self.boss_hp_max = BOSS_HP_BASE + BOSS_HP_STEP * (n - 1);
        self.boss_hp = self.boss_hp_max;
        self.boss_dir = if (rand() & 1) == 0 { 1 } else { -1 };
        self.boss_x = if self.boss_dir == 1 { -BOSS_W } else { WIN_W as f32 };
        self.boss_phase = 0.0;
        self.boss_active = true;
        // Comes in shooting, but not instantly: the player has just
        // cleared a formation and needs the moment it takes to read what
        // has arrived before the first volley lands.
        self.boss_fire.start(1.4);
    }

    pub(super) fn boss_speed(&self) -> f32 {
        BOSS_SPEED_BASE + BOSS_SPEED_STEP * (self.boss_number() - 1) as f32
    }

    pub(super) fn boss_volley_gap(&self) -> f32 {
        let gap = BOSS_VOLLEY_MIN - BOSS_VOLLEY_STEP * (self.boss_number() - 1) as f32;
        if gap < BOSS_VOLLEY_FLOOR { BOSS_VOLLEY_FLOOR } else { gap }
    }

    pub(super) fn boss_y(&self) -> f32 {
        // Rides a slow sine as it crosses, so its bombs do not all start
        // from the same height and the silhouette never sits still.
        BOSS_Y + (self.boss_phase.sin()) * BOSS_SWAY
    }
}

/// The mothership: cross, sway, drop volleys. Its bombs use the alien bomb
/// pool, free by then, so they are the bombs the player already knows, with
/// the existing collisions.
pub(super) fn update_boss(g: &mut Game, dt: f32, sfx: &Sounds) {
    if !g.boss_active { return; }

    if g.boss_flash > 0.0 {
        g.boss_flash -= dt;
        if g.boss_flash < 0.0 { g.boss_flash = 0.0; }
    }

    g.boss_phase += dt * 1.6;
    g.boss_x += g.boss_speed() * g.boss_dir as f32 * dt;
    // Turns at the walls rather than flying off: this one does not get to
    // leave. The ordinary UFO is a bonus that passes by; the mothership is
    // the level.
    if g.boss_x < 0.0 { g.boss_x = 0.0; g.boss_dir = 1; }
    if g.boss_x + BOSS_W > WIN_W as f32 { g.boss_x = WIN_W as f32 - BOSS_W; g.boss_dir = -1; }

    g.spin_lights(dt);

    if g.boss_fire.tick(dt) {
        // Three bombs: one under each wingtip and one under the middle.
        // Fired from where the ship actually is, so its sway is something
        // the player can read the volley off rather than noise.
        let y = g.boss_y() + BOSS_H;
        let xs = [g.boss_x + BOSS_W * 0.18, g.boss_x + BOSS_W * 0.5, g.boss_x + BOSS_W * 0.82];
        for x in xs {
            if let Some(i) = g.free_bullet(false) {
                g.bullets[i] = Bullet { x, y, active: true, player: false };
            }
        }
        play_sfx(&sfx.shoot);
        g.boss_fire.start(g.boss_volley_gap());
    }
}

/// Player shots against the mothership: only the player's slots, which
/// free_bullet() hands out from the front of the array (bombs from the back).
pub(super) fn hit_boss(g: &mut Game, sfx: &Sounds, bullet_starts: &[(f32, f32); N_BULLETS]) {
    if !g.boss_active { return; }
    let (bx, by) = (g.boss_x, g.boss_y());
    // `i` pairs the bullet slot with its swept start (the player slots head the array).
    for (i, start) in bullet_starts.iter().enumerate().take(MAX_PLAYER_BULLETS) {
        if !g.bullets[i].active { continue; }
        let (x, y) = (g.bullets[i].x, g.bullets[i].y);
        if !bullet_hits_rect(*start, (x, y), bx, by, BOSS_W, BOSS_H) { continue; }
        g.bullets[i].active = false;
        g.boss_hp -= 1;
        g.boss_flash = BOSS_FLASH;
        g.sess.add_score(BOSS_HIT_SCORE);
        // The puff goes where the shot landed, not at the centre: it is
        // the only feedback saying a hit registered at all, and a boss
        // that swallows shots silently reads as invulnerable.
        g.spawn_explosion_ex(x - 8.0, y - 8.0, EXPLOSION_TTL * 0.7, 1.0);
        play_sfx(&sfx.explosion);
        if g.boss_hp <= 0 {
            g.boss_active = false;
            g.boss_done = true;
            blip::bot::add("motherships", 1.0);
            g.sess.add_score(BOSS_KILL_SCORE * g.sess.level);
            g.spawn_player_death(bx + BOSS_W / 2.0 - ALIEN_W as f32 / 2.0, by + BOSS_H / 2.0);
            g.spawn_explosion_ex(bx + BOSS_W * 0.2, by, EXPLOSION_TTL * 2.0, 2.2);
            g.spawn_explosion_ex(bx + BOSS_W * 0.8, by + BOSS_H * 0.5, EXPLOSION_TTL * 2.2, 2.4);
            blip::stop_alert();
            return;
        }
    }
}
