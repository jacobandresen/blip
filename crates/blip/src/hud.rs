//! The HUD across the top of every game (coin, score, best, hearts) and the shared end screens.

use macroquad::color::Color;

use crate::color::*;
use crate::ctx::Blip;

/// The HUD bar's height, and its text's top: low enough in the bar that the curved-glass shader's barrel doesn't
/// bow the top row off the top edge near the corners.
const HUD_BAR: f32 = 28.0;
const HUD_TEXT_Y: f32 = 7.0;
/// The HUD: the score's place right of its coin, and the hearts' spacing and most shown.
const HUD_SCORE_DX: f32 = 24.0;
const HUD_HEART_STEP: f32 = 18.0;
const HUD_HEARTS: i32 = 5;
const HUD_SLOTS: i32 = 3;
/// The digits `n` takes (anything below 1 counts as one).
fn digits(n: i32) -> usize { n.max(0).checked_ilog10().map_or(1, |d| d as usize + 1) }
/// The width of the best-score line "HI <n>" at size 2.
fn hi_width(hi: i32) -> f32 { (3 + digits(hi)) as f32 * 12.0 }

impl Blip {
    /// Draw SCORE, the best score and LIVES across the top.
    /// The high-score field is clipped between the left and right fields.
    /// Where the HUD's SCORE starts: the shell clips 3 screen pixels, which at half scale is 6 game pixels.
    fn hud_score_x(&self) -> f32 {
        (self.width as f32 * 0.012).max(4.0).round()
    }

    /// The right edge of the HUD's SCORE cluster: the label and one digit cell per digit.
    fn hud_score_right(&self, score: i32) -> f32 {
        self.hud_score_x() + HUD_SCORE_DX + digits(score) as f32 * 12.0
    }

    /// A score multiple (a streak's xN), just after the HUD's score, while there is one (`times` above 1). Call once a
    /// frame. A rise pops: bigger and flashing for MULTIPLE_POP seconds. A broken streak shows the lost multiple in red
    /// for MULTIPLE_LOST, so a player sees the rule that broke it as it happens.
    pub fn draw_hud_multiple(&self, score: i32, times: i32) {
        const MULTIPLE_POP: f32 = 0.6;
        const MULTIPLE_LOST: f32 = 0.5;
        let (shown, before, since) = self.multiple.get();
        let (before, since) = if times != shown { (shown, 0.0) } else { (before, since + self.delta_time) };
        self.multiple.set((times, before, since));
        let x = self.hud_score_right(score) + 12.0;
        if times > before && since < MULTIPLE_POP {
            // Size 3 sits 3 px higher to stay centred in the 28 px bar; white and yellow alternate every 0.1 s.
            let color = if (since * 10.0) as i32 % 2 == 0 { BLIP_WHITE } else { BLIP_YELLOW };
            self.draw_text(&format!("X{times}"), x, 4.0, 3.0, color);
        } else if times > 1 {
            self.draw_text(&format!("X{times}"), x, 7.0, 2.0, BLIP_YELLOW);
        } else if before > 1 && since < MULTIPLE_LOST {
            self.draw_text(&format!("X{before}"), x, 7.0, 2.0, BLIP_RED);
        }
    }

    /// The HUD's bar, `h` tall: ink with a cream rim a pixel above its foot.
    pub fn draw_hud_bar(&self, h: f32) {
        let w = self.width as f32;
        self.fill_rect(0.0, 0.0, w, h, BLIP_INK);
        self.fill_rect(0.0, h - 3.0, w, 2.0, BLIP_CREAM);
    }

    /// The HUD's bar on a title screen, the best score in its middle, as an arcade's attract mode shows it.
    pub fn draw_attract_bar(&self) { self.draw_attract_bar_with(crate::web::high_score()); }

    fn draw_attract_bar_with(&self, hi: i32) {
        self.draw_hud_bar(HUD_BAR);
        if hi <= 0 { return; }
        self.draw_hi(((self.width as f32 - hi_width(hi)) / 2.0).round(), HUD_TEXT_Y, hi);
    }

    /// The best score in the HUD's bar: HI in gold, the number in cream.
    fn draw_hi(&self, x: f32, y: f32, hi: i32) {
        self.draw_text("HI", x, y, 2.0, BLIP_GOLD);
        self.draw_text(&hi.to_string(), x + 36.0, y, 2.0, BLIP_CREAM);
    }

