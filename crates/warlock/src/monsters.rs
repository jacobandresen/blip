//! The monsters: wraiths that flicker in and out of sight, steps along the corridors (lunging, hunting or on patrol),
//! gargoyles' fireballs, the wizards' bolts and lances, and what a kill leaves.

use super::*;

impl Game {
    pub(super) fn update_enemies(&mut self, dt: f32) {
        self.see_wraiths(dt);
        self.step_monsters(dt);
        self.gargoyles_cast(dt);
        // advance render lerp
        for e in &mut self.enemies {
            e.lerp = (e.lerp + dt / e.step).min(1.0);
        }
    }

    /// Wraiths flicker into sight, show near a wizard (lunging), in a reveal's glow and by a cleansed rune.
    pub(super) fn see_wraiths(&mut self, dt: f32) {
        for e in &mut self.enemies {
            e.revealed = (e.revealed - dt).max(0.0);
            if e.kind != EnemyKind::Wraith { continue; }
            e.flash += dt;
            let period = WRAITH_FLASH_ON + WRAITH_FLASH_OFF;
            if e.flash > period { e.flash -= period; }
            let near = (0..2).any(|k| self.active[k] && distance_lt(e.tile, self.wizards[k].tile, WRAITH_LUNGE_RANGE));
            // A fresh lunge first materializes: visible, but still, for a
            // beat — so the reveal is a warning, not an instant pounce.
            if near && !e.lunging { e.charge = WRAITH_CHARGE; }
            e.lunging = near;
            e.charge = (e.charge - dt).max(0.0);
            e.visible = e.revealed > 0.0
                || near
                || e.flash < WRAITH_FLASH_ON
                || self.runes.iter().any(|r| r.cleansed && distance_lt(e.tile, r.tile, RUNE_LIGHT));
        }
    }

    /// Each monster takes a step when its time comes: a lunging wraith toward the nearest wizard, a hunter to where
    /// it heard a reveal, the rest on patrol; decided against a snapshot of occupied tiles. The last few hurry, as the
    /// heartbeat does.
    pub(super) fn step_monsters(&mut self, dt: f32) {
        let occupied: Vec<Cell> = self.enemies.iter().filter(|e| !e.gone).map(|e| e.tile).collect();
        let pace = 0.75 + 0.25 * (occupied.len() as f32 / self.start_monsters.max(1) as f32).min(1.0);
        for i in 0..self.enemies.len() {
            let e = self.enemies[i];
            // (a materializing wraith is visible, but not yet moving)
            if e.gone || e.kind == EnemyKind::Gargoyle || e.kind == EnemyKind::Wraith && e.charge > 0.0 { continue; }
            self.enemies[i].step_t -= dt;
            if self.enemies[i].step_t > 0.0 { continue; }
            let interval = enemy_step(&e) * pace;
            let decision = if e.lunging {
                let target = (0..2)
                    .filter(|&k| self.active[k])
                    .map(|k| self.wizards[k].tile)
                    .min_by_key(|t| (t.c - e.tile.c).abs() + (t.r - e.tile.r).abs())
                    .unwrap_or(e.tile);
                bfs_next(&self.walls, &occupied, e.tile, target)
            } else if e.hunting {
                bfs_next(&self.walls, &occupied, e.tile, e.target)
            } else {
                decide_patrol(&self.walls, &occupied, &e)
            };
            let m = &mut self.enemies[i];
            m.step_t = interval;
            if let Some((next, dir)) = decision {
                match dir {
                    Dir::Left => m.face_left = true,
                    Dir::Right => m.face_left = false,
                    _ => {}
                }
                m.from = e.tile;
                m.tile = next;
                m.dir = dir;
                m.lerp = 0.0;
                m.step = interval;
            }
        }
    }

    /// Gargoyles raise their wings (the tell), then cast a fireball down an open corridor.
    pub(super) fn gargoyles_cast(&mut self, dt: f32) {
        for i in 0..self.enemies.len() {
            let e = self.enemies[i];
            if e.gone || e.kind != EnemyKind::Gargoyle { continue; }
            if e.windup <= 0.0 {
                self.enemies[i].shot_cd -= dt;
                if self.enemies[i].shot_cd <= 0.0 { self.enemies[i].windup = GARGOYLE_TELL; }
                continue;
            }
            self.enemies[i].windup -= dt;
            if self.enemies[i].windup > 0.0 { continue; }
            let dir = pick_open_dir(&self.walls, e.tile);
            self.fires.push(Projectile { proj: Proj::Fireball, tile: e.tile, from: e.tile, lerp: 0.0, dir, gone: false });
            self.enemies[i].shot_cd = GARGOYLE_CD;
            let (x, y) = center(e.tile);
            self.magic.burst(x, y - 2.0, 12, 90.0, FIREBALL, magic::Kind::Spark);
            self.magic.flash(x, y, 80.0, 0.3, FIREBALL);
            self.sfx(Sfx::Fire);
        }
    }

