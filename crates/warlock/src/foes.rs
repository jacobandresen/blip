//! The Warlock in his sanctum, moving between his spell circles behind his wards, and the dragon whelp that may follow
//! a chamber's last monster.

use super::*;

impl Game {
    pub(super) fn update_warlock(&mut self, dt: f32) {
        let Some(wl) = &mut self.warlock else { return };
        if wl.wards <= 0 { return; }
        wl.timer -= dt;
        if wl.timer > 0.0 { return; }
        match wl.state {
            WState::Materializing => {
                wl.state = WState::Aiming;
                wl.timer = 0.95;
                wl.aim_dir = pick_open_dir(&self.walls, wl.circles[wl.circle]);
                wl.aim_from = wl.circles[wl.circle];
            }
            WState::Aiming => {
                wl.state = WState::Casting;
                wl.timer = 0.12;
            }
            WState::Casting => {
                let (from, dir) = (wl.aim_from, wl.aim_dir);
                wl.state = WState::Pausing;
                wl.timer = 1.1;
                self.fires.push(Projectile { proj: Proj::Warlock, tile: from, from, lerp: 0.0, dir, gone: false });
                self.sfx(Sfx::Warlock);
            }
            WState::Pausing => self.warlock_advance(),
        }
    }

    pub(super) fn warlock_advance(&mut self) {
        let Some(wl) = &mut self.warlock else { return };
        for _ in 0..wl.circles.len() {
            wl.circle = (wl.circle + 1) % wl.circles.len();
            if !wl.sealed[wl.circle] {
                break;
            }
        }
        wl.state = WState::Materializing;
        wl.timer = 0.7;
        let (x, y) = center(wl.circles[wl.circle]);
        self.magic.summon(x, y, WARLOCK_C);
        const LINES: [&str; 7] = [
            "You will not leave.",
            "Burn, little wizard.",
            "I am NOT a vampire.",
            "My tower, my rules.",
            "Is that your best spell?",
            "Mind your robes.",
            "Again!",
        ];
        self.taunt = Some((LINES[rand_int(0, LINES.len() as i32 - 1) as usize].to_string(), 1.8));
    }

    pub(super) fn hit_warlock(&mut self) {
        let Some(wl) = &mut self.warlock else { return };
        if wl.wards <= 0 {
            return;
        }
        wl.wards -= 1;
        wl.sealed[wl.circle] = true;
        let tile = wl.circles[wl.circle];
        let wards_left = wl.wards;
        // `wl` is no longer used past here, so the borrow ends and the rest
        // of the method is free to touch other parts of `self`.
        self.add_points(SCORE_WARD);
        let (x, y) = center(tile);
        self.magic.shatter(x, y, WARLOCK_C);
        self.fx.popup(x, y - 8.0, &format!("+{SCORE_WARD}"), WARLOCK_C);
        blip::bot::add("ward", 1.0);
        self.sfx(Sfx::Ward);
        if wards_left > 0 {
            self.warlock_advance();
        } else {
            self.check_exit();
        }
    }

    /// The last monster down: a whelp may burst in for its few seconds.
    pub(super) fn maybe_whelp(&mut self) {
        if self.whelp_done || self.warlock.is_some() || self.sess.level < 2 || self.enemies.iter().any(|e| !e.gone) {
            return;
        }
        self.whelp_done = true;
        let far = (0..40)
            .filter_map(|_| self.random_open_cell())
            .map(|(c, r)| Cell { c, r })
            .max_by_key(|cl| (cl.c - self.wizards[0].tile.c).abs() + (cl.r - self.wizards[0].tile.r).abs());
        let Some(tile) = far else { return };
        self.whelp = Some(Whelp { tile, from: tile, lerp: 1.0, step_t: 0.4, dir: Dir::Left, left: WHELP_SECS, face_left: true });
        let (x, y) = center(tile);
        self.magic.summon(x, y, FIREBALL);
        self.magic.burst(x, y, 20, 140.0, FIREBALL, magic::Kind::Spark);
        self.banner = Some(("A DRAGON WHELP!".to_string(), 1.8));
        self.sfx(Sfx::Whelp);
        blip::bot::add("whelp", 1.0);
    }

    pub(super) fn update_whelp(&mut self, dt: f32) {
        let Some(w) = &mut self.whelp else { return };
        w.left -= dt;
        w.lerp = (w.lerp + dt / WHELP_STEP).min(1.0);
        if w.left <= 0.0 {
            let (x, y) = center(w.tile);
            self.whelp = None;
            self.magic.burst(x, y, 16, 100.0, FIREBALL, magic::Kind::Mote);
            self.magic.wave(x, y, 30.0, 0.4, 2.0, FIREBALL, false);
            self.banner = Some(("IT FLEW AWAY".to_string(), 1.2));
            self.check_exit();
            return;
        }
        w.step_t -= dt;
        if w.step_t > 0.0 {
            return;
        }
        w.step_t = WHELP_STEP;
        // prefer straight on, never straight back unless cornered
        let back = match w.dir { Dir::Up => Dir::Down, Dir::Down => Dir::Up, Dir::Left => Dir::Right, Dir::Right => Dir::Left };
        let mut opts: Vec<Dir> = DIRS.iter().copied().filter(|&d| {
            let (dc, dr) = d.delta();
            d != back && self.walls.floor(w.tile.c + dc, w.tile.r + dr)
        }).collect();
        if opts.is_empty() {
            opts.push(back);
        }
        let d = if opts.contains(&w.dir) && rand_int(0, 3) > 0 { w.dir } else { blip::rand_item(&opts).unwrap_or(back) };
        let (dc, dr) = d.delta();
        w.from = w.tile;
        w.tile = Cell { c: w.tile.c + dc, r: w.tile.r + dr };
        w.lerp = 0.0;
        w.dir = d;
        match d {
            Dir::Left => w.face_left = true,
            Dir::Right => w.face_left = false,
            _ => {}
        }
        if rand_int(0, 4) == 0 {
            let (x, y) = center(w.from);
            self.magic.cinders(x, y, FIREBALL);
        }
    }

    pub(super) fn whelp_px(&self) -> Option<(f32, f32)> {
        let w = self.whelp.as_ref()?;
        let a = center(w.from);
        let b = center(w.tile);
        Some((lerp(a.0, b.0, w.lerp), lerp(a.1, b.1, w.lerp)))
    }

    pub(super) fn strike_whelp(&mut self) {
        let Some((x, y)) = self.whelp_px() else { return };
        self.whelp = None;
        self.add_points(SCORE_WHELP);
        self.double_next = true;
        self.magic.dissolve(x, y, FIREBALL);
        self.magic.corpse(x, y, 0, 4, false, 0.8, FIREBALL);
        self.magic.wave(x, y, 80.0, 0.7, 3.0, RUNE_CLEAN, true);
        self.fx.popup(x, y - 22.0, &format!("+{SCORE_WHELP}"), RUNE_CLEAN);
        self.banner = Some(("NEXT CHAMBER PAYS DOUBLE".to_string(), 2.0));
        self.sfx(Sfx::Chamber);
        blip::bot::add("whelp_hit", 1.0);
        self.check_exit();
    }
}
