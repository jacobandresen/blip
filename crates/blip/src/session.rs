//! Session — tracks score, lives, and level for a single play session.
//!
//! Create one in `Game::new()` via [`Session::new`], and call [`Session::reset`]
//! at the start of each game to zero the score.

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
pub struct Session {
    pub score: i32,
    pub lives: i32,
    pub level: i32,
}

impl Session {
    /// Create a new session with zero score, `lives` lives, and level 1.
    pub fn new(lives: i32) -> Self {
        Self { score: 0, lives, level: 1 }
    }

    /// Add `pts` to the score.
    pub fn add_score(&mut self, pts: i32) {
        self.score += pts;
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

    /// Reset for a fresh game: zero the score, and restore lives and level.
    pub fn reset(&mut self, lives: i32) {
        self.score = 0;
        self.level = 1;
        self.lives = lives;
    }
}