    pub(super) fn update_projectiles(&mut self, dt: f32) {
        self.fly_bolts(dt);
        self.fly_fires(dt);
    }

    /// The wizards' bolts fly down their corridors, leaving a trail: a bolt stops at the first monster it meets (a
    /// lance runs on through them all), at the whelp, or at the Warlock while he stands warded in a circle.
    pub(super) fn fly_bolts(&mut self, dt: f32) {
        let mut kills = Vec::new();
        let mut warlock_hit = false;
        let mut whelp_hit = false;
        for b in self.bolts.iter_mut().filter(|b| !b.gone) {
            b.lerp += dt / if b.proj == Proj::Lance { BOLT_INTERVAL * 0.7 } else { BOLT_INTERVAL };
            let (px, py) = art::proj_px(b);
            self.magic.trail(px, py, BOLT_TRAIL);
            if b.lerp < 1.0 || !cross_cell(b, &self.walls, &mut self.magic, BOLT_TRAIL) { continue; }
            // A moving monster is hit in the cell it leaves as well as the one it enters: testing only the
            // one it enters let a bolt pass through an imp stepping toward the wizard (seed 4 lost a life so).
            if let Some(idx) = self.enemies.iter().position(|e| !e.gone && (e.tile == b.tile || (e.lerp < 1.0 && e.from == b.tile))) {
                if !kills.contains(&idx) { kills.push(idx); }
                if b.proj != Proj::Lance {
                    b.gone = true;
                    continue;
                }
            }
            if self.whelp.as_ref().is_some_and(|w| w.tile == b.tile || w.from == b.tile) {
                whelp_hit = true;
                b.gone = true;
                continue;
            }
            if let Some(wl) = &self.warlock {
                if wl.wards > 0 && wl.state == WState::Pausing && wl.circles[wl.circle] == b.tile {
                    warlock_hit = true;
                    b.gone = true;
                }
            }
        }
        self.bolts.retain(|b| !b.gone);
        for idx in kills { self.kill_enemy(idx); }
        if warlock_hit { self.hit_warlock(); }
        if whelp_hit { self.strike_whelp(); }
    }

    /// Fireballs (the gargoyles' and the Warlock's) roll down their corridors in cinders. One burns a wizard when the
    /// two drawn bodies meet, as monsters do (`check_contact`): testing tiles would hit as it enters her cell, while it
    /// is still drawn a whole cell (32 px) away, and miss her when she steps into its cell instead.
    pub(super) fn fly_fires(&mut self, dt: f32) {
        for f in self.fires.iter_mut().filter(|f| !f.gone) {
            f.lerp += dt / GARGOYLE_SHOT_STEP;
            let (px, py) = art::proj_px(f);
            let ember = if f.proj == Proj::Warlock { WARLOCK_C } else { FIREBALL };
            self.magic.cinders(px, py, ember);
            if f.lerp >= 1.0 { cross_cell(f, &self.walls, &mut self.magic, ember); }
        }
        let r2 = TOUCH_R.powi(2);
        let mut hit_idx = None;
        for f in self.fires.iter_mut().filter(|f| !f.gone) {
            let (fx, fy) = art::proj_px(f);
            let burnt = (0..2).find(|&k| self.active[k] && {
                let (wx, wy) = lerp_px(self.wizards[k].from, self.wizards[k].tile, self.wizards[k].lerp);
                (fx - wx).powi(2) + (fy - wy).powi(2) <= r2
            });
            if let Some(k) = burnt {
                hit_idx = hit_idx.or(Some(k));
                f.gone = true;
            }
        }
        self.fires.retain(|p| !p.gone);
        if let Some(k) = hit_idx {
            let lives = self.lives[k];
            self.hit_wizard(k);
            if self.lives[k] < lives { blip::bot::add("death_fire", 1.0); }
        }
    }

