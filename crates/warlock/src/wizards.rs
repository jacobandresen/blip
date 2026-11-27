//! The wizards: reading each one's stick and buttons, a step along the corridors (through a rune gate), reveal and the
//! bolt (a lance straight after reveal).

use super::*;

impl Game {
    /// Each wizard in play casts what the buttons ask: a bolt (held, again as it recharges) and reveal.
    pub(super) fn cast_spells(&mut self) {
        for i in 0..2 {
            if !self.active[i] || self.in_intro() { continue; }
            if self.bolt_pressed(i) || (self.bolt_held(i) && self.wizards[i].bolt_cd <= 0.0) { self.cast_bolt(i); }
            if self.reveal_pressed(i) { self.reveal(i); }
        }
    }

    pub(super) fn held_dir(&self, idx: usize) -> Option<Dir> {
        // Player one uses WASD; alone, the arrows are theirs too. Player two
        // uses the arrows (the deck's second station).
        let (w_up, w_down, w_left, w_right) = if idx == 0 {
            let arrows = !self.two_player;
            (
                key_held(BLIP_KEY_W) || (arrows && key_held(BLIP_KEY_UP)),
                key_held(BLIP_KEY_S) || (arrows && key_held(BLIP_KEY_DOWN)),
                key_held(BLIP_KEY_A) || (arrows && key_held(BLIP_KEY_LEFT)),
                key_held(BLIP_KEY_D) || (arrows && key_held(BLIP_KEY_RIGHT)),
            )
        } else {
            (
                key_held(BLIP_KEY_UP),
                key_held(BLIP_KEY_DOWN),
                key_held(BLIP_KEY_LEFT),
                key_held(BLIP_KEY_RIGHT),
            )
        };
        if w_up {
            Some(Dir::Up)
        } else if w_down {
            Some(Dir::Down)
        } else if w_left {
            Some(Dir::Left)
        } else if w_right {
            Some(Dir::Right)
        } else {
            None
        }
    }

    pub(super) fn bolt_pressed(&self, idx: usize) -> bool {
        if idx == 0 {
            key_pressed(BLIP_KEY_F) || key_pressed(BLIP_KEY_SPACE)
        } else {
            key_pressed(BLIP_KEY_J)
        }
    }
    pub(super) fn bolt_held(&self, idx: usize) -> bool {
        if idx == 0 {
            key_held(BLIP_KEY_F) || key_held(BLIP_KEY_SPACE)
        } else {
            key_held(BLIP_KEY_J)
        }
    }
    pub(super) fn reveal_pressed(&self, idx: usize) -> bool {
        if idx == 0 {
            key_pressed(BLIP_KEY_G) || key_pressed(BLIP_KEY_BUTTON2)
        } else {
            key_pressed(BLIP_KEY_K)
        }
    }

    // ---- play update ------------------------------------------------------

    /// While the title card holds the screen the cellar waits.
    pub(super) fn in_intro(&self) -> bool {
        self.intro > INTRO_SECS - INTRO_HOLD
    }

    pub(super) fn update_play(&mut self, dt: f32) {
        self.fx.update(dt);
        self.update_ambience(dt);
        if self.intro > 0.0 {
            self.intro = (self.intro - dt).max(0.0);
            if self.in_intro() {
                return;
            }
        }
        for i in 0..2 {
            if self.active[i] {
                self.update_wizard(i, dt);
            }
        }
        self.update_potions(dt);
        self.update_enemies(dt);
        self.update_whelp(dt);
        self.update_pulse(dt);
        self.check_contact();
        self.update_projectiles(dt);
        self.update_warlock(dt);
        if self.warlock.as_ref().is_some_and(|w| w.wards <= 0) {
            self.warlock = None;
        }
        for s in &mut self.sightings {
            s.t += dt;
        }
        self.sightings.retain(|s| s.t < SIGHTING_FADE);
        for g in &mut self.gates {
            g.cooldown = (g.cooldown - dt).max(0.0);
        }
        if let Some((_, t)) = &mut self.taunt {
            *t -= dt;
        }
        if let Some((_, t)) = &mut self.banner {
            *t -= dt;
        }
        for i in 0..2 {
            let w = &mut self.wizards[i];
            w.protect = (w.protect - dt).max(0.0);
            w.bolt_cd = (w.bolt_cd - dt).max(0.0);
            w.reveal_cd = (w.reveal_cd - dt).max(0.0);
            w.cast_t = (w.cast_t - dt).max(0.0);
            if w.reveal_t > 0.0 {
                w.reveal_t = (w.reveal_t - dt).max(0.0);
            }
        }
    }

