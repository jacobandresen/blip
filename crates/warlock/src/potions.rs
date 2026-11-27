//! Potions: now and then one is left where a monster falls, seeping away if nobody comes, and what each does to the wizard who drinks it.

use super::*;

impl Game {
    pub(super) fn drop_potion(&mut self, tile: Cell, kind: PotionKind) {
        if self.potions.iter().any(|p| p.tile == tile) {
            return;
        }
        self.potions.push(Potion { tile, kind, t: 0.0 });
    }

    /// Potions age and seep away; a wizard standing on one drinks it.
    pub(super) fn update_potions(&mut self, dt: f32) {
        for w in &mut self.wizards {
            w.haste = (w.haste - dt).max(0.0);
            w.combo_t = (w.combo_t - dt).max(0.0);
        }
        for p in &mut self.potions {
            p.t += dt;
        }
        self.potions.retain(|p| p.t < POTION_SECS);
        let mut i = 0;
        while i < self.potions.len() {
            let p = self.potions[i];
            if let Some(w) = (0..2).find(|&w| self.active[w] && self.wizards[w].tile == p.tile) {
                self.potions.remove(i);
                self.drink(w, p);
            } else {
                i += 1;
            }
        }
    }

    fn drink(&mut self, w: usize, p: Potion) {
        let (x, y) = center(p.tile);
        let (text, col) = match p.kind {
            PotionKind::Haste => {
                self.wizards[w].haste = HASTE_SECS;
                ("HASTE!".to_string(), POTION_HASTE)
            }
            PotionKind::Gold => {
                self.add_points(SCORE_GOLD);
                (format!("+{}", if self.double { SCORE_GOLD * 2 } else { SCORE_GOLD }), POTION_GOLD)
            }
            PotionKind::Life => {
                self.lives[w] = (self.lives[w] + 1).min(LIVES_MAX);
                ("A SPARE HAT!".to_string(), POTION_LIFE)
            }
        };
        self.fx.popup(x, y - 22.0, &text, col);
        self.magic.splash(x, y, col);
        blip::bot::add("potion", 1.0);
        self.sfx(Sfx::Potion);
    }
}