    pub(super) fn kill_enemy(&mut self, idx: usize) {
        let e = self.enemies[idx];
        if e.gone {
            return;
        }
        self.enemies[idx].gone = true;
        let pts = match e.kind {
            EnemyKind::Imp => SCORE_IMP,
            EnemyKind::Wraith => SCORE_WRAITH,
            EnemyKind::Spellhound => SCORE_HOUND,
            EnemyKind::Gargoyle => SCORE_GARGOYLE,
        };
        self.add_points(pts);
        let (x, y) = self.enemy_px(&e);
        let col = match e.kind {
            EnemyKind::Imp => IMP,
            EnemyKind::Wraith => WRAITH,
            EnemyKind::Spellhound => HOUND,
            EnemyKind::Gargoyle => FIREBALL,
        };
        self.magic.dissolve(x, y, col);
        let (sc, sr) = match e.kind {
            EnemyKind::Imp => (0, 2),
            EnemyKind::Wraith => (2, 2),
            EnemyKind::Spellhound => (4, 2),
            EnemyKind::Gargoyle => (0, 3),
        };
        self.magic.corpse(x, y, sc, sr, e.face_left, 0.8, col);
        self.fx.popup(x, y - 22.0, &format!("+{pts}"), BLIP_WHITE);
        if rand_int(0, 99) < POTION_CHANCE {
            let roll = rand_int(0, 9);
            self.drop_potion(e.tile, if roll == 0 { PotionKind::Life } else if roll <= 4 { PotionKind::Haste } else { PotionKind::Gold });
        }
        blip::bot::add("kill", 1.0);
        self.sfx(Sfx::Kill);
        self.maybe_whelp();
        self.check_exit();
    }
}

/// A projectile across its cell goes on into the next one (true), or, at a wall, is spent in a splash of `color`.
fn cross_cell(p: &mut Projectile, walls: &Walls, magic: &mut Magic, color: BlipColor) -> bool {
    p.from = p.tile;
    let (dc, dr) = p.dir.delta();
    let next = Cell { c: p.tile.c + dc, r: p.tile.r + dr };
    if walls.get(next.c, next.r) || !Walls::in_bounds(next.c, next.r) {
        p.gone = true;
        let (x, y) = center(p.tile);
        magic.splash(x + dc as f32 * 14.0, y + dr as f32 * 14.0, color);
        return false;
    }
    p.tile = next;
    p.lerp = 0.0;
    true
}

pub(super) fn enemy_step(e: &Enemy) -> f32 {
    match e.kind {
        EnemyKind::Imp => IMP_STEP,
        EnemyKind::Wraith => {
            if e.lunging {
                WRAITH_LUNGE_STEP
            } else {
                WRAITH_STEP
            }
        }
        EnemyKind::Spellhound => {
            if e.hunting {
                HOUND_HUNT_STEP
            } else {
                HOUND_WANDER_STEP
            }
        }
        EnemyKind::Gargoyle => f32::INFINITY,
    }
}

pub(super) fn pick_open_dir(walls: &Walls, tile: Cell) -> Dir {
    for d in DIRS {
        let (dc, dr) = d.delta();
        if walls.floor(tile.c + dc, tile.r + dr) {
            return d;
        }
    }
    Dir::Down
}

pub(super) fn is_taken(blocked: &[Cell], c: Cell) -> bool {
    blocked.contains(&c)
}

/// The first step (the cell and the way) of a shortest walk from `from` to `to` over the floor, around `blocked`;
/// none when `to` itself is a wall or taken.
pub(super) fn bfs_next(walls: &Walls, blocked: &[Cell], from: Cell, to: Cell) -> Option<(Cell, Dir)> {
    let open = |c: i32, r: i32| walls.floor(c, r) && !is_taken(blocked, Cell { c, r });
    if !open(to.c, to.r) { return None; }
    use blip::grid::Step;
    let (step, _) = blip::grid::path((from.c, from.r), (to.c, to.r), open, |_| true)?;
    let d = match step { Step::Up => Dir::Up, Step::Down => Dir::Down, Step::Left => Dir::Left, Step::Right => Dir::Right };
    let (dc, dr) = d.delta();
    Some((Cell { c: from.c + dc, r: from.r + dr }, d))
}

/// Pick the next step for a patrolling enemy: prefer straight, else a random
/// open corridor not already occupied.
pub(super) fn decide_patrol(walls: &Walls, blocked: &[Cell], e: &Enemy) -> Option<(Cell, Dir)> {
    let mut candidates: Vec<(Cell, Dir)> = Vec::new();
    for d in DIRS {
        let (dc, dr) = d.delta();
        let n = Cell { c: e.tile.c + dc, r: e.tile.r + dr };
        if walls.floor(n.c, n.r) && !is_taken(blocked, n) {
            candidates.push((n, d));
        }
    }
    if let Some(s) = candidates.iter().find(|(.., d)| *d == e.dir) {
        return Some(*s);
    }
    blip::rand_item(&candidates)
}