    /// Wizard `idx` along the corridors: a step in progress goes on, and at its end the open exit finishes the chamber
    /// and a rune gate warps; otherwise the stick starts the next step, where the floor is free.
    pub(super) fn update_wizard(&mut self, idx: usize, dt: f32) {
        let held = self.held_dir(idx);

        let (cur, moving, step_t, lerp, facing, from) = {
            let w = &self.wizards[idx];
            (w.tile, w.moving, w.step_t, w.lerp, w.facing, w.from)
        };

        let mut reach_exit = false;
        let mut reach_gate = None;
        let step_secs = if self.wizards[idx].haste > 0.0 { MOVE_SECS * HASTE_MOVE } else { MOVE_SECS };

        let (tile, from, step_t, lerp, moving, facing) = if moving {
            let ns = step_t - dt;
            if ns <= 0.0 {
                if self.exit_open && cur == self.exit {
                    reach_exit = true;
                } else if let Some(gi) = self.gate_index(cur) {
                    reach_gate = Some(gi);
                }
                (cur, cur, 0.0, 1.0, false, facing)
            } else {
                (cur, from, ns, (lerp + dt / step_secs).min(1.0), true, facing)
            }
        } else if let Some(dir) = held {
            let (dc, dr) = dir.delta();
            let n = Cell { c: cur.c + dc, r: cur.r + dr };
            if self.walls.floor(n.c, n.r) && !self.step_blocked(n, idx) {
                (n, cur, step_secs, 0.0, true, dir)
            } else {
                (cur, cur, 0.0, 1.0, false, dir)
            }
        } else {
            (cur, cur, 0.0, 1.0, false, facing)
        };

        let w = &mut self.wizards[idx];
        w.tile = tile;
        w.from = from;
        w.step_t = step_t;
        w.lerp = lerp;
        w.moving = moving;
        w.facing = facing;

        if reach_exit {
            self.finish_chamber();
        } else if let Some(gi) = reach_gate {
            self.warp_through_gate(idx, gi);
        }
    }

    pub(super) fn warp_through_gate(&mut self, idx: usize, gi: usize) {
        let gate = self.gates[gi];
        let dest = if self.wizards[idx].tile == gate.a { gate.b } else { gate.a };
        if self.enemy_occupied(dest)
            || self.wizard_occupies(dest, idx)
            || self.warlock.as_ref().is_some_and(|wl| wl.wards > 0 && wl.circles[wl.circle] == dest)
        {
            return;
        }
        let (x, y) = self.wizard_px(idx);
        self.magic.warp(x, y, GATE_C);
        self.gates[gi].cooldown = GATE_CD;
        self.place_wizard(idx, dest);
        let (x, y) = center(dest);
        self.magic.warp(x, y, GATE_C);
        blip::bot::add("gate", 1.0);
    }

    pub(super) fn reveal(&mut self, idx: usize) {
        if self.wizards[idx].reveal_cd > 0.0 {
            return;
        }
        self.wizards[idx].reveal_cd = REVEAL_CD;
        self.wizards[idx].reveal_t = 0.55;
        self.wizards[idx].combo_t = LANCE_WINDOW;
        blip::bot::add("reveal", 1.0);
        let (x, y) = self.wizard_px(idx);
        self.magic.reveal(x, y, REVEAL_C);
        // expose wraiths, leave a last-seen outline
        let mut newly = Vec::new();
        for e in &mut self.enemies {
            if e.kind == EnemyKind::Wraith && !e.gone {
                e.revealed = REVEAL_SECS;
                newly.push(e.tile);
            }
        }
        for tile in newly {
            self.sightings.push(Sighting { tile, t: 0.0 });
        }
        // spellhounds race to where the spell was cast
        let cast_point = self.wizards[idx].tile;
        for e in &mut self.enemies {
            if e.kind == EnemyKind::Spellhound && !e.gone {
                e.hunting = true;
                e.target = cast_point;
            }
        }
        // cleanse a stone beside this wizard
        let mut cleansed = false;
        for i in 0..self.runes.len() {
            let r = self.runes[i].tile;
            if !self.runes[i].cleansed && distance_lt(self.wizards[idx].tile, r, 1.5) {
                self.runes[i].cleansed = true;
                cleansed = true;
                self.add_points(SCORE_RUNE);
                let (rx, ry) = center(r);
                self.magic.cleanse(rx, ry, RUNE_CLEAN);
                self.fx.popup(rx, ry - 22.0, &format!("+{SCORE_RUNE}"), RUNE_CLEAN);
                blip::bot::add("cleanse", 1.0);
            }
        }
        if cleansed {
            self.sfx(Sfx::Cleanse);
        } else {
            self.sfx(Sfx::Reveal);
        }
        self.check_exit();
    }

    pub(super) fn cast_bolt(&mut self, idx: usize) {
        let live = self.bolts.iter().filter(|b| !b.gone).count();
        if live >= BOLT_MAX {
            return;
        }
        self.wizards[idx].bolt_cd = BOLT_RATE;
        self.wizards[idx].cast_t = 0.16;
        let (x, y) = self.wizard_px(idx);
        let (tx, ty) = art::staff_tip(&self.wizards[idx]);
        let (dc, dr) = self.wizards[idx].facing.delta();
        let col = if idx == 0 { REVEAL_C } else { WIZ_ROBE2 };
        self.magic.cast(x + tx, y + ty, dc as f32, dr as f32, col);
        let tile = self.wizards[idx].tile;
        // reveal, then bolt: the spells combine into a lance that pierces
        let lance = self.wizards[idx].combo_t > 0.0;
        self.wizards[idx].combo_t = 0.0;
        if lance {
            self.magic.splash(x + tx, y + ty, LANCE_C);
            self.magic.splash(x + tx, y + ty, col);
            self.magic.shake = self.magic.shake.max(0.25);
            if !self.lance_named {
                self.lance_named = true;
                self.banner = Some(("REVEAL + BOLT: ARCANE LANCE!".to_string(), 1.6));
            }
            blip::bot::add("lance", 1.0);
        }
        self.bolts.push(Projectile {
            proj: if lance { Proj::Lance } else { Proj::Bolt },
            tile,
            from: tile,
            lerp: 0.0,
            dir: self.wizards[idx].facing,
            gone: false,
        });
        blip::bot::add("bolt", 1.0);
        self.sfx(Sfx::Bolt);
    }
}
