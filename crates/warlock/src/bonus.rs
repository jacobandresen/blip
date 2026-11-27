//! The spell circle, the bonus chamber between floors: four stones light in a sequence of five, and the wizards repeat
//! it with reveal, standing by each stone in turn, before the circle fades (BONUS_TIME). A wrong stone ends it.

use super::*;

impl Game {
    /// Into the circle: the first cellar emptied, the wizards at their spawns, a fresh sequence of five stones.
    pub(super) fn start_bonus(&mut self) {
        self.walls = walls_for(&ROOMS[0]);
        self.enemies.clear();
        self.fires.clear();
        self.bolts.clear();
        self.warlock = None;
        self.sightings.clear();
        self.gates.clear();
        self.runes.clear();
        self.exit_open = false;
        self.torches = self.find_torches();
        self.room_stamp = self.room_stamp.wrapping_add(1);
        self.magic.clear();
        let stones: Vec<Cell> = [(3, 1), (11, 1), (3, 11), (11, 11)]
            .iter()
            .map(|&(c, r)| Cell { c, r })
            .collect();
        let len = 5;
        let mut seq = Vec::new();
        for _ in 0..len {
            seq.push(rand_int(0, 3) as usize);
        }
        self.place_wizard(0, self.player_spawn);
        if self.active[1] {
            self.place_wizard(1, self.second_spawn());
        }
        self.bonus = Some(Bonus {
            stones,
            seq,
            expect: 0,
            showing: true,
            show_i: 0,
            show_t: 0.0,
            left: BONUS_TIME,
        });
        self.banner = Some(("SPELL CIRCLE".to_string(), 2.0));
        self.taunt = None;
        blip::bot::set("wing", self.cleared_no_death as f64);
    }

    /// The circle shows its sequence a stone at a time, then waits for it back until BONUS_TIME runs out.
    pub(super) fn update_bonus(&mut self, dt: f32) {
        self.fx.update(dt);
        self.update_ambience(dt);
        // Either wizard may stand on the stones.
        for i in 0..2 {
            if i == 1 && !self.active[1] { continue; }
            self.update_wizard(i, dt);
            self.wizards[i].reveal_cd = (self.wizards[i].reveal_cd - dt).max(0.0);
            if self.wizards[i].reveal_t > 0.0 {
                self.wizards[i].reveal_t = (self.wizards[i].reveal_t - dt).max(0.0);
            }
        }
        if let Some((_, t)) = &mut self.banner {
            *t -= dt;
        }

        // The showing phase just advances the reveal clock.
        let repeat_state = {
            let Some(b) = &mut self.bonus else { return };
            if b.showing {
                b.show_t += dt;
                if b.show_t > 0.7 {
                    b.show_t = 0.0;
                    b.show_i += 1;
                    if b.show_i >= b.seq.len() {
                        b.showing = false;
                        b.expect = 0;
                    }
                }
                return;
            }
            b.left -= dt;
            (b.stones.clone(), b.seq.clone(), b.expect, b.left)
        };
        let (stones, seq, expect, left) = repeat_state;
        if left <= 0.0 {
            self.banner = Some(("THE CIRCLE FADES".to_string(), 1.6));
            self.leave_bonus();
            return;
        }

        let Some(who) = (0..2).find(|&i| (i == 0 || self.active[1]) && self.reveal_pressed(i) && self.wizards[i].reveal_cd <= 0.0) else {
            return;
        };
        self.touch_stone(who, &stones, &seq, expect);
    }

    /// Wizard `who` casts reveal in the circle: the next stone of the sequence scores (more for each one further in)
    /// and the last one finishes it; anywhere else ends it.
    fn touch_stone(&mut self, who: usize, stones: &[Cell], seq: &[usize], expect: usize) {
        self.wizards[who].reveal_cd = REVEAL_CD;
        self.wizards[who].reveal_t = 0.45;
        let (x, y) = self.wizard_px(who);
        self.fx.ring(x, y, CELL as f32 * 2.0, 0.4, REVEAL_C);
        let hit = stones.iter().position(|s| distance_lt(self.wizards[who].tile, *s, 1.5));
        match hit {
            Some(k) if k == seq[expect] => {
                let pts = 100 * (expect as i32 + 1);
                self.add_points(pts);
                let (sx, sy) = center(stones[k]);
                self.fx.ring(sx, sy, CELL as f32 * 1.3, 0.4, RUNE_CLEAN);
                self.fx.popup(sx, sy - 6.0, &format!("+{pts}"), RUNE_CLEAN);
                if let Some(b) = &mut self.bonus {
                    b.expect = expect + 1;
                }
                if expect + 1 >= seq.len() {
                    self.leave_bonus();
                    return;
                }
                self.sfx(Sfx::Cleanse);
            }
            _ => {
                self.sfx(Sfx::Death);
                self.leave_bonus();
            }
        }
    }

    pub(super) fn leave_bonus(&mut self) {
        self.bonus = None;
        // Back to play, or the next chamber is drawn but never updated.
        self.state = State::Play;
        self.sess.next_level();
        self.load_room();
    }
}
