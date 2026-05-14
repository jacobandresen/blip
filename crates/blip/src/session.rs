//! Session — tracks score, hi-score, lives, and level for a single play session.
//!
//! Create one in `Game::new()` via [`Session::new`], and call [`Session::reset`]
//! at the start of each game to reload the latest global hi-score and zero the score.

use crate::web;

/// Returned by [`Session::lose_life`] so callers can branch cleanly without
/// inspecting the lives count themselves.
#[derive(Copy, Clone, PartialEq, Eq)]
pub enum LifeResult {
    /// The player still has lives remaining — show the death animation and respawn.
    StillAlive,
    /// No lives left — transition to the game-over screen.
    GameOver,
}

/// Per-session score state shared by every game.
///
/// The hi-score is automatically saved to the web backend whenever `add_score`
/// pushes the score above the current hi — no manual save calls needed.
pub struct Session {
    pub score: i32,
    pub hi:    i32,
    pub lives: i32,
    pub level: i32,
}

impl Session {
    /// Create a new session with zero score, `lives` lives, and level 1.
    /// Loads the current global hi-score for `game_id` from the web backend.
    pub fn new(game_id: i32, lives: i32) -> Self {
        Self { score: 0, hi: web::load_hi_score(game_id), lives, level: 1 }
    }

    /// Add `pts` to the score. Automatically saves a new hi-score if beaten.
    pub fn add_score(&mut self, game_id: i32, pts: i32) {
        self.score += pts;
        if self.score > self.hi {
            self.hi = self.score;
            web::save_hi_score(game_id, self.hi);
        }
    }

    /// Decrement lives by one and report whether the game should end.
    pub fn lose_life(&mut self) -> LifeResult {
        self.lives -= 1;
        if self.lives <= 0 { LifeResult::GameOver } else { LifeResult::StillAlive }
    }

    /// Advance to the next level (increments the level counter only).
    pub fn next_level(&mut self) {
        self.level += 1;
    }

    /// Re-check the global hi-score. Call this on title and game-over screens so
    /// the display reflects scores set by other players during the session.
    pub fn refresh_hi(&mut self, game_id: i32) {
        self.hi = self.hi.max(web::load_hi_score(game_id));
    }

    /// Reset for a fresh game: zero the score, restore lives and level, and
    /// reload the global hi-score.
    pub fn reset(&mut self, game_id: i32, lives: i32) {
        self.score = 0;
        self.level = 1;
        self.lives = lives;
        self.hi = self.hi.max(web::load_hi_score(game_id));
    }
}