    /// The shared HUD, the same in every game: an ink bar with a cream rim, the coin and score left, the best in the
    /// middle and a heart a life right (empty hearts for lives lost below three, a count past five).
    pub fn draw_hud(&self, score: i32, lives: i32) {
        let w = self.width as f32;
        let ty = HUD_TEXT_Y;
        self.draw_hud_bar(HUD_BAR);

        // ---- the coin and the score, left ----
        let x0 = self.hud_score_x();
        self.draw_coin(x0, 4.0, 2.0);
        self.draw_number(score, x0 + HUD_SCORE_DX, ty, 2.0, BLIP_CREAM);
        let score_right = self.hud_score_right(score);

        // ---- hearts, right ----
        // Inset from the right edge to clear the curved glass's black rim on every canvas width.
        let right = w - (w * 0.085).round();
        // Three heart slots at least, so a life lost leaves an empty heart, as in the adventure games.
        let shown = lives.clamp(HUD_SLOTS, HUD_HEARTS);
        let count = if lives > HUD_HEARTS { format!("X{lives}") } else { String::new() };
        let count_w = crate::font::text_width(&count, 2.0);
        let hearts_left = right - count_w - shown as f32 * HUD_HEART_STEP;
        for i in 0..shown {
            self.draw_heart(hearts_left + i as f32 * HUD_HEART_STEP, 3.0, 2.0, i < lives);
        }
        self.draw_text(&count, right - count_w, ty, 2.0, BLIP_CREAM);

        // ---- HI <score>, centred in the gap ----
        let hi = crate::web::high_score();
        if hi <= 0 {
            return;
        }
        let lo = score_right + 16.0;
        let avail = hearts_left - 16.0 - lo;
        let head_w = hi_width(hi);
        // No clear room (a tiny canvas or a very long live score): no line.
        if head_w > avail {
            return;
        }
        self.draw_hi(lo + (avail - head_w) / 2.0, ty, hi);
    }

    /// The title screen's start prompt, the same in every game: PRESS START (a keyboard's Space and the deck's red
    /// button are labelled START; "PRESS START TO START" said it twice) or TAP TO START, white at size 3, outlined so
    /// it reads over artwork too.
    pub fn draw_start_prompt(&self, y: f32) {
        let prompt = crate::web::controls().pick("PRESS START", "PRESS START", "TAP TO START");
        self.draw_centered_outlined(prompt, self.width as f32 / 2.0, y, 3.0, BLIP_WHITE, BLIP_BLACK);
    }

    /// Draw the shared game-over screen over the game's chosen backdrop.
    /// `ready` shows the prompt to play again, after [`crate::GAME_OVER_MIN_WAIT`].
    pub fn draw_game_over(&self, score: i32, hi: i32, headline: Color, best: Color, prompt: Color, ready: bool) {
        self.draw_end_screen("GAME OVER", score, hi, headline, best, prompt, ready);
    }

    /// The screen a game ends on, won or lost: `title` (GAME OVER, YOU WON),
    /// the score, the best, and the prompt to play again once `ready`.
    #[allow(clippy::too_many_arguments)]
    pub fn draw_end_screen(&self, title: &str, score: i32, hi: i32, headline: Color, best: Color, prompt: Color, ready: bool) {
        let h = self.height as f32;
        self.draw_attract_bar();
        self.draw_end_panel();
        self.draw_logo(title, self.width as f32 / 2.0, h / 4.0, 5.0, headline);
        self.draw_centered(&format!("SCORE {score}"), h / 2.0, 3.0, BLIP_WHITE);
        self.draw_best(score, hi, h / 2.0 + 28.0, best);
        if ready {
            let again = crate::web::controls().pick("PRESS START", "PRESS START", "TAP TO PLAY");
            self.draw_centered(again, Self::game_over_prompt_y(h), 3.0, prompt);
        }
    }

    /// A game's own line on its game-over screen (the round reached, the
    /// length grown): between the best score and the prompt to play again.
    pub fn draw_game_over_note(&self, text: &str, color: Color) {
        let h = self.height as f32;
        let best_end = h / 2.0 + 28.0 + 16.0;
        self.draw_centered(text, (best_end + Self::game_over_prompt_y(h)) / 2.0 - 8.0, 2.0, color);
    }

    /// A line in place of the game-over screen's prompt (a continue's count, say), drawn with `ready` false.
    fn draw_game_over_prompt(&self, text: &str, color: Color) {
        let h = self.height as f32;
        self.draw_centered(text, Self::game_over_prompt_y(h), 3.0, color);
    }

    /// The continue offer's count, while it shows.
    pub fn draw_continue(&self, cont: &crate::Continue, color: Color) {
        if cont.showing() { self.draw_game_over_prompt(&format!("CONTINUE? {}", cont.seconds()), color); }
    }

    /// The end screen's message box, behind the title, the score and the prompt.
    fn draw_end_panel(&self) {
        let (w, h) = (self.width as f32, self.height as f32);
        let top = h / 4.0 - 26.0;
        let bottom = Self::game_over_prompt_y(h) + 21.0 + 22.0;
        let margin = (w * 0.08).round().max(12.0);
        self.draw_panel(margin, top, w - margin * 2.0, bottom - top, 1.0);
    }

    /// Two thirds down, or lower on a short screen, to leave the note its line.
    fn game_over_prompt_y(h: f32) -> f32 {
        (h * 2.0 / 3.0).max(h / 2.0 + 80.0)
    }

    /// Centred end-of-game line: `NEW BEST!` in `new_color` when this run beat
    /// (or tied) the best, otherwise `BEST 12345` in grey; nothing while no
    /// best is known.
    fn draw_best(&self, score: i32, hi: i32, y: f32, new_color: Color) {
        if hi > 0 && score >= hi {
            // It flashes (its colour and white, four times a second): the moment a player most wants to see.
            let lit = crate::blink(4.0);
            self.draw_centered("NEW BEST!", y, 2.0, if lit { new_color } else { BLIP_WHITE });
        } else if hi > 0 {
            self.draw_centered(&format!("BEST {hi}"), y, 2.0, blend(BLIP_CREAM, BLIP_INK, 0.35));
        }
    }
}
